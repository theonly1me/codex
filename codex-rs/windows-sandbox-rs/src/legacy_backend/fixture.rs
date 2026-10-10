use anyhow::Result;
use codex_utils_absolute_path::AbsolutePathBuf;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

pub(super) struct Fixture {
    _temporary: TempDir,
    pub(super) workspace: PathBuf,
    pub(super) home: PathBuf,
    outside: PathBuf,
    marker: PathBuf,
    script: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = std::env::var_os("INSTA_WORKSPACE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("workspace root")
                    .to_path_buf()
            });
        let temporary = TempDir::new_in(root).expect("create rejection fixture");
        let workspace = temporary.path().join("workspace");
        let home = temporary.path().join("codex-home");
        let outside = temporary.path().join("outside.txt");
        let marker = workspace.join("command-started.txt");
        let script = workspace.join("delete.cmd");
        fs::create_dir(&workspace).expect("create workspace");
        fs::create_dir(workspace.join(".git")).expect("create protected metadata");
        fs::write(&outside, "outside").expect("seed outside data");
        fs::write(
            &script,
            format!(
                "@echo off\r\necho started>\"{}\"\r\ndel /f /q \"{}\"\r\nrmdir \"{}\"\r\nexit /b 0\r\n",
                marker.display(),
                outside.display(),
                workspace.join(".git").display(),
            ),
        )
        .expect("write deletion command");
        Self {
            _temporary: temporary,
            workspace,
            home,
            outside,
            marker,
            script,
        }
    }

    pub(super) fn command(&self) -> Vec<String> {
        vec![
            "C:\\Windows\\System32\\cmd.exe".to_string(),
            "/d".to_string(),
            "/c".to_string(),
            self.script.to_string_lossy().into_owned(),
        ]
    }

    pub(super) fn roots(&self) -> Vec<AbsolutePathBuf> {
        vec![AbsolutePathBuf::from_absolute_path(&self.workspace).expect("absolute workspace")]
    }

    pub(super) fn environment(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    pub(super) fn assert_rejected<T>(&self, result: Result<T>) {
        let error = match result {
            Ok(_) => panic!("the unsafe legacy launch succeeded"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("cannot enforce deletion isolation")
        );
        assert!(error.to_string().contains("elevated"));
        assert_eq!(fs::read_to_string(&self.outside).unwrap(), "outside");
        assert!(self.workspace.join(".git").is_dir());
        assert!(!self.marker.exists(), "the unsafe command started");
        assert!(!self.home.exists(), "rejection changed the sandbox home");
    }
}
