pub mod block;
pub mod body;
pub mod builder;
pub mod callee;
pub mod display;
pub mod id;
pub mod immediate;
pub mod inst;
pub mod item;
pub mod layout;
pub mod mono;
pub mod op;
pub mod passes;
pub mod substitute;
pub mod terminator;
pub mod ty;
pub mod ty_query;
pub mod value;
pub mod verify;

use indexmap::IndexMap;
use kestrel_hecs::Entity;

pub use body::OssaBody;
pub use id::{
    BlockId, FieldIdx, MonoFuncId, TyId,
    ValueId, VariantIdx,
};
pub use immediate::{Immediate, ImmediateKind};
pub use item::{CopyBehavior, DropBehavior, Layout, TargetConfig, TypeInfo, TypeParamDef};
pub use layout::{EnumLayout, StructLayout};
pub use op::{FloatBits, FloatMathKind, FloatPredicateKind, IntBits, Op, Signedness};
pub use substitute::{SubstMap, substitute};
pub use terminator::SwitchCase;
pub use ty::{MirTy, ParamConvention, TyArena};
pub use item::WitnessMethodKey;
pub use value::Ownership;

use item::enum_def::EnumDef;
use item::function::FunctionDef;
use item::protocol::ProtocolDef;
use item::static_def::StaticDef;
use item::struct_def::StructDef;
use item::witness::WitnessDef;

#[derive(Debug)]
pub struct MirModule {
    pub name: String,
    pub functions: IndexMap<Entity, FunctionDef>,
    pub structs: IndexMap<Entity, StructDef>,
    pub enums: IndexMap<Entity, EnumDef>,
    pub protocols: IndexMap<Entity, ProtocolDef>,
    pub witnesses: Vec<WitnessDef>,
    pub statics: IndexMap<Entity, StaticDef>,
    pub ty_arena: TyArena,
    pub entity_names: IndexMap<Entity, String>,
}

impl MirModule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            functions: IndexMap::new(),
            structs: IndexMap::new(),
            enums: IndexMap::new(),
            protocols: IndexMap::new(),
            witnesses: Vec::new(),
            statics: IndexMap::new(),
            ty_arena: TyArena::new(),
            entity_names: IndexMap::new(),
        }
    }

    pub fn register_name(&mut self, entity: Entity, name: impl Into<String>) {
        self.entity_names.insert(entity, name.into());
    }

    pub fn resolve_name(&self, entity: Entity) -> &str {
        self.entity_names
            .get(&entity)
            .map(|s| s.as_str())
            .unwrap_or("<unknown>")
    }

    pub fn add_function(&mut self, func: FunctionDef) -> Entity {
        let entity = func.entity;
        self.functions.insert(entity, func);
        entity
    }

    pub fn add_struct(&mut self, def: StructDef) -> Entity {
        let entity = def.entity;
        self.structs.insert(entity, def);
        entity
    }

    pub fn add_enum(&mut self, def: EnumDef) -> Entity {
        let entity = def.entity;
        self.enums.insert(entity, def);
        entity
    }

    pub fn add_protocol(&mut self, def: ProtocolDef) -> Entity {
        let entity = def.entity;
        self.protocols.insert(entity, def);
        entity
    }

    pub fn add_witness(&mut self, def: WitnessDef) {
        self.witnesses.push(def);
    }

    pub fn add_static(&mut self, def: StaticDef) -> Entity {
        let entity = def.entity;
        self.statics.insert(entity, def);
        entity
    }
}
