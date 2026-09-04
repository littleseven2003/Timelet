use std::{fs, io::Write, path::PathBuf, sync::Mutex};

use serde::{de::DeserializeOwned, Serialize};

pub trait Document: Default + Serialize + DeserializeOwned {
    fn validate(&self) -> Result<(), String>;
}

// 同一文件的读取、校验、备份和替换必须在同一把锁内，磁盘始终是唯一事实来源。
pub struct DataFile<T> {
    path: PathBuf,
    lock: Mutex<()>,
    document: std::marker::PhantomData<T>,
}

impl<T: Document> DataFile<T> {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
            document: std::marker::PhantomData,
        }
    }

    fn load(&self) -> Result<(T, Option<Vec<u8>>), String> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok((T::default(), None));
            }
            Err(error) => return Err(self.error("读取失败，原文件未被覆盖", error)),
        };
        let value: T = serde_json::from_slice(&bytes).map_err(|error| {
            self.error("内容无法识别，已停止写入；请保留原文件并检查备份", error)
        })?;
        value
            .validate()
            .map_err(|error| self.error("校验失败，已停止写入", error))?;
        Ok((value, Some(bytes)))
    }

    pub fn read(&self) -> Result<T, String> {
        let _guard = self.lock.lock().map_err(|_| "数据正在恢复，请重启应用")?;
        self.load().map(|(value, _)| value)
    }

    pub fn change(&self, mutate: impl FnOnce(&mut T) -> Result<(), String>) -> Result<(), String> {
        let _guard = self.lock.lock().map_err(|_| "数据正在恢复，请重启应用")?;
        // 每次写入前重新校验，外部损坏或高版本文件同样不能被旧状态覆盖。
        let (mut value, previous) = self.load()?;
        mutate(&mut value)?;
        value.validate()?;
        let bytes = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| self.error("无法创建数据目录", e))?;
        }
        if let Some(previous) = previous {
            let backup = self.path.with_extension("json.bak");
            Self::replace(&backup, &previous)
                .map_err(|e| self.error("备份失败，本次保存已取消", e))?;
        }
        Self::replace(&self.path, &bytes).map_err(|e| self.error("保存失败，原文件保持不变", e))
    }

    fn replace(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
        let tmp = path.with_extension(format!(
            "{}.tmp",
            path.extension().unwrap_or_default().to_string_lossy()
        ));
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(tmp, path)
    }

    fn error(&self, message: &str, error: impl std::fmt::Display) -> String {
        format!("{message}（{}）：{error}", self.path.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };

    #[derive(Default, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Sample {
        count: usize,
    }
    impl Document for Sample {
        fn validate(&self) -> Result<(), String> {
            Ok(())
        }
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static ID: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "timelet-storage-test-{}-{}",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn file(&self) -> DataFile<Sample> {
            DataFile::new(self.0.join("data.json"))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn concurrent_writes_survive_reload_and_keep_backup() {
        let fixture = Fixture::new();
        let data = Arc::new(fixture.file());
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let data = data.clone();
                std::thread::spawn(move || {
                    for _ in 0..8 {
                        data.change(|v| {
                            v.count += 1;
                            Ok(())
                        })
                        .unwrap();
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(fixture.file().read().unwrap().count, 64);
        let backup: Sample =
            serde_json::from_slice(&fs::read(fixture.0.join("data.json.bak")).unwrap()).unwrap();
        assert_eq!(backup.count, 63);
    }

    #[test]
    fn corrupt_or_unknown_content_is_not_overwritten_and_can_be_retried() {
        let fixture = Fixture::new();
        let data = fixture.file();
        for raw in ["{broken", r#"{"count":2,"future":true}"#] {
            fs::write(&data.path, raw).unwrap();
            assert!(data.read().is_err());
            assert!(data
                .change(|v| {
                    v.count = 0;
                    Ok(())
                })
                .is_err());
            assert_eq!(fs::read_to_string(&data.path).unwrap(), raw);
        }
        fs::write(&data.path, r#"{"count":7}"#).unwrap();
        assert_eq!(data.read().unwrap().count, 7);
    }

    #[test]
    fn failed_backup_or_replacement_does_not_publish_mutation() {
        let fixture = Fixture::new();
        let data = fixture.file();
        data.change(|v| {
            v.count = 1;
            Ok(())
        })
        .unwrap();
        let blocked_backup = fixture.0.join("data.json.bak.tmp");
        fs::create_dir(&blocked_backup).unwrap();
        assert!(data
            .change(|v| {
                v.count = 2;
                Ok(())
            })
            .is_err());
        assert_eq!(data.read().unwrap().count, 1);
        fs::remove_dir(blocked_backup).unwrap();
        fs::create_dir(fixture.0.join("data.json.tmp")).unwrap();
        assert!(data
            .change(|v| {
                v.count = 3;
                Ok(())
            })
            .is_err());
        assert_eq!(fixture.file().read().unwrap().count, 1);
    }
}
