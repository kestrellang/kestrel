pub mod clone_elab;
pub mod dataflow;
pub mod drop_elab;
pub mod drop_shim;
pub mod init_state;
pub mod layout;
pub mod liveness;
pub mod verify;

use crate::item::TargetConfig;
use crate::MirModule;

/// Run the full generic MIR pipeline: clone elab → drop shim → drop elab → layout → verify.
/// Returns the verify result. The module is mutated in place.
pub fn run_pipeline(
    module: &mut MirModule,
    target: &TargetConfig,
    next_entity: &mut u32,
) -> verify::VerifyResult {
    clone_elab::run_clone_elaboration(module);
    drop_shim::synthesize_drop_shims(module, next_entity);
    drop_elab::run_drop_elaboration(module);
    layout::run_layout_pass(module, target);
    verify::verify(module)
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;
    use crate::builder::ModuleBuilder;
    use crate::immediate::Immediate;
    use crate::item::enum_def::{EnumCaseDef, EnumDef};
    use crate::item::protocol::ProtocolDef;
    use crate::item::struct_def::{FieldDef, StructDef};
    use crate::item::{CopyBehavior, DropBehavior, Layout, TypeInfo};
    use crate::operand::{Operand, UseMode};
    use crate::place::Place;
    use crate::statement::Rvalue;
    use crate::ty::ParamConvention;
    use crate::{FieldIdx, VariantIdx};

    fn target() -> TargetConfig {
        TargetConfig::host_64()
    }

    fn get_body<'a>(module: &'a MirModule, idx: crate::FunctionIdx) -> &'a crate::MirBody {
        module.functions[idx.index()].body.as_ref().unwrap()
    }

    fn setup_cloneable(m: &mut ModuleBuilder) -> kestrel_hecs::Entity {
        let cloneable = m.fresh_entity();
        m.register_name(cloneable, "std.Cloneable");
        m.add_protocol(ProtocolDef::new(cloneable, "std.Cloneable"));
        cloneable
    }

    fn add_clone_droppable_struct(
        m: &mut ModuleBuilder,
        name: &str,
        cloneable: kestrel_hecs::Entity,
    ) -> (kestrel_hecs::Entity, crate::TyId) {
        let i64_ty = m.i64();
        let entity = m.fresh_entity();
        m.register_name(entity, name);
        let ty = m.named(entity, vec![]);
        let mut def = StructDef::new(entity, name);
        def.add_field(FieldDef::new("data", i64_ty));
        def.type_info = TypeInfo {
            copy: CopyBehavior::Clone(cloneable),
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![],
            },
            layout: None,
        };
        m.add_struct(def);
        (entity, ty)
    }

    fn add_droppable_struct(
        m: &mut ModuleBuilder,
        name: &str,
    ) -> (kestrel_hecs::Entity, crate::TyId) {
        let i64_ty = m.i64();
        let entity = m.fresh_entity();
        m.register_name(entity, name);
        let ty = m.named(entity, vec![]);
        let mut def = StructDef::new(entity, name);
        def.add_field(FieldDef::new("data", i64_ty));
        def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![],
            },
            layout: None,
        };
        m.add_struct(def);
        (entity, ty)
    }

    // ---- 1. Clone-typed locals through full pipeline ----

    #[test]
    fn clone_local_full_pipeline() {
        // x is Clone-typed, copied twice then discarded.
        // Clone elab inserts a clone for the live copy.
        // Drop elab inserts drops for owned values at return.
        // Verify checks everything is clean.
        let mut m = ModuleBuilder::new("test");
        let cloneable = setup_cloneable(&mut m);
        let (_, s_ty) = add_clone_droppable_struct(&mut m, "MyStr", cloneable);
        let unit_ty = m.unit();

        let mut f = m.function("f", unit_ty);
        let x = f.local("x", s_ty);
        let y = f.local("y", s_ty);
        let z = f.local("z", s_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            // y = copy x (x live after — needs clone)
            b.assign(
                Place::local(y),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            // z = copy x (x dead after — last use, just move)
            b.assign(
                Place::local(z),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            b.ret_unit();
        }

        let mut module = m.finish();
        let mut next_entity = 100;
        let result = run_pipeline(&mut module, &target(), &mut next_entity);
        assert!(result.is_ok(), "pipeline errors: {:?}", result.errors);
    }

    // ---- 2. Mixed bitwise + droppable locals ----

    #[test]
    fn mixed_bitwise_and_droppable() {
        let mut m = ModuleBuilder::new("test");
        let (_, d_ty) = add_droppable_struct(&mut m, "Handle");
        let i64_ty = m.i64();
        let unit_ty = m.unit();

        let mut f = m.function("f", unit_ty);
        let fi = f.index();
        let x = f.local("x", d_ty);    // droppable
        let n = f.local("n", i64_ty);   // bitwise
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.assign_const(Place::local(n), Immediate::i64(42));
            b.ret_unit();
        }

        let mut module = m.finish();
        let mut next_entity = 100;
        let result = run_pipeline(&mut module, &target(), &mut next_entity);
        assert!(result.is_ok(), "pipeline errors: {:?}", result.errors);

        // x should be dropped, n should not
        let body = get_body(&module, fi);
        let has_drop_x = body.blocks[0].stmts.iter().any(|s| {
            matches!(&s.kind, crate::StatementKind::Drop { place } if place.root_local() == Some(x))
        });
        assert!(has_drop_x, "droppable local x should have a Drop");
    }

    // ---- 3. Diamond CFG with Maybe locals ----

    #[test]
    fn diamond_maybe_local_pipeline() {
        let mut m = ModuleBuilder::new("test");
        let (_, d_ty) = add_droppable_struct(&mut m, "Res");
        let i64_ty = m.i64();
        let unit_ty = m.unit();
        let bool_ty = m.bool();

        let mut f = m.function("f", unit_ty);
        let fi = f.index();
        let x = f.local("x", d_ty);
        let y = f.local("y", d_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        let bb3 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            let mut b = f.block_at(bb1);
            b.use_move(Place::local(y), Place::local(x));
            b.jump(bb3);
        }
        {
            f.block_at(bb2).jump(bb3);
        }
        {
            f.block_at(bb3).ret_unit();
        }

        let mut module = m.finish();
        let mut next_entity = 100;
        let result = run_pipeline(&mut module, &target(), &mut next_entity);
        assert!(result.is_ok(), "pipeline errors: {:?}", result.errors);

        let body = get_body(&module, fi);
        let has_drop_if = body.blocks.iter().any(|bb| {
            bb.stmts
                .iter()
                .any(|s| matches!(s.kind, crate::StatementKind::DropIf { .. }))
        });
        assert!(has_drop_if, "Maybe locals should have DropIf");
    }

    // ---- 4. Return droppable local (moved to caller, not dropped) ----

    #[test]
    fn return_droppable_not_dropped() {
        let mut m = ModuleBuilder::new("test");
        let (_, d_ty) = add_droppable_struct(&mut m, "Owned");

        let mut f = m.function("f", d_ty);
        let fi = f.index();
        let x = f.local("x", d_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.ret(Operand::Place(Place::local(x)));
        }

        let mut module = m.finish();
        let mut next_entity = 100;
        let result = run_pipeline(&mut module, &target(), &mut next_entity);
        assert!(result.is_ok(), "pipeline errors: {:?}", result.errors);

        let body = get_body(&module, fi);
        let has_drop_x = body.blocks[0].stmts.iter().any(|s| {
            matches!(&s.kind, crate::StatementKind::Drop { place } if place.root_local() == Some(x))
        });
        assert!(!has_drop_x, "returned local should not be dropped");
    }

    // ---- 5. Nested droppable fields (shim transitivity + layout) ----

    #[test]
    fn nested_droppable_with_layout() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();

        // Inner: droppable
        let inner_entity = m.fresh_entity();
        m.register_name(inner_entity, "Inner");
        let inner_ty = m.named(inner_entity, vec![]);
        let mut inner_def = StructDef::new(inner_entity, "Inner");
        inner_def.add_field(FieldDef::new("val", i64_ty));
        inner_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![],
            },
            layout: None,
        };
        m.add_struct(inner_def);

        // Outer: has Inner field, also droppable
        let outer_entity = m.fresh_entity();
        m.register_name(outer_entity, "Outer");
        let outer_ty = m.named(outer_entity, vec![]);
        let mut outer_def = StructDef::new(outer_entity, "Outer");
        outer_def.add_field(FieldDef::new("inner", inner_ty));
        outer_def.add_field(FieldDef::new("count", i64_ty));
        outer_def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![FieldIdx::new(0)],
            },
            layout: None,
        };
        m.add_struct(outer_def);

        let unit_ty = m.unit();
        let mut f = m.function("f", unit_ty);
        let x = f.local("x", outer_ty);
        let bb0 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.ret_unit();
        }

        let mut module = m.finish();
        let mut next_entity = 100;
        let result = run_pipeline(&mut module, &target(), &mut next_entity);
        assert!(result.is_ok(), "pipeline errors: {:?}", result.errors);

        // Both types should have layouts computed
        assert!(module.structs[0].type_info.layout.is_some(), "Inner should have layout");
        assert!(module.structs[1].type_info.layout.is_some(), "Outer should have layout");

        // Drop shims should exist for both
        assert!(
            module.functions.iter().any(|f| f.name.contains("__drop$Inner")),
            "Inner should have drop shim"
        );
        assert!(
            module.functions.iter().any(|f| f.name.contains("__drop$Outer")),
            "Outer should have drop shim"
        );
    }

    // ---- 6. Clone + diamond + return (combines everything) ----

    #[test]
    fn full_integration() {
        let mut m = ModuleBuilder::new("test");
        let cloneable = setup_cloneable(&mut m);
        let (_, s_ty) = add_clone_droppable_struct(&mut m, "Str", cloneable);
        let bool_ty = m.bool();

        let mut f = m.function("f", s_ty);
        let x = f.local("x", s_ty);
        let y = f.local("y", s_ty);
        let cond = f.local("cond", bool_ty);
        let bb0 = f.block_id();
        let bb1 = f.block_id();
        let bb2 = f.block_id();
        {
            let mut b = f.block_at(bb0);
            b.assign_const(Place::local(x), Immediate::i64(0));
            b.assign_const(Place::local(cond), Immediate::bool(true));
            b.branch(Operand::Place(Place::local(cond)), bb1, bb2);
        }
        {
            let mut b = f.block_at(bb1);
            // Clone x into y, return y (x still live → clone needed)
            b.assign(
                Place::local(y),
                Rvalue::Use(Operand::Place(Place::local(x)), UseMode::Copy),
            );
            b.ret(Operand::Place(Place::local(y)));
        }
        {
            // Return x directly
            f.block_at(bb2).ret(Operand::Place(Place::local(x)));
        }

        let mut module = m.finish();
        let mut next_entity = 100;
        let result = run_pipeline(&mut module, &target(), &mut next_entity);
        assert!(result.is_ok(), "pipeline errors: {:?}", result.errors);
    }
}
