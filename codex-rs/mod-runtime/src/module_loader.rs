use crate::ModResult;
use crate::source::read_source;
use crate::transform::transform;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

pub(crate) struct ModuleLoader {
    pub root: PathBuf,
    pub modules: HashMap<String, v8::Global<v8::Module>>,
    pub paths: HashMap<i32, String>,
}

const BUILTINS: &[(&str, &str)] = &[
    ("codex:mods/bootstrap", include_str!("runtime/bootstrap.js")),
    ("codex:mods/events", include_str!("runtime/events.js")),
    ("codex:mods/api", include_str!("runtime/api.js")),
    ("codex:mods/ui", include_str!("runtime/ui.js")),
    ("codex:mods/state", include_str!("runtime/state.js")),
    ("codex:mods/clock", include_str!("runtime/clock.js")),
    (
        "codex:mods/cancellation",
        include_str!("runtime/cancellation.js"),
    ),
    ("codex:mods/encoding", include_str!("runtime/encoding.js")),
    ("claude-code", include_str!("runtime/compatibility.js")),
    (
        "claude-code/jsx-runtime",
        include_str!("runtime/jsx-runtime.js"),
    ),
];

pub(crate) fn compile<'scope>(
    scope: &mut v8::PinScope<'scope, '_>,
    path: &str,
) -> ModResult<v8::Local<'scope, v8::Module>> {
    if let Some(module) = scope
        .get_slot::<ModuleLoader>()
        .and_then(|loader| loader.modules.get(path))
    {
        return Ok(v8::Local::new(scope, module));
    }
    let source = if let Some((_, source)) = BUILTINS.iter().find(|(name, _)| *name == path) {
        source.to_string()
    } else {
        transform(Path::new(path), &read_source(Path::new(path))?)?
    };
    let source = v8::String::new(scope, &source)
        .ok_or_else(|| "cannot allocate module source".to_string())?;
    let name =
        v8::String::new(scope, path).ok_or_else(|| "cannot allocate module name".to_string())?;
    let origin = v8::ScriptOrigin::new(
        scope,
        name.into(),
        0,
        0,
        false,
        0,
        None,
        false,
        false,
        true,
        None,
    );
    let mut source = v8::script_compiler::Source::new(source, Some(&origin));
    let module = v8::script_compiler::compile_module(scope, &mut source)
        .ok_or_else(|| format!("{path}: JavaScript compilation failed"))?;
    let identity = module.get_identity_hash().get();
    let global = v8::Global::new(scope, module);
    if let Some(loader) = scope.get_slot_mut::<ModuleLoader>() {
        loader.modules.insert(path.to_string(), global);
        loader.paths.insert(identity, path.to_string());
    }
    Ok(module)
}

pub(crate) fn resolve<'scope>(
    context: v8::Local<'scope, v8::Context>,
    specifier: v8::Local<'scope, v8::String>,
    _attributes: v8::Local<'scope, v8::FixedArray>,
    referrer: v8::Local<'scope, v8::Module>,
) -> Option<v8::Local<'scope, v8::Module>> {
    v8::callback_scope!(unsafe scope, context);
    let specifier = specifier.to_rust_string_lossy(scope);
    let referrer_id = referrer.get_identity_hash().get();
    let resolved = (|| {
        let loader = scope
            .get_slot::<ModuleLoader>()
            .ok_or_else(|| "mod loader stopped".to_string())?;
        let parent = loader
            .paths
            .get(&referrer_id)
            .ok_or_else(|| "unknown importing module".to_string())?;
        if specifier == "claude-code"
            || specifier == "claude-code/jsx-runtime"
            || (specifier.starts_with("codex:mods/")
                && (parent.starts_with("codex:mods/") || parent.starts_with("claude-code")))
        {
            return Ok(specifier);
        }
        if !specifier.starts_with("./") && !specifier.starts_with("../") {
            return Err(format!("unsupported mod import `{specifier}`"));
        }
        let parent = Path::new(parent)
            .parent()
            .ok_or_else(|| "module has no parent".to_string())?;
        resolve_path(&loader.root, &parent.join(&specifier))
            .map(|path| path.to_string_lossy().into_owned())
    })();
    match resolved.and_then(|path| compile(scope, &path)) {
        Ok(module) => Some(module),
        Err(error) => {
            if let Some(error) = v8::String::new(scope, &error) {
                scope.throw_exception(error.into());
            }
            None
        }
    }
}

fn resolve_path(root: &Path, requested: &Path) -> ModResult<PathBuf> {
    let candidates = std::iter::once(requested.to_path_buf())
        .chain(
            ["ts", "tsx", "js", "jsx", "mts", "mjs", "cts", "cjs"]
                .into_iter()
                .map(|extension| requested.with_extension(extension)),
        )
        .chain(
            ["index.ts", "index.tsx", "index.js"]
                .into_iter()
                .map(|name| requested.join(name)),
        );
    let resolved = candidates
        .filter(|path| path.is_file())
        .find_map(|path| path.canonicalize().ok())
        .ok_or_else(|| format!("{}: module not found", requested.display()))?;
    if !resolved.starts_with(root) {
        return Err(format!(
            "{}: import escapes plugin root {}",
            resolved.display(),
            root.display()
        ));
    }
    Ok(resolved)
}
