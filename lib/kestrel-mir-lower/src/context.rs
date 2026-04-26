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
    /// Global counter for generating unique synthetic entities (closures, thunks).
    /// Uses a counter separate from per-function temp counters to avoid collisions.
    pub synthetic_entity_counter: u32,
    /// Global closure counter for unique naming across all functions.
    pub closure_counter: u32,
}

impl<'a> LowerCtx<'a> {
    pub fn new(world: &'a World, root: Entity, name: &str) -> Self {
        let query = world.query_context();
        Self {
            world,
            query,
            root,
            module: MirModule::new(name),
            synthetic_entity_counter: 0,
            closure_counter: 0,
        }
    }

    /// Register an entity's qualified name in the module's name map.
    /// Returns the name string.
    pub fn register_name(&mut self, entity: Entity) -> String {
        let name = qualified_name(self.world, entity);
        self.module.register_name(entity, name.clone());
        name
    }

    /// Generate a unique synthetic entity for closures, thunks, etc.
    /// Uses the high end of the u32 range to avoid collisions with real entities.
    pub fn next_synthetic_entity(&mut self) -> Entity {
        let id = self.synthetic_entity_counter;
        self.synthetic_entity_counter += 1;
        Entity::from_raw(u32::MAX / 2 - id)
    }

    /// Consume the context and return the built MIR module.
    pub fn finish(self) -> MirModule {
        self.module
    }
}
