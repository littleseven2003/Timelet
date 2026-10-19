use std::collections::HashSet;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NotificationRule {
    pub revision: String,
    pub enabled_at: DateTime<Utc>,
}

impl NotificationRule {
    pub fn validate(&self) -> Result<(), String> {
        validate_identifier(&self.revision)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OccurrenceStatus {
    Pending,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeliveryStatus {
    Pending,
    Submitting,
    Submitted,
    Failed,
    NeedsConfirmation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Occurrence {
    pub id: String,
    pub entry_id: String,
    pub revision: String,
    pub planned_at: DateTime<Utc>,
    pub effective_at: DateTime<Utc>,
    pub status: OccurrenceStatus,
    pub delivery: DeliveryStatus,
    pub last_error: Option<String>,
}

impl Occurrence {
    pub fn new(
        entry_id: String,
        revision: String,
        planned_at: DateTime<Utc>,
    ) -> Result<Self, String> {
        validate_identifier(&entry_id)?;
        validate_identifier(&revision)?;
        Ok(Self {
            id: occurrence_id(&entry_id, &revision, planned_at),
            entry_id,
            revision,
            planned_at,
            effective_at: planned_at,
            status: OccurrenceStatus::Pending,
            delivery: DeliveryStatus::Pending,
            last_error: None,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_identifier(&self.entry_id)?;
        validate_identifier(&self.revision)?;
        if self.id != occurrence_id(&self.entry_id, &self.revision, self.planned_at) {
            return Err("提醒发生标识与条目、规则修订或计划时间不一致".into());
        }
        if self.effective_at < self.planned_at {
            return Err("提醒有效时间不能早于计划时间".into());
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReminderState {
    pub paused_until: Option<DateTime<Utc>>,
    pub occurrences: Vec<Occurrence>,
}

impl ReminderState {
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = HashSet::new();
        for occurrence in &self.occurrences {
            occurrence.validate()?;
            if !ids.insert(&occurrence.id) {
                return Err("提醒发生记录重复".into());
            }
        }
        Ok(())
    }
}

fn occurrence_id(entry_id: &str, revision: &str, planned_at: DateTime<Utc>) -> String {
    // 长度前缀避免标识中的分隔符碰撞；统一 UTC 保证同一时刻不因时区写法而重复。
    format!(
        "timelet:{}:{}:{}:{}:{}",
        entry_id.len(),
        entry_id,
        revision.len(),
        revision,
        planned_at.to_rfc3339_opts(SecondsFormat::AutoSi, true)
    )
}

fn validate_identifier(value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.contains('\0') {
        return Err("提醒条目或规则修订标识为空或无法识别".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value).unwrap().into()
    }

    #[test]
    fn occurrence_identity_survives_snooze_and_timezone_representation() {
        let utc = instant("2026-10-19T09:00:00Z");
        let local = instant("2026-10-19T17:00:00+08:00");
        let mut occurrence = Occurrence::new("entry:a".into(), "rev:1".into(), utc).unwrap();
        let equivalent = Occurrence::new("entry:a".into(), "rev:1".into(), local).unwrap();
        assert_eq!(occurrence.id, equivalent.id);
        occurrence.effective_at = utc + chrono::Duration::minutes(10);
        occurrence.status = OccurrenceStatus::Completed;
        occurrence.delivery = DeliveryStatus::Submitted;
        let persisted = serde_json::to_vec(&occurrence).unwrap();
        let restored: Occurrence = serde_json::from_slice(&persisted).unwrap();
        assert_eq!(restored, occurrence);
        assert_eq!(restored.id, equivalent.id);
        assert!(restored.validate().is_ok());
        assert_ne!(
            occurrence.id,
            Occurrence::new("entry:a".into(), "rev:2".into(), utc)
                .unwrap()
                .id
        );
        assert_ne!(
            occurrence_id("a:b", "c", utc),
            occurrence_id("a", "b:c", utc)
        );
    }

    #[test]
    fn damaged_or_duplicate_occurrences_are_rejected() {
        let mut occurrence =
            Occurrence::new("a".into(), "r1".into(), instant("2026-10-19T09:00:00Z")).unwrap();
        let mut state = ReminderState {
            occurrences: vec![occurrence.clone(), occurrence.clone()],
            ..Default::default()
        };
        assert!(state.validate().is_err());
        state.occurrences.pop();
        assert!(state.validate().is_ok());
        occurrence.revision = "r2".into();
        assert!(occurrence.validate().is_err());
        occurrence.revision = "r1".into();
        occurrence.effective_at = occurrence.planned_at - chrono::Duration::seconds(1);
        assert!(occurrence.validate().is_err());
        assert!(Occurrence::new(" ".into(), "r".into(), occurrence.planned_at).is_err());
        assert!(Occurrence::new("a".into(), "r\0".into(), occurrence.planned_at).is_err());
    }

    #[test]
    fn unknown_or_invalid_reminder_fields_are_not_silently_defaulted() {
        let rule: NotificationRule =
            serde_json::from_str(r#"{"revision":"r1","enabledAt":"2026-10-19T17:00:00+08:00"}"#)
                .unwrap();
        assert!(rule.validate().is_ok());
        assert_eq!(rule.enabled_at, instant("2026-10-19T09:00:00Z"));
        for json in [
            r#"{"revision":"r1","enabledAt":"broken"}"#,
            r#"{"revision":"r1"}"#,
            r#"{"revision":"r1","enabledAt":"2026-10-19T09:00:00Z","future":true}"#,
        ] {
            assert!(serde_json::from_str::<NotificationRule>(json).is_err());
        }
        assert!(NotificationRule {
            revision: "".into(),
            enabled_at: rule.enabled_at,
        }
        .validate()
        .is_err());
        assert!(serde_json::from_str::<DeliveryStatus>(r#""delivered""#).is_err());
        assert!(serde_json::from_str::<ReminderState>(r#"{"pausedUntil":null}"#).is_err());
        let state: ReminderState =
            serde_json::from_str(r#"{"pausedUntil":null,"occurrences":[]}"#).unwrap();
        assert_eq!(state, ReminderState::default());
    }
}
