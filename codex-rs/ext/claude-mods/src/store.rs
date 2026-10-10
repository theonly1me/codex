use crate::ModResult;
use crate::storage::Storage;
use codex_file_system::ExecutorFileSystem;
use codex_file_system::ReadFileOptions;
use codex_file_system::RemoveOptions;
use codex_utils_path_uri::PathUri;
use serde_json::Value;
use std::path::PathBuf;

impl Storage {
    pub(crate) async fn get(&self, plugin: &str, namespace: &str, key: &str) -> ModResult<Value> {
        let _transaction = self
            .transaction
            .acquire()
            .await
            .map_err(|_| "mod storage stopped".to_string())?;
        let path = self.key_path(plugin, namespace, key)?;
        let path = PathUri::from_host_native_path(&path).map_err(|error| error.to_string())?;
        let sandbox = None;
        match self
            .file_system
            .read_file_text(
                &path,
                ReadFileOptions {
                    follow_symlinks: false,
                },
                sandbox,
            )
            .await
        {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|error| format!("invalid mod state {path}: {error}")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Value::Null),
            Err(error) => Err(error.to_string()),
        }
    }

    pub(crate) async fn set(
        &self,
        plugin: &str,
        namespace: &str,
        key: &str,
        value: Value,
    ) -> ModResult<Value> {
        let _transaction = self
            .transaction
            .acquire()
            .await
            .map_err(|_| "mod storage stopped".to_string())?;
        let path = self.key_path(plugin, namespace, key)?;
        let contents = serde_json::to_vec(&value).map_err(|error| error.to_string())?;
        if contents.len() > 1024 * 1024 {
            return Err("mod state exceeds 1 MiB".into());
        }
        self.write(&path, contents).await?;
        Ok(Value::Null)
    }

    pub(crate) async fn delete(&self, plugin: &str, key: &str) -> ModResult<Value> {
        let _transaction = self
            .transaction
            .acquire()
            .await
            .map_err(|_| "mod storage stopped".to_string())?;
        let path = self.key_path(plugin, "store", key)?;
        self.check_parents(
            path.parent()
                .ok_or_else(|| "mod store has no parent".to_string())?,
        )?;
        let path = PathUri::from_host_native_path(&path).map_err(|error| error.to_string())?;
        let sandbox = None;
        self.file_system
            .remove(
                &path,
                RemoveOptions {
                    recursive: false,
                    force: true,
                    follow_symlinks: false,
                },
                sandbox,
            )
            .await
            .map_err(|error| error.to_string())?;
        Ok(Value::Null)
    }

    fn key_path(&self, plugin: &str, namespace: &str, key: &str) -> ModResult<PathBuf> {
        if plugin.len() > 256 || key.len() > 256 {
            return Err("mod storage key exceeds 256 bytes".into());
        }
        let encode = |value: &str| {
            value
                .bytes()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        Ok(self
            .root
            .join("persistent")
            .join(encode(plugin))
            .join(namespace)
            .join(format!("{}.json", encode(key))))
    }
}
