//! HIR → MIR lowering for kestrel-mir-2.
//!
//! Consumes the typed ECS world (post–type-inference) and produces a
//! `MirModule` ready for the kestrel-mir-2 pass pipeline.

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
}
