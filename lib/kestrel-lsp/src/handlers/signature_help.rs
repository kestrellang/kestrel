//! `textDocument/signatureHelp` — popup that appears when typing `(` or
//! `,` inside a function call, listing the signature(s) of the callee with
//! the active parameter highlighted.
//!
//! Dispatch:
//!   1. Find the smallest enclosing `ExprCall` whose `ArgumentList` contains
//!      the cursor (cursor on the callee identifier itself doesn't trigger).
//!   2. Locate the cursor among the `Argument` children of that ArgumentList
//!      → active parameter index. Direct comma-counting doesn't work because
//!      the parser folds each leading `,` into the *following* `Argument`
//!      node.
//!   3. Resolve the callee to one or more candidate entities. A direct
//!      `HirExpr::Call { callee }` whose callee is a `Def` or `OverloadSet`
//!      gives the candidates straight from name resolution; method /
//!      protocol calls and overload-resolved Call exprs go through
//!      `TypedBody::resolutions`.
//!   4. Render each candidate as `name(label: Type, ...) -> Ret` with
//!      character offsets for each parameter so VS Code can highlight by
//!      label range.

use kestrel_ast_builder::{AstType, Callable, Name, NodeKind, TypeAnnotation};
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::{HirBody, HirExpr, HirExprId};
use kestrel_hir_lower::LowerBody;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use kestrel_type_infer::result::TypedBody;
use kestrel_type_infer::InferBody;
use rowan::TextSize;
use std::collections::HashMap;
use tower_lsp::lsp_types::{
    ParameterInformation, ParameterLabel, SignatureHelp, SignatureHelpParams, SignatureInformation,
};

use crate::semantic;
use crate::server::{rebuild_compiler, url_to_path, SharedState};

pub async fn handle(state: SharedState, params: SignatureHelpParams) -> Option<SignatureHelp> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let path = url_to_path(&uri);

    let (sources, line_index) = {
        let s = state.lock().await;
        let li = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        (s.sources.clone(), li)
    };
    let offset = line_index.position_to_offset(pos);

    tokio::task::spawn_blocking(move || -> Option<SignatureHelp> {
        let (compiler, _) = rebuild_compiler(&sources);
        let file_entity = semantic::file_entity_for_path(&compiler, &path)?;
        signature_help_at(compiler.world(), &sources, file_entity, offset, compiler.root())
    })
    .await
    .ok()?
}

/// Sync core. Returns a SignatureHelp for the call enclosing `offset`, or
/// `None` if the cursor isn't inside the argument list of any call.
pub fn signature_help_at(
    world: &World,
    sources: &HashMap<String, String>,
    file_entity: Entity,
    offset: usize,
    root: Entity,
) -> Option<SignatureHelp> {
    // 1. Find the enclosing call via CST so we can confirm cursor is in the
    //    argument list (not on the callee) and count commas. The CST is the
    //    source of truth for trivia/comma layout — HIR drops them.
    let body_entity = semantic::body_entity_at(world, file_entity, offset)?;
    let cst = world.get::<kestrel_ast_builder::Valued>(body_entity)?;
    let pos = TextSize::from(offset as u32);
    let (_call_node, arg_list) = enclosing_call_at(&cst.0, pos)?;
    let active = active_param_index(&arg_list, pos);

    // 2. Match the CST call to its HirExpr by finding the smallest call-like
    //    HirExpr whose span contains the cursor offset. (CST text ranges
    //    include leading trivia and don't line up exactly with HIR spans, so
    //    we don't try to match end-to-end.)
    let ctx = world.query_context();
    let hir: HirBody = ctx.query(LowerBody { entity: body_entity, root })?;
    let expr_id = enclosing_call_hir(&hir, offset)?;

    // 3. Resolve candidate callee entities.
    let typed = ctx.query(InferBody { entity: body_entity, root });
    let candidates = resolve_callees(&hir, typed.as_ref(), expr_id)?;
    if candidates.is_empty() {
        return None;
    }

    // 4. Render each candidate. Skip ones we can't render (intrinsics
    //    without a Callable component, etc).
    let mut signatures: Vec<SignatureInformation> = Vec::new();
    let mut active_resolved: Option<Entity> =
        typed.as_ref().and_then(|t| t.resolutions.get(&expr_id).copied());
    let mut active_signature: Option<u32> = None;
    for e in candidates.iter() {
        if let Some(sig) = render_signature(world, sources, *e, active) {
            if active_resolved == Some(*e) && active_signature.is_none() {
                active_signature = Some(signatures.len() as u32);
                active_resolved = None;
            }
            signatures.push(sig);
        }
    }
    if signatures.is_empty() {
        return None;
    }
    let active_signature = active_signature.unwrap_or(0);

    Some(SignatureHelp {
        signatures,
        active_signature: Some(active_signature),
        active_parameter: Some(active),
    })
}

/// Smallest `ExprCall` whose `ArgumentList` text range contains `offset`.
/// We check the ArgumentList specifically (rather than the whole ExprCall)
/// so that cursor on the callee identifier doesn't trigger the popup.
fn enclosing_call_at(cst: &SyntaxNode, offset: TextSize) -> Option<(SyntaxNode, SyntaxNode)> {
    let mut best: Option<(SyntaxNode, SyntaxNode, u32)> = None;
    for node in cst.descendants() {
        if node.kind() != SyntaxKind::ExprCall {
            continue;
        }
        let Some(arg_list) = node.children().find(|c| c.kind() == SyntaxKind::ArgumentList)
        else {
            continue;
        };
        let range = arg_list.text_range();
        // Inclusive on both ends so cursor right after `(` or right before
        // `)` still triggers.
        if range.start() <= offset && offset <= range.end() {
            let len: u32 = range.len().into();
            if best.as_ref().map(|(_, _, l)| len < *l).unwrap_or(true) {
                best = Some((node, arg_list, len));
            }
        }
    }
    best.map(|(n, a, _)| (n, a))
}

/// Determine which argument the cursor is in. The parser models `,` as the
/// leading token of the *following* `Argument` node, so direct comma-counting
/// gives wrong answers when the comma lives inside `Argument(", 2")`. We
/// instead enumerate `Argument` children: the cursor falls inside one (→ that
/// argument's index) or sits between them (→ the next index).
fn active_param_index(arg_list: &SyntaxNode, offset: TextSize) -> u32 {
    let mut idx: u32 = 0;
    let mut count: u32 = 0;
    for child in arg_list.children() {
        if child.kind() != SyntaxKind::Argument {
            continue;
        }
        let range = child.text_range();
        if range.start() <= offset && offset <= range.end() {
            return count;
        }
        if range.end() < offset {
            idx = count + 1;
        }
        count += 1;
    }
    idx
}

/// Smallest `Call` / `MethodCall` / `ProtocolCall` HirExpr whose span
/// contains `offset`. Used to find the call enclosing the cursor.
fn enclosing_call_hir(body: &HirBody, offset: usize) -> Option<HirExprId> {
    let mut best: Option<(HirExprId, usize)> = None;
    for (id, expr) in body.exprs.iter() {
        if !matches!(
            expr,
            HirExpr::Call { .. } | HirExpr::MethodCall { .. } | HirExpr::ProtocolCall { .. }
        ) {
            continue;
        }
        let span = semantic::hir_expr_span(expr);
        if span.start <= offset && offset <= span.end {
            let len = span.end - span.start;
            if best.map(|(_, l)| len < l).unwrap_or(true) {
                best = Some((id, len));
            }
        }
    }
    best.map(|(id, _)| id)
}

fn resolve_callees(
    hir: &HirBody,
    typed: Option<&TypedBody>,
    expr_id: HirExprId,
) -> Option<Vec<Entity>> {
    match &hir.exprs[expr_id] {
        HirExpr::Call { callee, .. } => {
            match &hir.exprs[*callee] {
                HirExpr::Def(e, _, _) => Some(vec![*e]),
                HirExpr::OverloadSet { candidates, .. } => Some(candidates.clone()),
                _ => typed
                    .and_then(|t| t.resolutions.get(&expr_id).copied())
                    .map(|e| vec![e]),
            }
        },
        HirExpr::MethodCall { .. } | HirExpr::ProtocolCall { .. } => typed
            .and_then(|t| t.resolutions.get(&expr_id).copied())
            .map(|e| vec![e]),
        _ => None,
    }
}

/// Render a function/initializer/method entity as a `SignatureInformation`.
/// The label is built programmatically (rather than slicing source) so we
/// can return precise character offsets for each parameter.
fn render_signature(
    world: &World,
    sources: &HashMap<String, String>,
    entity: Entity,
    active_hint: u32,
) -> Option<SignatureInformation> {
    let callable = world.get::<Callable>(entity)?;
    let kind = world.get::<NodeKind>(entity);
    let raw_name = world.get::<Name>(entity).map(|n| n.0.clone());
    // Initializers carry `init` as their canonical name; for hover-style
    // display, prefer the parent type name (e.g. `Point(x:, y:)`).
    let display_name = match (kind, raw_name.as_deref()) {
        (Some(NodeKind::Initializer), _) => world
            .parent_of(entity)
            .and_then(|p| world.get::<Name>(p))
            .map(|n| n.0.clone())
            .or(raw_name),
        _ => raw_name,
    }
    .unwrap_or_else(|| "_".to_string());

    let file_path = entity_file_path(world, entity);
    let source = file_path.as_ref().and_then(|p| sources.get(p).map(|s| s.as_str()));

    let mut label = String::new();
    let mut params: Vec<ParameterInformation> = Vec::new();
    label.push_str(&display_name);
    label.push('(');
    for (i, p) in callable.params.iter().enumerate() {
        if i > 0 {
            label.push_str(", ");
        }
        let start = label.chars().count() as u32;
        // `_` label means "no external label". `label == name` means the
        // label is implicit. Other cases render `label name`.
        match p.label.as_deref() {
            Some(l) if l == p.name => label.push_str(&p.name),
            Some(l) if l == "_" => label.push_str(&p.name),
            Some(l) => {
                label.push_str(l);
                label.push(' ');
                label.push_str(&p.name);
            },
            None => label.push_str(&p.name),
        }
        if let Some(ty) = &p.ty {
            label.push_str(": ");
            label.push_str(&type_text(source, ty));
        }
        let end = label.chars().count() as u32;
        params.push(ParameterInformation {
            label: ParameterLabel::LabelOffsets([start, end]),
            documentation: None,
        });
    }
    label.push(')');
    if let Some(ret) = world.get::<TypeAnnotation>(entity) {
        // Initializers don't have a written return type; skip the arrow
        // unless the annotation is meaningful.
        if !matches!(kind, Some(NodeKind::Initializer)) {
            label.push_str(" -> ");
            label.push_str(&type_text(source, &ret.0));
        }
    }

    let active_param = if callable.params.is_empty() {
        None
    } else {
        Some(active_hint.min(callable.params.len() as u32 - 1))
    };

    Some(SignatureInformation {
        label,
        documentation: None,
        parameters: Some(params),
        active_parameter: active_param,
    })
}

/// Slice the source text covering `ty`'s span. Falls back to a placeholder
/// when the source isn't available (rare — only for entities without a
/// known file path, like intrinsics).
fn type_text(source: Option<&str>, ty: &AstType) -> String {
    let span = ast_type_span(ty);
    if let Some(src) = source {
        if let Some(slice) = src.get(span.start..span.end) {
            // Collapse any embedded newlines / runs of whitespace so the
            // signature stays a single line.
            return collapse_ws(slice);
        }
    }
    "_".to_string()
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_ws = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !last_ws {
                out.push(' ');
                last_ws = true;
            }
        } else {
            out.push(ch);
            last_ws = false;
        }
    }
    out.trim().to_string()
}

fn ast_type_span(ty: &AstType) -> Span {
    match ty {
        AstType::Named { span, .. } => span.clone(),
        AstType::Tuple(_, span) => span.clone(),
        AstType::Function { span, .. } => span.clone(),
        AstType::Array(_, span) => span.clone(),
        AstType::Dictionary(_, _, span) => span.clone(),
        AstType::Optional(_, span) => span.clone(),
        AstType::Result { span, .. } => span.clone(),
        AstType::Unit(span) => span.clone(),
        AstType::Never(span) => span.clone(),
        AstType::Inferred(span) => span.clone(),
    }
}

fn entity_file_path(world: &World, entity: Entity) -> Option<String> {
    use kestrel_ast_builder::{FileId, FilePath};
    if let Some(p) = world.get::<FilePath>(entity) {
        return Some(p.0.clone());
    }
    let fid = world.get::<FileId>(entity)?;
    world.get::<FilePath>(fid.0).map(|p| p.0.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;
    use std::collections::HashMap;

    fn at_offset(src: &str, offset: usize) -> Option<SignatureHelp> {
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/sig.ks", src.into());
        c.build(f);
        let mut sources = HashMap::new();
        sources.insert("/tmp/sig.ks".to_string(), src.to_string());
        super::signature_help_at(c.world(), &sources, f, offset, c.root())
    }

    #[test]
    fn first_param_active() {
        let src = "module T\n\
                   func add(a: lang.i64, b: lang.i64) -> lang.i64 { a }\n\
                   func main() { add(1, 2); }\n";
        // Cursor right after `(` in the call.
        let off = src.rfind("add(").unwrap() + "add(".len();
        let sh = at_offset(src, off).expect("signature help");
        assert_eq!(sh.signatures.len(), 1);
        assert_eq!(sh.active_parameter, Some(0), "{:?}", sh);
        assert!(
            sh.signatures[0].label.contains("add(a: lang.i64, b: lang.i64)"),
            "{}",
            sh.signatures[0].label
        );
    }

    #[test]
    fn after_comma_advances_active_param() {
        let src = "module T\n\
                   func add(a: lang.i64, b: lang.i64) -> lang.i64 { a }\n\
                   func main() { add(1, 2); }\n";
        // Cursor right after the comma + space.
        let off = src.find(", 2").unwrap() + ", ".len();
        let sh = at_offset(src, off).expect("signature help");
        assert_eq!(sh.active_parameter, Some(1), "{:?}", sh);
    }

    #[test]
    fn cursor_on_callee_returns_none() {
        let src = "module T\n\
                   func add(a: lang.i64, b: lang.i64) -> lang.i64 { a }\n\
                   func main() { add(1, 2); }\n";
        // Cursor on the `a` of `add` (the callee identifier).
        let off = src.find("add(1").unwrap() + 1;
        let sh = at_offset(src, off);
        assert!(sh.is_none(), "expected no signature on callee, got {sh:?}");
    }

    #[test]
    fn nested_call_picks_inner() {
        let src = "module T\n\
                   func inner(x: lang.i64) -> lang.i64 { x }\n\
                   func outer(y: lang.i64) -> lang.i64 { y }\n\
                   func main() { outer(inner(1)); }\n";
        // Cursor right after `inner(`.
        let off = src.rfind("inner(").unwrap() + "inner(".len();
        let sh = at_offset(src, off).expect("signature help");
        assert!(
            sh.signatures[0].label.starts_with("inner("),
            "{}",
            sh.signatures[0].label
        );
    }

    #[test]
    fn overload_set_lists_all_candidates() {
        // Two `add` overloads with different parameter shapes — both should
        // appear in `signatures`. (Overload resolution at the call site
        // picks the active one based on argument types; for this test we
        // only assert that both are surfaced.)
        let src = "module T\n\
                   func add(a: lang.i64, b: lang.i64) -> lang.i64 { a }\n\
                   func add(a: lang.f64, b: lang.f64) -> lang.f64 { a }\n\
                   func main() { add(1, 2); }\n";
        let off = src.rfind("add(").unwrap() + "add(".len();
        let sh = at_offset(src, off).expect("signature help");
        assert!(
            sh.signatures.len() >= 2,
            "expected ≥2 overloads, got {sh:?}"
        );
    }

    #[test]
    fn parameter_label_offsets_point_into_label() {
        let src = "module T\n\
                   func add(a: lang.i64, b: lang.i64) -> lang.i64 { a }\n\
                   func main() { add(1, 2); }\n";
        let off = src.rfind("add(").unwrap() + "add(".len();
        let sh = at_offset(src, off).expect("signature help");
        let sig = &sh.signatures[0];
        let params = sig.parameters.as_ref().expect("params");
        assert_eq!(params.len(), 2);
        for p in params {
            let ParameterLabel::LabelOffsets([s, e]) = p.label else {
                panic!("expected offset-shaped label");
            };
            let chars: Vec<char> = sig.label.chars().collect();
            let slice: String = chars[s as usize..e as usize].iter().collect();
            assert!(
                slice.contains(": lang.i64"),
                "param label {slice:?} not in signature {:?}",
                sig.label
            );
        }
    }
}
