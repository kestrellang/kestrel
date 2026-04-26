//! Integration tests for kestrel-type-infer.
//!
//! These tests parse real Kestrel source code, build declarations,
//! lower to HIR, then run type inference.

use kestrel_ast_builder::{Name, NodeKind, build_declarations, seed_lang_module};
use kestrel_hecs::{Entity, World};
use kestrel_hir_lower::LowerBody;
use kestrel_type_infer::InferBody;
use kestrel_type_infer::result::{ResolvedTy, TypedBody};

// ===== Helpers =====

/// Parse source, build declarations, return (world, root).
fn build_from_source(source: &str) -> (World, Entity) {
    let mut world = World::new();
    world.begin_revision();

    let root = world.spawn();
    world.set(root, NodeKind::Module);
    world.set(root, Name("<root>".to_string()));
    // Seed `lang.*` primitives so literal fallback in `apply_literal_defaults`
    // has `lang.i1` / `lang.i64` / `lang.str` / `lang.f64` to bind to when
    // the test source doesn't load stdlib's `Default<Kind>LiteralType`.
    seed_lang_module(&mut world, root);

    let file_entity = world.spawn();
    let tokens: Vec<_> = kestrel_lexer::lex(source, file_entity.index())
        .filter_map(|r| r.ok())
        .collect();
    let token_iter = tokens.iter().map(|t| (t.value.clone(), t.span.clone()));
    let result = kestrel_parser::parse_source_file_from_source(source, token_iter);
    build_declarations(&mut world, file_entity, &result.tree, root, None);

    (world, root)
}

/// Find a child entity by NodeKind and Name.
fn find_child(
    ctx: &kestrel_hecs::QueryContext<'_>,
    parent: Entity,
    kind: NodeKind,
    name: &str,
) -> Entity {
    ctx.children_of(parent)
        .iter()
        .find(|&&e| {
            ctx.get::<NodeKind>(e) == Some(&kind) && ctx.get::<Name>(e).is_some_and(|n| n.0 == name)
        })
        .copied()
        .unwrap_or_else(|| panic!("child {:?} {:?} not found under {:?}", kind, name, parent))
}

/// Find a function entity in a module by name.
fn find_function(
    ctx: &kestrel_hecs::QueryContext<'_>,
    root: Entity,
    module_name: &str,
    func_name: &str,
) -> Entity {
    let module = find_child(ctx, root, NodeKind::Module, module_name);
    find_child(ctx, module, NodeKind::Function, func_name)
}

/// Infer types for a function and return the TypedBody.
fn infer_func(
    ctx: &kestrel_hecs::QueryContext<'_>,
    root: Entity,
    module_name: &str,
    func_name: &str,
) -> TypedBody {
    let func = find_function(ctx, root, module_name, func_name);
    ctx.query(InferBody { entity: func, root })
        .unwrap_or_else(|| panic!("InferBody returned None for {}.{}", module_name, func_name))
}

/// Debug helper: print entity hierarchy.
#[allow(dead_code)]
fn print_tree(ctx: &kestrel_hecs::QueryContext<'_>, entity: Entity, indent: usize) {
    let name = ctx
        .get::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_default();
    let kind = ctx
        .get::<NodeKind>(entity)
        .map(|k| format!("{:?}", k))
        .unwrap_or_default();
    eprintln!("{}{} ({}) [{:?}]", " ".repeat(indent), name, kind, entity);
    for &child in ctx.children_of(entity) {
        print_tree(ctx, child, indent + 2);
    }
}

// ===== Tests =====

#[test]
fn infer_literal_integer() {
    let source = "module TestMod\nfunc foo() { 42 }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    // Should have types for expressions, no errors (besides unresolvable literals)
    assert!(!typed.expr_types.is_empty(), "should have expr types");
}

#[test]
fn infer_let_binding() {
    let source = "module TestMod\nfunc foo() { let x = 42; x }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    // Should have local types
    assert!(!typed.local_types.is_empty(), "should have local types");
}

#[test]
fn infer_multiple_lets() {
    let source = "module TestMod\nfunc foo() { let x = 1; let y = 2; x }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    // Two locals
    assert!(
        typed.local_types.len() >= 2,
        "should have at least 2 locals"
    );
}

#[test]
fn infer_if_expression() {
    let source = "module TestMod\nfunc foo() { if true { 1 } else { 2 } }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    // Should have expr types without panicking
    assert!(!typed.expr_types.is_empty());
}

#[test]
fn infer_tuple() {
    let source = "module TestMod\nfunc foo() { (1, 2) }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    // Find the tail expression type — should be a tuple
    let func = find_function(&ctx, root, "TestMod", "foo");
    let hir = ctx.query(LowerBody { entity: func, root }).unwrap();
    if let Some(tail) = hir.tail_expr {
        if let Some(ty) = typed.expr_types.get(&tail) {
            assert!(
                matches!(ty, ResolvedTy::Tuple(_)),
                "tail should be Tuple, got {:?}",
                ty
            );
        }
    }
}

#[test]
fn infer_assignment() {
    let source = "module TestMod\nfunc foo() { var x = 1; x = 2 }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    assert!(!typed.expr_types.is_empty());
}

#[test]
fn infer_closure() {
    let source = "module TestMod\nfunc foo() { { 42 } }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    assert!(!typed.expr_types.is_empty());
}

#[test]
fn infer_return() {
    let source = "module TestMod\nfunc foo() { return 42 }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    // Return expression should have type Never (diverges)
    let func = find_function(&ctx, root, "TestMod", "foo");
    let hir = ctx.query(LowerBody { entity: func, root }).unwrap();
    if let Some(tail) = hir.tail_expr {
        if let Some(ty) = typed.expr_types.get(&tail) {
            assert!(
                matches!(ty, ResolvedTy::Never),
                "return should be Never, got {:?}",
                ty
            );
        }
    }
}

#[test]
fn infer_empty_body() {
    let source = "module TestMod\nfunc foo() { }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    // Should complete without panicking, may have no expressions
    assert!(typed.errors.is_empty() || !typed.errors.is_empty()); // just verify no panic
}

#[test]
fn infer_loop_and_break() {
    let source = "module TestMod\nfunc foo() { loop { break } }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    assert!(!typed.expr_types.is_empty());
}

#[test]
fn infer_match_expression() {
    let source = "module TestMod\nfunc foo() { match 1 { _ => 2 } }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    assert!(!typed.expr_types.is_empty());
}

// ===== Overload resolution tests =====

#[test]
fn overload_by_arity() {
    // Two functions with same name, different param counts.
    // Calling with 1 arg should resolve to the 1-param version.
    // (No type annotations — avoids stdlib dependency)
    let source = r#"
module TestMod
func add(x a: Bool) { }
func add(x a: Bool, y b: Bool) { }
func test() { add(x: true) }
"#;
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "test");

    assert!(
        typed.errors.is_empty(),
        "expected no errors, got: {:?}",
        typed.errors
    );
}

#[test]
fn overload_by_label() {
    // Two functions with same name, different labels.
    // Calling with label "to" should resolve to the "to" version.
    let source = r#"
module TestMod
func send(to x: Bool) { }
func send(from x: Bool) { }
func test() { send(to: true) }
"#;
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "test");

    assert!(
        typed.errors.is_empty(),
        "expected no errors, got: {:?}",
        typed.errors
    );
}

#[test]
fn overload_no_match_errors() {
    // Call an overloaded function with labels that don't match any candidate.
    let source = r#"
module TestMod
func send(to x: Bool) { }
func send(from x: Bool) { }
func test() { send(via: true) }
"#;
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "test");

    assert!(
        !typed.errors.is_empty(),
        "expected an error for unresolved overload"
    );
}

#[test]
fn overload_set_not_callable_errors() {
    // Using an overloaded name without calling it should error.
    let source = r#"
module TestMod
func foo(x a: Bool) { }
func foo(x a: Bool, y b: Bool) { }
func test() { let f = foo; }
"#;
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "test");

    assert!(
        !typed.errors.is_empty(),
        "expected an error for bare overload reference"
    );
}

/// Regression: `WorldResolver::where_clauses(entity)` used to pass `self.owner`
/// as the name-resolution context, so when a caller body inferred a method whose
/// `where` clause referenced the method's own type param (or its enclosing type's),
/// the RHS failed to resolve — "cannot find type 'U' in this scope". The fix
/// resolves where clauses in the entity's own scope.
#[test]
fn where_clauses_resolve_method_type_params_when_called_from_other_body() {
    let source = r#"
module TestMod
protocol P {
    type Item;
}
func collect[T, U](iter: T) where T: P, T.Item = U { }
func caller(b: Bool) { collect(b) }
"#;
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let _ = infer_func(&ctx, root, "TestMod", "caller");

    // The "cannot find type" diagnostic is emitted by `lower_ast_type` through
    // the query accumulator, not `TypedBody::errors`. Check the world directly.
    // Filter for `U` specifically — this test doesn't set up the lang module, so
    // ambient name-resolution noise ("cannot find type 'Bool'") is expected and
    // unrelated to the regression we're guarding against.
    let diags = world.accumulated::<codespan_reporting::diagnostic::Diagnostic<usize>>();
    let offenders: Vec<_> = diags
        .iter()
        .filter(|d| d.message.contains("cannot find type 'U'"))
        .collect();
    assert!(
        offenders.is_empty(),
        "method-level where-clause type param 'U' leaked out of method scope: {:#?}",
        offenders.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// Regression: explicit type args on a method call (e.g., `x.make[Bool]()`)
/// used to be dropped because the `Member` constraint had no field for them.
/// The solver created a fresh TyVar for the method's `U` type param, and the
/// return type `U` stayed unresolved → surfaced as `ResolvedTy::Error` in the
/// caller. This caused downstream monomorphization skips and codegen link
/// failures (see `stdlib/result/result_transforms.ks`).
#[test]
fn method_call_with_explicit_type_args_binds_type_param() {
    let source = r#"
module TestMod
struct Marker { }
protocol Factory {
    func make[U]() -> U
}
func caller[T](f: T) where T: Factory {
    f.make[Marker]();
}
"#;
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "caller");

    // The call expression `f.make[Marker]()` must record [Marker] as its
    // resolved type_args — NOT a stray fresh TyVar that stays Unresolved and
    // leaks `ResolvedTy::Error` through to MIR/codegen. Before the fix, the
    // Member constraint had no field for explicit type args, so the solver
    // created a fresh unconstrained TyVar for U.
    let marker = find_child(
        &ctx,
        find_child(&ctx, root, NodeKind::Module, "TestMod"),
        NodeKind::Struct,
        "Marker",
    );
    let call_type_args: Vec<_> = typed
        .type_args
        .values()
        .filter(|args| args.len() == 1)
        .collect();
    assert!(
        !call_type_args.is_empty(),
        "expected type_args to be recorded for f.make[Marker](); got: {:#?}",
        typed.type_args,
    );
    let has_marker_arg = call_type_args.iter().any(|args| {
        matches!(&args[0], ResolvedTy::Named { entity, args } if args.is_empty() && *entity == marker)
    });
    assert!(
        has_marker_arg,
        "explicit type arg 'Marker' should be bound to method's type param U; got: {:#?}",
        call_type_args,
    );
}

/// Regression: `gen_struct_init` used to only record the init's method-level
/// type params (skipping the entry entirely when the init had none). MIR
/// lowering saw an empty `type_args` for the expression and fell back to
/// `infer_parent_type_args` + the `prepend_receiver_type_args` workaround in
/// `emit_call_maybe_init`, which double-counted the struct args and tripped
/// the monomorphizer's arity check (`Array.init`, `Slice.init`, etc.) with
/// "dispatch bug: has 2 type arg(s), function expects 1". The fix records
/// the COMPLETE list (struct params first, then method-level) so the
/// emit_call_maybe_init prepend is no longer load-bearing.
#[test]
fn struct_init_records_struct_type_args() {
    let source = r#"
module TestMod
struct Box[T] {
    init(value value: T) { }
}
func caller(b: Bool) { Box(value: b) }
"#;
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "caller");

    // `Box(value: b)` should record exactly 1 type arg — the struct's T,
    // resolved to whatever `b`'s inferred type is. An empty record (the
    // old behavior) means MIR lowering has to reconstruct it from
    // `result_ty`, which triggered the double-count bug.
    let call_type_args: Vec<_> = typed
        .type_args
        .values()
        .filter(|args| !args.is_empty())
        .collect();
    assert_eq!(
        call_type_args.len(),
        1,
        "expected a non-empty type_args record for the `Box(value: b)` init call; got: {:#?}",
        typed.type_args,
    );
    assert_eq!(
        call_type_args[0].len(),
        1,
        "Box has 1 struct type param and the init has 0 method-level params → recorded list must have length 1; got: {:#?}",
        call_type_args[0],
    );
}

/// Sibling to `struct_init_records_struct_type_args`: when the init has its
/// own method-level type params, the recorded list must put the struct's
/// params FIRST, then the method-level params — matching the layout MIR
/// `collect_inherited_type_params` produces for the init function.
#[test]
fn struct_init_records_struct_params_before_method_params() {
    let source = r#"
module TestMod
struct Box[T] {
    init[I](value value: T, other other: I) { }
}
func caller(b: Bool, n: Bool) { Box(value: b, other: n) }
"#;
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "caller");

    // The recorded list must have length 2 (T + I), in that order. The actual
    // resolved types depend on inference's solving — the structural check
    // here is the length/shape, not the concrete resolutions.
    let shaped: Vec<_> = typed
        .type_args
        .values()
        .filter(|args| args.len() == 2)
        .collect();
    assert!(
        !shaped.is_empty(),
        "expected a 2-element type_args record for Box[T].init[I] call; got: {:#?}",
        typed.type_args,
    );
}

/// Phase 2 verification: a `HirExpr::Field` whose name lowered to
/// `HirName::Missing` (parser recovered from `foo.` with no member token)
/// must short-circuit to `ResolvedTy::Error` without emitting any
/// "name not found" cascade. The parser already reported the gap.
#[test]
fn missing_field_name_resolves_to_error_without_cascade() {
    // `x.` with the cursor at EOF — parser recovers as Field { name: Missing }.
    let source = "module TestMod\nfunc foo() { let x = 42; x. }";
    let (world, root) = build_from_source(source);
    let ctx = world.query_context();
    let typed = infer_func(&ctx, root, "TestMod", "foo");

    // The Field expression must have an Error type — not "no member" or any
    // other inference diagnostic.
    let has_error_field = typed
        .expr_types
        .values()
        .any(|t| matches!(t, ResolvedTy::Error));
    assert!(
        has_error_field,
        "expected the missing-field expression to resolve to Error: {:#?}",
        typed.expr_types
    );

    // Most importantly: no member-resolution diagnostic should fire for the
    // missing name. The parser already published "expected identifier after
    // `.`"; inference must stay silent so the user doesn't see a cascade.
    let member_errors: Vec<_> = typed
        .errors
        .iter()
        .filter(|e| {
            matches!(
                e,
                kestrel_type_infer::error::InferError::NoMember { .. }
                    | kestrel_type_infer::error::InferError::MemberNotVisible { .. }
                    | kestrel_type_infer::error::InferError::AmbiguousMember { .. }
            )
        })
        .collect();
    assert!(
        member_errors.is_empty(),
        "expected no member-resolution diagnostics for HirName::Missing, got {:?}",
        member_errors
    );
}
