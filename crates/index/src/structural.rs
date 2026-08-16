use crate::error::{IndexError, Result};
use crate::languages::SourceLanguage;
use rune_core::NodeKind;
use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};

const MAX_PARSE_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParsedFile {
    pub language: String,
    pub symbols: Vec<ParsedSymbol>,
    pub imports: Vec<ParsedImport>,
    pub exports: Vec<String>,
    pub calls: Vec<ParsedCall>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParsedSymbol {
    pub name: String,
    pub kind: NodeKind,
    pub start_byte: u32,
    pub end_byte: u32,
    pub start_line: u32,
    pub end_line: u32,
    pub is_test: bool,
    pub test_framework: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParsedImport {
    pub source: String,
    pub names: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParsedCall {
    pub callee: String,
    pub caller: Option<String>,
    pub start_line: u32,
}

pub fn parse_source(language: SourceLanguage, path: &std::path::Path, source: &str) -> Result<ParsedFile> {
    if source.len() > MAX_PARSE_BYTES {
        return Err(IndexError::msg(format!(
            "refusing to parse {} ({} bytes) above {MAX_PARSE_BYTES}",
            path.display(),
            source.len()
        )));
    }
    if source.as_bytes().contains(&0) {
        return Err(IndexError::msg(format!("binary file not parsed: {}", path.display())));
    }
    let lang = ts_language(language)?;
    let mut parser = Parser::new();
    parser
        .set_language(&lang)
        .map_err(|err| IndexError::TreeSitter(err.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| IndexError::TreeSitter("parser returned no tree".into()))?;
    let mut out = ParsedFile {
        language: language.as_str().to_string(),
        symbols: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        calls: Vec::new(),
    };
    let root = tree.root_node();
    match language {
        SourceLanguage::Rust => walk_rust(root, source.as_bytes(), &mut out, RustCtx::default(), None),
        SourceLanguage::Python => walk_python(root, source.as_bytes(), &mut out, None),
        SourceLanguage::JavaScript | SourceLanguage::TypeScript | SourceLanguage::Tsx => {
            walk_js(root, source.as_bytes(), &mut out, None)
        }
        SourceLanguage::Go => walk_go(root, source.as_bytes(), path, &mut out, None),
        _ => {}
    }
    drop(tree);
    Ok(out)
}

fn ts_language(language: SourceLanguage) -> Result<tree_sitter::Language> {
    Ok(match language {
        SourceLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
        SourceLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        SourceLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        SourceLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        SourceLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        SourceLanguage::Go => tree_sitter_go::LANGUAGE.into(),
        other => {
            return Err(IndexError::TreeSitter(format!(
                "no tree-sitter grammar for {}",
                other.as_str()
            )))
        }
    })
}

#[derive(Clone, Copy, Default)]
struct RustCtx {
    in_impl: bool,
    in_trait: bool,
}

fn walk_rust(node: Node<'_>, src: &[u8], out: &mut ParsedFile, ctx: RustCtx, caller: Option<String>) {
    match node.kind() {
        "function_item" => {
            if let Some(name) = field_text(node, "name", src) {
                let is_test = rust_has_test_attr(node, src);
                let kind = if is_test {
                    NodeKind::Test
                } else if ctx.in_impl || ctx.in_trait {
                    NodeKind::Method
                } else {
                    NodeKind::Function
                };
                out.symbols.push(symbol(
                    name.clone(),
                    kind,
                    node,
                    is_test,
                    is_test.then_some("cargo_test"),
                ));
                for child in children(node) {
                    walk_rust(child, src, out, ctx, Some(name.clone()));
                }
                return;
            }
        }
        "struct_item" => push_named(node, src, out, NodeKind::Type, false, None),
        "enum_item" => push_named(node, src, out, NodeKind::Type, false, None),
        "trait_item" => {
            push_named(node, src, out, NodeKind::Trait, false, None);
            for child in children(node) {
                walk_rust(child, src, out, RustCtx { in_trait: true, in_impl: false }, caller.clone());
            }
            return;
        }
        "impl_item" => {
            for child in children(node) {
                walk_rust(child, src, out, RustCtx { in_impl: true, in_trait: false }, caller.clone());
            }
            return;
        }
        "mod_item" => push_named(node, src, out, NodeKind::Module, false, None),
        "type_item" => push_named(node, src, out, NodeKind::Type, false, None),
        "const_item" | "static_item" => push_named(node, src, out, NodeKind::Variable, false, None),
        "use_declaration" => {
            if let Some(text) = node_text(node, src) {
                let source = text
                    .trim()
                    .trim_start_matches("pub ")
                    .trim_start_matches("use ")
                    .trim_end_matches(';')
                    .trim()
                    .to_string();
                if !source.is_empty() {
                    out.imports.push(ParsedImport {
                        source,
                        names: Vec::new(),
                    });
                }
            }
        }
        "call_expression" => {
            if let Some(func) = node.child_by_field_name("function") {
                if let Some(callee) = callee_name(func, src) {
                    out.calls.push(ParsedCall {
                        callee,
                        caller: caller.clone(),
                        start_line: (node.start_position().row as u32) + 1,
                    });
                }
            }
        }
        _ => {}
    }
    for child in children(node) {
        walk_rust(child, src, out, ctx, caller.clone());
    }
}

fn walk_python(node: Node<'_>, src: &[u8], out: &mut ParsedFile, caller: Option<String>) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = field_text(node, "name", src) {
                let is_test = name.starts_with("test_");
                let kind = if is_test { NodeKind::Test } else { NodeKind::Function };
                out.symbols.push(symbol(
                    name.clone(),
                    kind,
                    node,
                    is_test,
                    is_test.then_some("pytest"),
                ));
                for child in children(node) {
                    walk_python(child, src, out, Some(name.clone()));
                }
                return;
            }
        }
        "class_definition" => {
            if let Some(name) = field_text(node, "name", src) {
                let is_test = name.starts_with("Test");
                out.symbols.push(symbol(
                    name.clone(),
                    if is_test { NodeKind::Test } else { NodeKind::Class },
                    node,
                    is_test,
                    is_test.then_some("pytest"),
                ));
            }
        }
        "import_statement" | "import_from_statement" => {
            if let Some(text) = node_text(node, src) {
                let source = python_import_source(&text);
                out.imports.push(ParsedImport {
                    source,
                    names: Vec::new(),
                });
            }
        }
        "call" => {
            if let Some(func) = node.child_by_field_name("function") {
                if let Some(callee) = callee_name(func, src) {
                    out.calls.push(ParsedCall {
                        callee,
                        caller: caller.clone(),
                        start_line: (node.start_position().row as u32) + 1,
                    });
                }
            }
        }
        _ => {}
    }
    for child in children(node) {
        walk_python(child, src, out, caller.clone());
    }
}

fn walk_js(node: Node<'_>, src: &[u8], out: &mut ParsedFile, caller: Option<String>) {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            if let Some(name) = field_text(node, "name", src) {
                out.symbols.push(symbol(name.clone(), NodeKind::Function, node, false, None));
                for child in children(node) {
                    walk_js(child, src, out, Some(name.clone()));
                }
                return;
            }
        }
        "method_definition" => {
            if let Some(name) = field_text(node, "name", src) {
                out.symbols.push(symbol(name.clone(), NodeKind::Method, node, false, None));
                for child in children(node) {
                    walk_js(child, src, out, Some(name.clone()));
                }
                return;
            }
        }
        "class_declaration" => {
            if let Some(name) = field_text(node, "name", src) {
                out.symbols.push(symbol(name, NodeKind::Class, node, false, None));
            }
        }
        "lexical_declaration" | "variable_declaration" => {
            if let Some(name) = js_first_declarator_name(node, src) {
                if has_function_init(node) {
                    out.symbols.push(symbol(name.clone(), NodeKind::Function, node, false, None));
                    for child in children(node) {
                        walk_js(child, src, out, Some(name.clone()));
                    }
                    return;
                }
            }
        }
        "import_statement" => {
            if let Some(source) = js_string_literal(node, src) {
                out.imports.push(ParsedImport {
                    source,
                    names: Vec::new(),
                });
            }
        }
        "export_statement" => {
            if let Some(text) = node_text(node, src) {
                out.exports.push(text.trim().to_string());
            }
        }
        "call_expression" => {
            if let Some(func) = node.child_by_field_name("function") {
                if let Some(callee) = callee_name(func, src) {
                    if matches!(callee.as_str(), "it" | "test" | "describe") {
                        let title = js_first_string_arg(node, src).unwrap_or_else(|| callee.clone());
                        out.symbols.push(symbol(
                            title.clone(),
                            NodeKind::Test,
                            node,
                            true,
                            Some("jest"),
                        ));
                        for child in children(node) {
                            walk_js(child, src, out, Some(title.clone()));
                        }
                        return;
                    }
                    out.calls.push(ParsedCall {
                        callee,
                        caller: caller.clone(),
                        start_line: (node.start_position().row as u32) + 1,
                    });
                }
            }
        }
        _ => {}
    }
    for child in children(node) {
        walk_js(child, src, out, caller.clone());
    }
}

fn walk_go(node: Node<'_>, src: &[u8], path: &std::path::Path, out: &mut ParsedFile, caller: Option<String>) {
    let test_file = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.ends_with("_test.go"))
        .unwrap_or(false);
    match node.kind() {
        "function_declaration" => {
            if let Some(name) = field_text(node, "name", src) {
                let is_test = test_file
                    && (name.starts_with("Test") || name.starts_with("Benchmark") || name.starts_with("Example") || name.starts_with("Fuzz"));
                let kind = if is_test { NodeKind::Test } else { NodeKind::Function };
                out.symbols.push(symbol(
                    name.clone(),
                    kind,
                    node,
                    is_test,
                    is_test.then_some("go_test"),
                ));
                for child in children(node) {
                    walk_go(child, src, path, out, Some(name.clone()));
                }
                return;
            }
        }
        "method_declaration" => {
            if let Some(name) = field_text(node, "name", src) {
                out.symbols.push(symbol(name.clone(), NodeKind::Method, node, false, None));
                for child in children(node) {
                    walk_go(child, src, path, out, Some(name.clone()));
                }
                return;
            }
        }
        "type_declaration" => {
            for child in children(node) {
                if child.kind() == "type_spec" {
                    if let Some(name) = field_text(child, "name", src) {
                        out.symbols.push(symbol(name, NodeKind::Type, child, false, None));
                    }
                }
            }
        }
        "import_spec" => {
            if let Some(source) = js_string_literal(node, src) {
                out.imports.push(ParsedImport {
                    source,
                    names: Vec::new(),
                });
            }
        }
        "call_expression" => {
            if let Some(func) = node.child_by_field_name("function") {
                if let Some(callee) = callee_name(func, src) {
                    out.calls.push(ParsedCall {
                        callee,
                        caller: caller.clone(),
                        start_line: (node.start_position().row as u32) + 1,
                    });
                }
            }
        }
        _ => {}
    }
    for child in children(node) {
        walk_go(child, src, path, out, caller.clone());
    }
}

fn push_named(
    node: Node<'_>,
    src: &[u8],
    out: &mut ParsedFile,
    kind: NodeKind,
    is_test: bool,
    framework: Option<&str>,
) {
    if let Some(name) = field_text(node, "name", src) {
        out.symbols
            .push(symbol(name, kind, node, is_test, framework));
    }
}

fn symbol(
    name: String,
    kind: NodeKind,
    node: Node<'_>,
    is_test: bool,
    test_framework: Option<&str>,
) -> ParsedSymbol {
    ParsedSymbol {
        name,
        kind,
        start_byte: node.start_byte() as u32,
        end_byte: node.end_byte() as u32,
        start_line: node.start_position().row as u32 + 1,
        end_line: node.end_position().row as u32 + 1,
        is_test,
        test_framework: test_framework.map(ToOwned::to_owned),
    }
}

fn children<'a>(node: Node<'a>) -> impl Iterator<Item = Node<'a>> {
    (0..node.child_count()).filter_map(move |i| node.child(i))
}

fn field_text(node: Node<'_>, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|child| node_text(child, src))
}

fn node_text(node: Node<'_>, src: &[u8]) -> Option<String> {
    node.utf8_text(src).ok().map(|s| s.to_string())
}

fn callee_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "property_identifier" | "field_identifier" | "type_identifier" => node_text(node, src),
        "field_expression" | "member_expression" => node
            .child_by_field_name("field")
            .or_else(|| node.child_by_field_name("property"))
            .and_then(|n| node_text(n, src))
            .or_else(|| {
                node.named_child(node.named_child_count().saturating_sub(1))
                    .and_then(|n| node_text(n, src))
            }),
        "scoped_identifier" | "selector_expression" => node
            .named_child(node.named_child_count().saturating_sub(1))
            .and_then(|n| node_text(n, src)),
        _ => node
            .named_child(0)
            .and_then(|n| callee_name(n, src))
            .or_else(|| node_text(node, src)),
    }
}

fn rust_has_test_attr(node: Node<'_>, src: &[u8]) -> bool {
    for child in children(node) {
        if matches!(child.kind(), "attribute_item" | "inner_attribute_item") {
            if let Some(text) = node_text(child, src) {
                if is_test_attribute(&text) {
                    return true;
                }
            }
        }
    }
    let mut prev = node.prev_named_sibling();
    while let Some(p) = prev {
        if p.kind() == "attribute_item" {
            if let Some(text) = node_text(p, src) {
                if is_test_attribute(&text) {
                    return true;
                }
            }
            prev = p.prev_named_sibling();
        } else {
            break;
        }
    }
    false
}

fn is_test_attribute(text: &str) -> bool {
    let t = text.trim();
    t.contains("#[test]")
        || t.contains("#[tokio::test")
        || t.contains("#[async_std::test")
        || t.contains("::test]")
        || t.contains("::test(")
}

fn python_import_source(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("from ") {
        rest.split(" import ").next().unwrap_or(rest).trim().to_string()
    } else {
        trimmed.trim_start_matches("import ").trim().to_string()
    }
}

fn js_string_literal(node: Node<'_>, src: &[u8]) -> Option<String> {
    for child in children(node) {
        if matches!(child.kind(), "string" | "string_fragment" | "interpreted_string_literal") {
            if let Some(text) = node_text(child, src) {
                return Some(text.trim_matches(['\'', '"', '`']).to_string());
            }
        }
        if let Some(found) = js_string_literal(child, src) {
            return Some(found);
        }
    }
    None
}

fn js_first_string_arg(node: Node<'_>, src: &[u8]) -> Option<String> {
    let args = node.child_by_field_name("arguments")?;
    js_string_literal(args, src)
}

fn js_first_declarator_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    for child in children(node) {
        if child.kind() == "variable_declarator" {
            return field_text(child, "name", src);
        }
    }
    None
}

fn has_function_init(node: Node<'_>) -> bool {
    fn walk(node: Node<'_>) -> bool {
        if matches!(
            node.kind(),
            "arrow_function" | "function" | "function_expression" | "generator_function"
        ) {
            return true;
        }
        children(node).any(walk)
    }
    walk(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn rust_function_and_test_are_indexed() {
        let src = r#"
            pub fn greet() {}
            #[test]
            fn greet_ok() { greet(); }
        "#;
        let parsed = parse_source(SourceLanguage::Rust, Path::new("lib.rs"), src).unwrap();
        assert!(parsed.symbols.iter().any(|s| s.name == "greet" && s.kind == NodeKind::Function));
        assert!(parsed.symbols.iter().any(|s| s.name == "greet_ok" && s.is_test));
        assert!(parsed.calls.iter().any(|c| c.callee == "greet"));
    }
}
