use std::io;
use std::path::Path;

pub(super) fn stage() -> io::Result<()> {
    let executable = std::env::current_exe()?;
    let resources = executable
        .parent()
        .ok_or_else(|| io::Error::other("Windows test executable has no parent directory"))?
        .join("codex-resources");
    if let Err(error) = std::fs::create_dir_all(&resources)
        && !(error.kind() == io::ErrorKind::PermissionDenied && resources.is_dir())
    {
        return Err(error);
    }
    for name in ["codex-windows-sandbox-setup", "codex-command-runner"] {
        let source = codex_utils_cargo_bin::cargo_bin(name).map_err(io::Error::other)?;
        let destination = resources.join(Path::new(name).with_extension("exe"));
        if let Err(error) = codex_utils_cargo_bin::copy_executable(&source, &destination)
            && !(error.kind() == io::ErrorKind::PermissionDenied && destination.is_file())
        {
            return Err(io::Error::new(
                error.kind(),
                format!("stage Windows sandbox helper {name}: {error}"),
            ));
        }
    }
    Ok(())
}
