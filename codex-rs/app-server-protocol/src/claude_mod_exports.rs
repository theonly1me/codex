use std::path::Path;

pub(crate) fn is_claude_mod_export(path: &Path) -> bool {
    let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with("ClaudeMod")
        || [
            "ModCommand",
            "ModPane",
            "ModNode",
            "ModElement",
            "ModRenderRequest",
        ]
        .contains(&name)
}

pub(crate) fn declaration(contents: &str) -> String {
    contents
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
