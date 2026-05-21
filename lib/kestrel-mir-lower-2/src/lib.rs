//! HIR → MIR lowering for kestrel-mir-2.
//!
//! Consumes the typed ECS world (post–type-inference) and produces a
//! `MirModule` ready for the kestrel-mir-2 pass pipeline.

mod context;
mod name;
pub mod ty;

pub use context::LowerCtx;

use kestrel_hecs::{Entity, World};
use kestrel_mir_2::MirModule;

/// Lower the entire compiled program to MIR.
///
/// Takes the ECS world and root module entity. Call after type inference.
pub fn lower_module(world: &World, root: Entity) -> MirModule {
    let ctx = LowerCtx::new(world, root, "main");
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
    fn lower_module_returns_empty_for_now() {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);

        let mir = lower_module(c.world(), c.root());
        assert_eq!(mir.name, "main");
        // No items lowered yet — just verifying the scaffold works
        assert!(mir.functions.is_empty());
    }
}
