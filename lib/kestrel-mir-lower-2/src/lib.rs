//! HIR → MIR lowering for kestrel-mir-2.
//!
//! Consumes the typed ECS world (post–type-inference) and produces a
//! `MirModule` ready for the kestrel-mir-2 pass pipeline.

pub(crate) mod body;
mod context;
mod items;
mod name;
pub mod ty;
mod validate;

pub use context::LowerCtx;

use kestrel_hecs::{Entity, World};
use kestrel_mir_2::MirModule;

/// Lower the entire compiled program to MIR.
///
/// Takes the ECS world and root module entity. Call after type inference.
pub fn lower_module(world: &World, root: Entity) -> MirModule {
    let mut ctx = LowerCtx::new(world, root, "main");

    // Phase 1: item declarations (structs, enums, protocols, functions, statics)
    items::lower_items(&mut ctx);

    // Phase 2: witness tables (depends on structs/enums being present)
    items::witness_lower::lower_witnesses(&mut ctx);

    // Phase 3: static init thunks + master init + inject into main
    items::static_lower::synthesize_static_inits(&mut ctx);

    // Phase 4: validate no MirTy::Error escaped
    let _error_count = validate::validate_no_error_types(&ctx, &ctx.module);

    ctx.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;
    use kestrel_compiler_driver::CompilerDriver;

    fn stdlib_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../lang/std")
            .canonicalize()
            .expect("stdlib path should exist at lang/std")
    }

    #[test]
    fn lower_stdlib_items() {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);

        let mir = lower_module(c.world(), c.root());
        assert_eq!(mir.name, "main");
        assert!(
            !mir.structs.is_empty(),
            "should have lowered structs"
        );
        assert!(
            !mir.enums.is_empty(),
            "should have lowered enums"
        );
        assert!(
            !mir.protocols.is_empty(),
            "should have lowered protocols"
        );
        assert!(
            !mir.functions.is_empty(),
            "should have lowered functions"
        );
        assert!(
            !mir.witnesses.is_empty(),
            "should have lowered witnesses"
        );

        eprintln!(
            "MIR-2 lowering: {} structs, {} enums, {} protocols, {} functions, {} witnesses, {} statics",
            mir.structs.len(),
            mir.enums.len(),
            mir.protocols.len(),
            mir.functions.len(),
            mir.witnesses.len(),
            mir.statics.len(),
        );
    }

    #[test]
    fn stdlib_functions_have_bodies() {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);
        CompilerDriver::new(&c).infer_all();

        let mir = lower_module(c.world(), c.root());

        let with_bodies = mir.functions.iter().filter(|f| f.body.is_some()).count();
        let total = mir.functions.len();
        let total_blocks: usize = mir
            .functions
            .iter()
            .filter_map(|f| f.body.as_ref())
            .map(|b| b.blocks.len())
            .sum();

        eprintln!(
            "Body lowering: {}/{} functions have bodies, {} total blocks",
            with_bodies, total, total_blocks,
        );

        assert!(
            with_bodies > 100,
            "expected many functions with bodies, got {with_bodies}"
        );
        // Control flow (if/else, loops) produces multiple blocks per body
        assert!(
            total_blocks > with_bodies,
            "expected more blocks than functions (if/else/loop), got {total_blocks} blocks for {with_bodies} bodies"
        );

        // Count call statements
        let call_count: usize = mir
            .functions
            .iter()
            .filter_map(|f| f.body.as_ref())
            .flat_map(|b| &b.blocks)
            .flat_map(|b| &b.stmts)
            .filter(|s| matches!(s.kind, kestrel_mir_2::StatementKind::Call { .. }))
            .count();
        eprintln!("Call statements: {}", call_count);
        assert!(
            call_count > 100,
            "expected many call statements, got {call_count}"
        );

        // Check for degenerate bodies that would trip the pass pipeline
        let empty_bodies = mir
            .functions
            .iter()
            .filter(|f| {
                f.body
                    .as_ref()
                    .is_some_and(|b| b.locals.is_empty() || b.blocks.is_empty())
            })
            .count();
        eprintln!("Degenerate bodies (0 locals or 0 blocks): {}", empty_bodies);

        // Count Op assignments (from intrinsic lowering)
        let op_count: usize = mir
            .functions
            .iter()
            .filter_map(|f| f.body.as_ref())
            .flat_map(|b| &b.blocks)
            .flat_map(|b| &b.stmts)
            .filter(|s| matches!(
                s.kind,
                kestrel_mir_2::StatementKind::Assign {
                    rvalue: kestrel_mir_2::Rvalue::Op1 { .. }
                        | kestrel_mir_2::Rvalue::Op2 { .. }
                        | kestrel_mir_2::Rvalue::Op3 { .. },
                    ..
                }
            ))
            .count();
        eprintln!("Op statements (intrinsics): {}", op_count);
    }

    #[test]
    fn stdlib_passes_pipeline() {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);
        CompilerDriver::new(&c).infer_all();

        let mut mir = lower_module(c.world(), c.root());
        let target = kestrel_mir_2::TargetConfig::host_64();
        let mut next_entity = c.world().entity_count() as u32;
        let result =
            kestrel_mir_2::passes::run_pipeline(&mut mir, &target, &mut next_entity);

        let func_count = mir.functions.len();
        let thunk_count = mir
            .functions
            .iter()
            .filter(|f| {
                matches!(
                    f.kind,
                    kestrel_mir_2::item::function::FunctionKind::Thunk { .. }
                )
            })
            .count();
        let shim_count = mir
            .functions
            .iter()
            .filter(|f| {
                matches!(
                    f.kind,
                    kestrel_mir_2::item::function::FunctionKind::DropShim { .. }
                )
            })
            .count();

        eprintln!(
            "Pipeline: {} functions ({} thunks, {} drop shims), {} verify errors",
            func_count,
            thunk_count,
            shim_count,
            result.errors.len()
        );

        // Categorize verify errors by function
        let mut by_func: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
        for e in &result.errors {
            by_func.entry(&mir.functions[e.func_idx].name).or_default().push(&e.message);
        }
        let mut sorted: Vec<_> = by_func.into_iter().collect();
        sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        for (func, errors) in sorted.iter().take(15) {
            let first = errors[0];
            if errors.len() == 1 {
                eprintln!("  {func}: {first}");
            } else {
                eprintln!("  {func} ({}x): {first}", errors.len());
            }
        }

        // Dump a specific function for debugging
        for func in &mir.functions {
            if func.name == "std.iter.CycleIterator.init" || func.name == "std.collections.Array.init" {
                eprintln!("\n--- {} ---", func.name);
                if let Some(body) = &func.body {
                    for (i, local) in body.locals.iter().enumerate() {
                        let marker = if i < body.param_count { " [param]" } else { "" };
                        eprintln!("  %{} {}: {:?}{}", i, local.name, mir.ty_arena.get(local.ty), marker);
                    }
                    for (bi, block) in body.blocks.iter().enumerate() {
                        eprintln!("  bb{bi}:");
                        for (si, stmt) in block.stmts.iter().enumerate() {
                            eprintln!("    [{si}] {:?}", stmt.kind);
                        }
                        eprintln!("    term: {:?}", block.terminator.kind);
                    }
                }
            }
        }

        assert!(func_count > 3000, "expected 3000+ functions, got {func_count}");
        assert!(thunk_count > 100, "expected thunks, got {thunk_count}");
        assert!(shim_count > 5, "expected drop shims, got {shim_count}");
    }
}
