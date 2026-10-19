use std::{cell::RefCell, ptr::NonNull, sync::Arc};

use block2::{DynBlock, RcBlock};
use objc2::{
    define_class, msg_send, rc::Retained, runtime::ProtocolObject, AnyThread, DefinedClass,
};
use objc2_foundation::{
    ns_string, NSArray, NSDictionary, NSError, NSObject, NSObjectProtocol, NSSet, NSString,
};
use objc2_user_notifications::*;
use serde::Serialize;

const CATEGORY: &str = "timelet.reminder";
const COMPLETE: &str = "timelet.complete";
const SNOOZE: &str = "timelet.snooze";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotificationAction {
    Open,
    Complete,
    Snooze,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationResponse {
    pub occurrence_id: String,
    pub entry_id: String,
    pub revision: String,
    pub action: NotificationAction,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionStatus {
    pub authorization: String,
    pub alerts_enabled: bool,
    pub notification_center_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ReminderNotification {
    pub occurrence_id: String,
    pub entry_id: String,
    pub revision: String,
    pub title: String,
    pub body: String,
}

type ResponseHandler = Arc<dyn Fn(NotificationResponse) + Send + Sync>;

define_class!(
    // NSObject 无额外子类约束；回调只处理可跨线程的 Rust 值。
    #[unsafe(super = NSObject)]
    #[ivars = ResponseHandler]
    struct NotificationDelegate;

    unsafe impl NSObjectProtocol for NotificationDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for NotificationDelegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            completion
                .call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List,));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion: &DynBlock<dyn Fn()>,
        ) {
            // 不允许业务回调的 panic 跨越 Objective-C 边界；任何响应均须结束系统回调。
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if let Some(response) = decode_response(response) {
                    (self.ivars())(response);
                }
            }));
            completion.call(());
            if result.is_err() {
                eprintln!("通知响应处理失败");
            }
        }
    }
);

thread_local! {
    // 通知中心的 delegate 为弱引用，必须由应用持有到退出。
    static DELEGATE: RefCell<Option<Retained<NotificationDelegate>>> = const { RefCell::new(None) };
}

/// 启动时注册动作，不触发通知授权；须由主线程调用一次。
pub fn init(handler: impl Fn(NotificationResponse) + Send + Sync + 'static) -> Result<(), String> {
    if objc2_foundation::MainThreadMarker::new().is_none() {
        return Err("通知初始化必须在主线程执行".into());
    }
    DELEGATE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_some() {
            return Err("通知服务已初始化".into());
        }
        let this = NotificationDelegate::alloc().set_ivars(Arc::new(handler) as ResponseHandler);
        // NSObject 的 init 不改变本类已初始化的 Rust ivars。
        let delegate: Retained<NotificationDelegate> = unsafe { msg_send![super(this), init] };
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let complete = UNNotificationAction::actionWithIdentifier_title_options(
            &NSString::from_str(COMPLETE),
            ns_string!("完成"),
            UNNotificationActionOptions::empty(),
        );
        let snooze = UNNotificationAction::actionWithIdentifier_title_options(
            &NSString::from_str(SNOOZE),
            ns_string!("10分钟后提醒"),
            UNNotificationActionOptions::empty(),
        );
        let category =
            UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                &NSString::from_str(CATEGORY),
                &NSArray::from_retained_slice(&[complete, snooze]),
                &NSArray::<NSString>::new(),
                UNNotificationCategoryOptions::empty(),
            );
        center.setNotificationCategories(&NSSet::from_retained_slice(&[category]));
        center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        *slot = Some(delegate);
        Ok(())
    })
}

pub fn permission(callback: impl Fn(PermissionStatus) + Send + Sync + 'static) {
    let block = RcBlock::new(move |settings: NonNull<UNNotificationSettings>| {
        // 系统在回调期间持有 settings，此处不保存借用。
        let settings = unsafe { settings.as_ref() };
        let authorization = match settings.authorizationStatus() {
            UNAuthorizationStatus::NotDetermined => "notDetermined",
            UNAuthorizationStatus::Denied => "denied",
            UNAuthorizationStatus::Authorized => "authorized",
            UNAuthorizationStatus::Provisional => "provisional",
            _ => "unknown",
        };
        callback(PermissionStatus {
            authorization: authorization.into(),
            alerts_enabled: settings.alertSetting() == UNNotificationSetting::Enabled,
            notification_center_enabled: settings.notificationCenterSetting()
                == UNNotificationSetting::Enabled,
        });
    });
    UNUserNotificationCenter::currentNotificationCenter()
        .getNotificationSettingsWithCompletionHandler(&block);
}

/// 仅响应用户的授权操作调用，不在应用启动时调用。
pub fn request_permission(callback: impl Fn(Result<bool, String>) + Send + Sync + 'static) {
    let block = RcBlock::new(move |granted: objc2::runtime::Bool, error: *mut NSError| {
        callback(error_result(error).map(|()| granted.as_bool()));
    });
    UNUserNotificationCenter::currentNotificationCenter()
        .requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &block,
        );
}

/// 返回系统接受请求的结果，不将其视为用户已经看到通知。
pub fn send(
    notification: ReminderNotification,
    callback: impl Fn(Result<(), String>) + Send + Sync + 'static,
) -> Result<(), String> {
    validate(&notification)?;
    let callback = Arc::new(callback);
    permission(move |permission| {
        if !matches!(
            permission.authorization.as_str(),
            "authorized" | "provisional"
        ) {
            callback(Err("系统通知权限未开启".into()));
            return;
        }
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&notification.title));
        content.setBody(&NSString::from_str(&notification.body));
        content.setCategoryIdentifier(&NSString::from_str(CATEGORY));
        let keys = [ns_string!("entryId"), ns_string!("revision")];
        let values = [
            NSString::from_str(&notification.entry_id),
            NSString::from_str(&notification.revision),
        ];
        let info = NSDictionary::from_slices(&keys, &[&*values[0], &*values[1]]);
        // 字典键和值均为 NSString，可安全桥接为通知 userInfo。
        unsafe {
            content.setUserInfo(info.cast_unchecked());
        }
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&notification.occurrence_id),
            &content,
            None,
        );
        let callback = callback.clone();
        let block = RcBlock::new(move |error: *mut NSError| callback(error_result(error)));
        UNUserNotificationCenter::currentNotificationCenter()
            .addNotificationRequest_withCompletionHandler(&request, Some(&block));
    });
    Ok(())
}

pub fn remove(occurrence_ids: &[String]) {
    let ids: Vec<_> = occurrence_ids
        .iter()
        .map(|id| NSString::from_str(id))
        .collect();
    let ids = NSArray::from_retained_slice(&ids);
    let center = UNUserNotificationCenter::currentNotificationCenter();
    center.removePendingNotificationRequestsWithIdentifiers(&ids);
    center.removeDeliveredNotificationsWithIdentifiers(&ids);
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSnapshot {
    pub pending: Vec<String>,
    pub delivered: Vec<String>,
}

pub fn snapshot(callback: impl Fn(NotificationSnapshot) + Send + Sync + 'static) {
    let callback = Arc::new(callback);
    let block = RcBlock::new(move |requests: NonNull<NSArray<UNNotificationRequest>>| {
        let pending: Vec<_> = unsafe { requests.as_ref() }
            .iter()
            .map(|r| r.identifier().to_string())
            .collect();
        let callback = callback.clone();
        let block = RcBlock::new(move |notifications: NonNull<NSArray<UNNotification>>| {
            let delivered = unsafe { notifications.as_ref() }
                .iter()
                .map(|n| n.request().identifier().to_string())
                .collect();
            callback(NotificationSnapshot {
                pending: pending.clone(),
                delivered,
            });
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .getDeliveredNotificationsWithCompletionHandler(&block);
    });
    UNUserNotificationCenter::currentNotificationCenter()
        .getPendingNotificationRequestsWithCompletionHandler(&block);
}

fn validate(notification: &ReminderNotification) -> Result<(), String> {
    if [
        &notification.occurrence_id,
        &notification.entry_id,
        &notification.revision,
        &notification.title,
    ]
    .iter()
    .any(|s| s.trim().is_empty() || s.contains('\0'))
    {
        return Err("通知标识、条目、修订与标题不能为空或包含空字符".into());
    }
    Ok(())
}

fn error_result(error: *mut NSError) -> Result<(), String> {
    // NSError 指针只在系统回调期间借用，立即转换为 Rust 字符串。
    match unsafe { error.as_ref() } {
        Some(error) => Err(error.localizedDescription().to_string()),
        None => Ok(()),
    }
}

fn decode_response(response: &UNNotificationResponse) -> Option<NotificationResponse> {
    let request = response.notification().request();
    decode_request(&request, &response.actionIdentifier().to_string())
}

fn decode_request(
    request: &UNNotificationRequest,
    identifier: &str,
) -> Option<NotificationResponse> {
    let content = request.content();
    if content.categoryIdentifier().to_string() != CATEGORY {
        return None;
    }
    let action = match identifier {
        COMPLETE => NotificationAction::Complete,
        SNOOZE => NotificationAction::Snooze,
        value if value == unsafe { UNNotificationDefaultActionIdentifier }.to_string() => {
            NotificationAction::Open
        }
        _ => return None,
    };
    let info = content.userInfo();
    let value = |key| {
        info.objectForKey(&NSString::from_str(key))
            .and_then(|value| value.downcast::<NSString>().ok())
            .map(|value| value.to_string())
    };
    let entry_id = value("entryId")?;
    let revision = value("revision")?;
    let occurrence_id = request.identifier().to_string();
    if [entry_id.as_str(), revision.as_str(), occurrence_id.as_str()]
        .iter()
        .any(|s| s.trim().is_empty())
    {
        return None;
    }
    Some(NotificationResponse {
        occurrence_id,
        entry_id,
        revision,
        action,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2::runtime::AnyObject;

    fn request(
        category: &str,
        entry: &AnyObject,
        revision: &str,
    ) -> Retained<UNNotificationRequest> {
        let content = UNMutableNotificationContent::new();
        content.setCategoryIdentifier(&NSString::from_str(category));
        let revision = NSString::from_str(revision);
        let info = NSDictionary::from_slices(
            &[ns_string!("entryId"), ns_string!("revision")],
            &[entry, &*revision as &AnyObject],
        );
        // 测试覆盖来自系统的非字符串字段，确保解码拒绝错误类型。
        unsafe {
            content.setUserInfo(info.cast_unchecked());
        }
        UNNotificationRequest::requestWithIdentifier_content_trigger(
            ns_string!("occurrence-1"),
            &content,
            None,
        )
    }

    #[test]
    fn native_payload_keeps_occurrence_entry_and_revision_for_each_action() {
        let request = request(CATEGORY, ns_string!("entry-1"), "revision-1");
        let default = unsafe { UNNotificationDefaultActionIdentifier }.to_string();
        for (identifier, action) in [
            (COMPLETE, NotificationAction::Complete),
            (SNOOZE, NotificationAction::Snooze),
            (default.as_str(), NotificationAction::Open),
        ] {
            assert_eq!(
                decode_request(&request, identifier),
                Some(NotificationResponse {
                    occurrence_id: "occurrence-1".into(),
                    entry_id: "entry-1".into(),
                    revision: "revision-1".into(),
                    action,
                })
            );
        }
        assert!(decode_request(&request, "unrecognized-action").is_none());
        assert!(decode_request(
            &request,
            &unsafe { UNNotificationDismissActionIdentifier }.to_string()
        )
        .is_none());
    }

    #[test]
    fn foreign_or_malformed_native_payload_cannot_become_a_business_action() {
        let wrong_type = NSObject::new();
        for request in [
            request("foreign.category", ns_string!("entry-1"), "revision-1"),
            request(CATEGORY, &wrong_type, "revision-1"),
            request(CATEGORY, ns_string!(""), "revision-1"),
            request(CATEGORY, ns_string!("entry-1"), ""),
        ] {
            assert!(decode_request(&request, COMPLETE).is_none());
        }
    }
}
