//! HIR → MIR lowering for kestrel-mir-2.
//!
//! Consumes the typed ECS world (post–type-inference) and produces a
//! `MirModule` ready for the kestrel-mir-2 pass pipeline.

pub(crate) mod body;
mod context;
mod items;
mod name;
pub mod ty;

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
}
