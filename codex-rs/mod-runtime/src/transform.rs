use crate::ModResult;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::JsxOptions;
use oxc_transformer::JsxRuntime;
use oxc_transformer::TransformOptions;
use oxc_transformer::Transformer;
use std::path::Path;

pub(crate) fn transform(path: &Path, source: &str) -> ModResult<String> {
    let allocator = Allocator::default();
    let source_type =
        SourceType::from_path(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        return Err(format!("{}: {:?}", path.display(), parsed.diagnostics));
    }
    let mut program = parsed.program;
    let semantic = SemanticBuilder::new()
        .with_excess_capacity(2.0)
        .build(&program);
    if !semantic.diagnostics.is_empty() {
        return Err(format!("{}: {:?}", path.display(), semantic.diagnostics));
    }
    let options = TransformOptions {
        jsx: JsxOptions {
            runtime: JsxRuntime::Classic,
            pragma: Some("__mods.h".into()),
            pragma_frag: Some("__mods.Fragment".into()),
            ..JsxOptions::default()
        },
        ..TransformOptions::default()
    };
    let transformed = Transformer::new(&allocator, path, &options)
        .build_with_scoping(semantic.semantic.into_scoping(), &mut program);
    if !transformed.diagnostics.is_empty() {
        return Err(format!("{}: {:?}", path.display(), transformed.diagnostics));
    }
    Ok(Codegen::new().build(&program).code)
}
