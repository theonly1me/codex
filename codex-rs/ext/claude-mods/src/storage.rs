use crate::ModResult;
use codex_exec_server::LocalFileSystem;
use codex_file_system::CreateDirectoryOptions;
use codex_file_system::ExecutorFileSystem;
use codex_file_system::ReadFileOptions;
use codex_utils_path_uri::PathUri;
use serde_json::Value;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use tokio::sync::Semaphore;

pub(crate) struct Storage {
    pub home: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) file_system: LocalFileSystem,
    pub(crate) transaction: Semaphore,
}

impl Storage {
    pub(crate) fn new(codex_home: &Path) -> ModResult<Self> {
        let root = codex_home.join("claude-mods");
        if std::fs::symlink_metadata(&root).is_ok_and(|metadata| metadata.is_symlink()) {
            return Err(format!(
                "{}: mod storage refuses a symbolic link",
                root.display()
            ));
        }
        std::fs::create_dir_all(&root).map_err(|error| format!("{}: {error}", root.display()))?;
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        let home = root.join("home");
        Ok(Self {
            home,
            root,
            file_system: LocalFileSystem::unsandboxed(),
            transaction: Semaphore::new(1),
        })
    }

    pub(crate) fn owns_path(&self, path: &Path) -> bool {
        path.starts_with(&self.home)
            && !path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
    }

    pub(crate) async fn read_owned(&self, path: &Path) -> ModResult<String> {
        if !self.owns_path(path) {
            return Err("path is outside the Codex mod home".into());
        }
        let _transaction = self
            .transaction
            .acquire()
            .await
            .map_err(|_| "mod storage stopped".to_string())?;
        let path = PathUri::from_host_native_path(path).map_err(|error| error.to_string())?;
        let sandbox = None;
        self.file_system
            .read_file_text(
                &path,
                ReadFileOptions {
                    follow_symlinks: false,
                },
                sandbox,
            )
            .await
            .map_err(|error| error.to_string())
    }

    pub(crate) async fn write_owned(&self, path: &Path, contents: String) -> ModResult<Value> {
        if !self.owns_path(path) {
            return Err("path is outside the Codex mod home".into());
        }
        let _transaction = self
            .transaction
            .acquire()
            .await
            .map_err(|_| "mod storage stopped".to_string())?;
        self.write(path, contents.into_bytes()).await?;
        Ok(Value::Null)
    }

    pub(crate) async fn write(&self, path: &Path, contents: Vec<u8>) -> ModResult<()> {
        let parent = path
            .parent()
            .ok_or_else(|| "mod state has no parent directory".to_string())?;
        self.check_parents(parent)?;
        let parent = PathUri::from_host_native_path(parent).map_err(|error| error.to_string())?;
        let sandbox = None;
        self.file_system
            .create_directory(
                &parent,
                CreateDirectoryOptions {
                    recursive: true,
                    follow_symlinks: false,
                },
                sandbox,
            )
            .await
            .map_err(|error| error.to_string())?;
        if std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_symlink()) {
            return Err(format!(
                "{}: mod storage refuses a symbolic link",
                path.display()
            ));
        }
        let contents = String::from_utf8(contents).map_err(|error| error.to_string())?;
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || codex_utils_path::write_atomically(&path, &contents))
            .await
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())
    }

    pub(crate) fn check_parents(&self, path: &Path) -> ModResult<()> {
        for ancestor in path
            .ancestors()
            .take_while(|ancestor| ancestor.starts_with(&self.root))
        {
            match std::fs::symlink_metadata(ancestor) {
                Ok(metadata) if metadata.is_symlink() => {
                    return Err(format!(
                        "{}: mod storage refuses a symbolic link",
                        ancestor.display()
                    ));
                }
                Ok(metadata) if !metadata.is_dir() => {
                    return Err(format!(
                        "{}: mod storage requires a directory",
                        ancestor.display()
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(())
    }
}
