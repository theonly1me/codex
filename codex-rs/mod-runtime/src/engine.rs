use crate::ModPlugin;
use crate::ModResult;
use crate::bridge::Bridge;
use crate::module_loader::ModuleLoader;
use crate::module_loader::compile;
use crate::module_loader::resolve;
use crate::value::decode;
use crate::value::encode;
use crate::value::error_text;
use serde_json::Value;

pub(crate) fn install(scope: &mut v8::PinScope<'_, '_>) -> ModResult<()> {
    let global = scope.get_current_context().global(scope);
    let host = v8::Function::new(scope, crate::bridge::host_callback)
        .ok_or_else(|| "cannot create mod host bridge".to_string())?;
    let settle = v8::Function::new(scope, crate::bridge::settle_callback)
        .ok_or_else(|| "cannot create mod completion bridge".to_string())?;
    for (name, function) in [("__modsHost", host), ("__modsSettle", settle)] {
        let key = v8::String::new(scope, name)
            .ok_or_else(|| "cannot allocate bridge name".to_string())?;
        if global.set(scope, key.into(), function.into()) != Some(true) {
            return Err(format!("cannot install {name}"));
        }
    }
    evaluate(scope, "codex:mods/bootstrap")?;
    Ok(())
}

fn evaluate<'scope>(
    scope: &mut v8::PinScope<'scope, '_>,
    path: &str,
) -> ModResult<v8::Local<'scope, v8::Module>> {
    let module = compile(scope, path)?;
    if module.instantiate_module(scope, resolve) != Some(true) {
        return Err(format!("{path}: cannot instantiate module"));
    }
    let evaluated = module
        .evaluate(scope)
        .ok_or_else(|| format!("{path}: cannot evaluate module"))?;
    scope.perform_microtask_checkpoint();
    if let Ok(promise) = v8::Local::<v8::Promise>::try_from(evaluated) {
        match promise.state() {
            v8::PromiseState::Rejected => return Err(error_text(scope, promise.result(scope))),
            v8::PromiseState::Pending => {
                return Err(format!(
                    "{path}: asynchronous module initialization is unsupported"
                ));
            }
            v8::PromiseState::Fulfilled => {}
        }
    }
    Ok(module)
}

pub(crate) fn load(scope: &mut v8::PinScope<'_, '_>, plugin: &ModPlugin) -> ModResult<()> {
    if let Some(loader) = scope.get_slot_mut::<ModuleLoader>() {
        loader.root = plugin.root.clone();
    }
    for path in std::iter::once(&plugin.module).chain(&plugin.additional_modules) {
        let module = evaluate(scope, &path.to_string_lossy())?;
        let namespace = v8::Local::<v8::Object>::try_from(module.get_module_namespace())
            .map_err(|_| "mod module has no namespace".to_string())?;
        let key = v8::String::new(scope, "register")
            .ok_or_else(|| "cannot allocate register name".to_string())?;
        let register = namespace
            .get(scope, key.into())
            .ok_or_else(|| "mod has no register export".to_string())?;
        if !register.is_function() {
            return Err(format!("{}: register must be a function", plugin.name));
        }
        let metadata = encode(
            scope,
            &serde_json::json!({"id":plugin.id,"name":plugin.name,"root":plugin.root}),
        )?;
        let options = encode(scope, &plugin.options)?;
        call(scope, "register", &[metadata, register, options])?;
    }
    Ok(())
}

pub(crate) fn call<'scope>(
    scope: &mut v8::PinScope<'scope, '_>,
    method: &str,
    arguments: &[v8::Local<'scope, v8::Value>],
) -> ModResult<v8::Local<'scope, v8::Value>> {
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "__mods")
        .ok_or_else(|| "cannot allocate runtime name".to_string())?;
    let object = global
        .get(scope, key.into())
        .ok_or_else(|| "mod runtime is unavailable".to_string())?;
    let object =
        v8::Local::<v8::Object>::try_from(object).map_err(|_| "invalid mod runtime".to_string())?;
    let key = v8::String::new(scope, method)
        .ok_or_else(|| "cannot allocate runtime method".to_string())?;
    let function = object
        .get(scope, key.into())
        .ok_or_else(|| format!("missing mod runtime method {method}"))?;
    let function = v8::Local::<v8::Function>::try_from(function)
        .map_err(|_| format!("invalid runtime method {method}"))?;
    function
        .call(scope, object.into(), arguments)
        .ok_or_else(|| format!("mod runtime method {method} failed"))
}

pub(crate) fn resolve_promise(
    scope: &mut v8::PinScope<'_, '_>,
    call_id: u64,
    response: ModResult<Value>,
) -> ModResult<()> {
    let resolver = scope
        .get_slot_mut::<Bridge>()
        .and_then(|bridge| bridge.promises.remove(&call_id));
    let Some(resolver) = resolver else {
        return Ok(());
    };
    let resolver = v8::Local::new(scope, resolver);
    match response {
        Ok(value) => {
            let value = encode(scope, &value)?;
            resolver.resolve(scope, value);
        }
        Err(error) => {
            let value = encode(scope, &Value::String(error))?;
            resolver.reject(scope, value);
        }
    }
    Ok(())
}

pub(crate) fn caught(
    scope: &mut v8::PinScope<'_, '_>,
    operation: impl FnOnce(&mut v8::PinScope<'_, '_>) -> ModResult<()>,
) -> ModResult<()> {
    let catcher = std::pin::pin!(v8::TryCatch::new(scope));
    let mut catcher = catcher.init();
    let outcome = operation(&mut catcher);
    if let Some(error) = catcher.exception() {
        return Err(error_text(&mut catcher, error));
    }
    outcome
}

pub(crate) fn inspect(scope: &mut v8::PinScope<'_, '_>, method: &str) -> ModResult<Value> {
    let value = call(scope, method, &[])?;
    decode(scope, value)
}
