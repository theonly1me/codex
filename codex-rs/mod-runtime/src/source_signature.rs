use crate::ModResult;
use crate::source::read_source;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;

pub(crate) fn compute(root: &Path) -> ModResult<u64> {
    let mut hasher = DefaultHasher::new();
    let mut count = 0;
    visit(root, &mut hasher, &mut count)?;
    Ok(hasher.finish())
}

fn visit(directory: &Path, hasher: &mut DefaultHasher, count: &mut usize) -> ModResult<()> {
    let mut paths = std::fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<PathBuf>, _>>()
        .map_err(|error| error.to_string())?;
    paths.sort();
    for path in paths {
        *count += 1;
        if *count > 4096 {
            return Err("mod source tree exceeds 4096 entries".into());
        }
        let metadata = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.is_dir() {
            if path
                .file_name()
                .is_some_and(|name| name == "node_modules" || name == ".git")
            {
                continue;
            }
            visit(&path, hasher, count)?;
        } else if metadata.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    ["ts", "tsx", "js", "jsx", "mts", "mjs", "cts", "cjs", "json"]
                        .contains(&extension)
                })
        {
            path.hash(hasher);
            read_source(&path)?.hash(hasher);
        }
    }
    Ok(())
}
