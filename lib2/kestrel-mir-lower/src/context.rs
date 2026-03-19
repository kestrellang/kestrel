//! Lowering context — central state during HIR → MIR lowering.

use kestrel_hecs::{Entity, QueryContext, World};
use kestrel_mir::MirModule;

use crate::name::qualified_name;

/// Central state maintained during lowering.
pub struct LowerCtx<'a> {
    /// The ECS world (read-only).
    pub world: &'a World,
    /// Query context for running resolution queries.
    pub query: QueryContext<'a>,
    /// Root module entity.
    pub root: Entity,
    /// The MIR module being built.
    pub module: MirModule,
}

impl<'a> LowerCtx<'a> {
    pub fn new(world: &'a World, root: Entity, name: &str) -> Self {
        let query = world.query_context();
        Self {
            world,
            query,
            root,
            module: MirModule::new(name),
        }
    }

    /// Register an entity's qualified name in the module's name map.
    /// Returns the name string.
    pub fn register_name(&mut self, entity: Entity) -> String {
        let name = qualified_name(self.world, entity);
        self.module.register_name(entity, name.clone());
        name
    }

    /// Consume the context and return the built MIR module.
    pub fn finish(self) -> MirModule {
        self.module
    }
}
