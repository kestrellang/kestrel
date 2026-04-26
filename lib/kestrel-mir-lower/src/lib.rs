//! HIR → MIR lowering for kestrel-mir.
//!
//! Takes the typed ECS world after type inference and produces a `MirModule`
//! containing all declarations and function bodies.
//!
//! Currently handles:
//! - Item declarations: structs, enums, protocols, function signatures
//! - Type resolution: AstType → HirTy → MirTy, ResolvedTy → MirTy
//! - Function bodies: literals, locals, assignments, return, if/else,
//!   loops, break/continue, blocks, field access, tuple index
//!
//! Not yet: calls, closures, match, pattern matching, witnesses.

mod body_lower;
mod context;
mod enum_lower;
mod function_lower;
mod item;
mod name;
mod protocol_lower;
mod resolved_ty;
mod static_lower;
mod struct_lower;
pub mod ty;
mod validate;
mod witness_lower;

pub use context::LowerCtx;

use kestrel_hecs::{Entity, World};
use kestrel_mir::MirModule;

/// Lower the entire compiled program to MIR.
///
/// Takes the ECS world and root module entity directly (no compiler dependency).
/// Call after type inference has run.
pub fn lower_module(world: &World, root: Entity) -> MirModule {
    let mut ctx = LowerCtx::new(world, root, "main");
    item::lower_items(&mut ctx);
    witness_lower::lower_witnesses(&mut ctx);
    // Statics and the synthetic `__kestrel_init_statics` function exist only
    // after all items are known (we need per-static init thunks registered
    // before main-injection runs).
    static_lower::synthesize_static_inits(&mut ctx);
    // Validate that no `MirTy::Error` escaped into the built module. Any that
    // did signals an upstream bind/inference failure whose fallback leaked
    // through; surface it as a compiler error before codegen trips on it.
    let err_count = validate::validate_no_error_types(&ctx, &ctx.module);
    let mut module = ctx.finish();
    module.lowering_error_count = err_count;
    module
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler_driver::CompilerDriver;
    use kestrel_compiler::Compiler;
    use kestrel_mir::WitnessMethodKey;
    use std::path::PathBuf;

    fn stdlib_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../lang/std")
            .canonicalize()
            .expect("stdlib path should exist at lang/std")
    }

    /// Helper: set source, then build it.
    fn set_and_build(c: &mut Compiler, path: &str, source: &str) {
        let entity = c.set_source(path, source.into());
        c.build(entity);
    }

    #[test]
    fn lower_simple_struct() {
        let mut c = Compiler::new();
        set_and_build(
            &mut c,
            "test.ks",
            "module Test\nstruct Point { var x: Int64; var y: Int64 }",
        );

        let mir = lower_module(c.world(), c.root());
        let output = mir.display().to_string();

        // Should contain the struct (types are <error> without stdlib for Int64)
        assert!(
            output.contains("Point"),
            "MIR should contain struct Point:\n{}",
            output
        );
    }

    #[test]
    fn lower_simple_enum() {
        let mut c = Compiler::new();
        set_and_build(
            &mut c,
            "test.ks",
            "module Test\nenum Color { case Red\ncase Green\ncase Blue }",
        );

        let mir = lower_module(c.world(), c.root());
        let output = mir.display().to_string();

        assert!(
            output.contains("Color"),
            "MIR should contain enum Color:\n{}",
            output
        );
        assert!(
            output.contains("Red"),
            "MIR should contain case Red:\n{}",
            output
        );
    }

    #[test]
    fn lower_stdlib_smoke() {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);

        let mir = lower_module(c.world(), c.root());

        // Should have lowered many items
        assert!(!mir.structs.is_empty(), "should have lowered some structs");
        assert!(
            !mir.functions.is_empty(),
            "should have lowered some functions"
        );

        // Print summary
        eprintln!(
            "MIR lowering: {} structs, {} enums, {} protocols, {} functions",
            mir.structs.len(),
            mir.enums.len(),
            mir.protocols.len(),
            mir.functions.len(),
        );
    }

    #[test]
    fn stdlib_types_resolve() {
        // Load stdlib and verify that field types resolve to real types, not <error>
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);

        let mir = lower_module(c.world(), c.root());
        let output = mir.display().to_string();

        // Count <error> types vs total fields across all structs
        let error_count = mir
            .structs
            .iter()
            .flat_map(|s| &s.fields)
            .filter(|f| f.ty == kestrel_mir::MirTy::Error)
            .count();
        let total_fields = mir.structs.iter().flat_map(|s| &s.fields).count();

        eprintln!(
            "Type resolution: {}/{} fields resolved ({} errors)",
            total_fields - error_count,
            total_fields,
            error_count,
        );

        // Most stdlib fields should resolve
        assert!(
            error_count < total_fields / 2,
            "too many unresolved field types: {} errors out of {} total",
            error_count,
            total_fields,
        );

        // Spot-check: Int64 struct should have a field with lang.i64 type
        let has_raw_field = output.contains("raw: i64");
        eprintln!("Has raw: i64 field: {}", has_raw_field);
    }

    #[test]
    fn lower_function_body() {
        // Simple function body with if/else and return
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);
        set_and_build(
            &mut c,
            "test.ks",
            "module Test\nfunc abs(x: Int64) -> Int64 {\n  if x < 0 {\n    return -x\n  }\n  return x\n}",
        );

        // Run inference so TypedBody is available
        CompilerDriver::new(&c).infer_all();

        let mir = lower_module(c.world(), c.root());

        // Count functions with bodies
        let bodies = mir.functions.iter().filter(|f| f.body.is_some()).count();
        let total_blocks: usize = mir
            .functions
            .iter()
            .filter_map(|f| f.body.as_ref())
            .map(|b| b.blocks.len())
            .sum();

        eprintln!(
            "Body lowering: {}/{} functions have bodies, {} total blocks",
            bodies,
            mir.functions.len(),
            total_blocks,
        );

        // At least some stdlib functions should have bodies now
        assert!(bodies > 0, "no function bodies were lowered");
        assert!(
            total_blocks > bodies,
            "should have multiple blocks (from if/else)"
        );
    }

    #[test]
    fn lower_calls() {
        // Verify that call statements appear in the MIR dump
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);
        CompilerDriver::new(&c).infer_all();

        let mir = lower_module(c.world(), c.root());
        let output = mir.display().to_string();

        // Count "call " occurrences in the output
        let call_count = output.matches("call ").count();
        eprintln!("Call statements in MIR: {}", call_count);
        assert!(
            call_count > 100,
            "expected many call statements, got {}",
            call_count
        );

        // Should have witness_method calls from operator desugaring
        let witness_count = output.matches("witness_method").count();
        eprintln!("Witness method calls: {}", witness_count);

        // Should have apply_partial from closure lowering
        let partial_count = output.matches("apply partial").count();
        eprintln!("Apply partial (closures): {}", partial_count);
    }

    #[test]
    fn string_literals_decode_escapes_like_lib1() {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);
        set_and_build(
            &mut c,
            "test.ks",
            "module Test\nfunc banner() -> String {\n  \"\\x1b[31mhello\\n\"\n}",
        );
        CompilerDriver::new(&c).infer_all();

        let mir = lower_module(c.world(), c.root());
        let output = mir.display().to_string();

        assert!(
            output.contains("str.ptr \"\\u{1b}[31mhello\\n\""),
            "expected decoded escape sequences in MIR:\n{}",
            output
        );
    }

    #[test]
    fn lower_witnesses() {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);

        let mir = lower_module(c.world(), c.root());

        eprintln!("Witnesses: {}", mir.witnesses.len());

        // Should have generated witnesses for stdlib conformances
        assert!(!mir.witnesses.is_empty(), "should have generated witnesses");

        // Count method bindings across all witnesses
        let total_bindings: usize = mir.witnesses.iter().map(|w| w.method_bindings.len()).sum();
        eprintln!("Total method bindings: {}", total_bindings);

        // Print a few witness samples
        let output = mir.display().to_string();
        let witness_lines: Vec<&str> = output
            .lines()
            .filter(|l| l.starts_with("witness "))
            .take(10)
            .collect();
        for line in &witness_lines {
            eprintln!("  {}", line);
        }
    }

    #[test]
    fn witness_includes_protocol_extension_methods() {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);
        set_and_build(
            &mut c,
            "test.ks",
            r#"module Test

protocol Greeter {
    func greet() -> String
}

extend Greeter {
    func shout() -> String {
        return self.greet()
    }
}

struct Bob { }

extend Bob: Greeter {
    func greet() -> String { return "hi" }
}
"#,
        );
        CompilerDriver::new(&c).infer_all();

        let mir = lower_module(c.world(), c.root());

        // Find the witness for Bob: Greeter
        let bob_witness = mir
            .witnesses
            .iter()
            .find(|w| {
                let proto_name = mir.resolve_name(w.protocol);
                proto_name.contains("Greeter")
            })
            .expect("should have a witness for Bob: Greeter");

        assert!(
            bob_witness
                .method_bindings
                .contains_key(&WitnessMethodKey::bare("greet")),
            "witness should contain 'greet' (direct protocol method)"
        );
        assert!(
            bob_witness
                .method_bindings
                .contains_key(&WitnessMethodKey::bare("shout")),
            "witness should contain 'shout' (protocol extension method)"
        );
    }

    #[test]
    fn witness_keeps_overloaded_protocol_extension_methods() {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);
        set_and_build(
            &mut c,
            "test.ks",
            r#"module Test

protocol P { }

extend P {
    func value() -> Int64 { return 1 }
    func value(by x: Int64) -> Int64 { return x }
}

struct S { }

extend S: P { }
"#,
        );
        CompilerDriver::new(&c).infer_all();

        let mir = lower_module(c.world(), c.root());

        let witness = mir
            .witnesses
            .iter()
            .find(|w| {
                let protocol_name = mir.resolve_name(w.protocol);
                let implements_p = protocol_name == "P" || protocol_name.ends_with(".P");
                let implements_s = match &w.implementing_type {
                    kestrel_mir::MirTy::Named { entity, .. } => {
                        let type_name = mir.resolve_name(*entity);
                        type_name == "S" || type_name.ends_with(".S")
                    },
                    _ => false,
                };
                implements_p && implements_s
            })
            .expect("should have a witness for S: P");

        assert!(
            witness
                .method_bindings
                .contains_key(&WitnessMethodKey::bare("value")),
            "witness should contain value()"
        );
        assert!(
            witness.method_bindings.contains_key(&WitnessMethodKey::new(
                "value",
                vec![Some("by".to_string())],
            )),
            "witness should contain value(by:)"
        );
    }

    #[test]
    fn run_all_passes() {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);

        let mir = lower_module(c.world(), c.root()).with_all_passes();

        // Layout pass should have computed some struct layouts
        let layouts_computed = mir.structs.iter().filter(|s| s.layout.is_some()).count();
        eprintln!(
            "Layouts: {}/{} structs have computed layouts",
            layouts_computed,
            mir.structs.len(),
        );

        // Thunk pass should have generated thunk functions
        let thunk_count = mir
            .functions
            .iter()
            .filter(|f| matches!(f.kind, kestrel_mir::FunctionKind::Thunk { .. }))
            .count();
        eprintln!("Thunks: {}", thunk_count);

        // Deinit pass should have inserted deinit statements
        let deinit_count: usize = mir
            .functions
            .iter()
            .filter_map(|f| f.body.as_ref())
            .flat_map(|b| &b.blocks)
            .flat_map(|b| &b.stmts)
            .filter(|s| matches!(s.kind, kestrel_mir::StatementKind::Deinit { .. }))
            .count();
        eprintln!("Deinit statements: {}", deinit_count);

        // All passes should complete without panic
        assert!(
            layouts_computed > 0,
            "layout pass should compute some layouts"
        );
    }
}
