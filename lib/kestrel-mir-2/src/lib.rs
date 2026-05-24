pub mod body;
pub mod builder;
pub mod display;
pub mod id;
pub mod immediate;
pub mod item;
pub mod layout;
pub mod mono;
pub mod op;
pub mod operand;
pub mod passes;
pub mod place;
pub mod place_ty;
pub mod statement;
pub mod substitute;
pub mod terminator;
pub mod ty;
pub mod ty_query;

use indexmap::IndexMap;
use kestrel_hecs::Entity;

pub use body::{BasicBlock, LocalDef, MirBody, ScopeId};
pub use id::{
    BlockId, EnumIdx, FieldIdx, FunctionIdx, LocalId, MonoFuncId, ProtocolIdx, StaticIdx,
    StructIdx, TyId, VariantIdx, WitnessIdx,
};
pub use immediate::{Immediate, ImmediateKind};
pub use item::{CopyBehavior, DropBehavior, Layout, TargetConfig, TypeInfo, TypeParamDef};
pub use layout::{EnumLayout, StructLayout};
pub use op::{FloatBits, FloatMathKind, FloatPredicateKind, IntBits, Op, Signedness};
pub use operand::{ArgMode, Operand, UseMode};
pub use place::{Place, PlaceBase, PlaceElem};
pub use statement::{Callee, Rvalue, Statement, StatementKind, WitnessMethodKey};
pub use substitute::{SubstMap, substitute};
pub use terminator::{SwitchCase, Terminator, TerminatorKind};
pub use ty::{MirTy, ParamConvention, TyArena};

pub use mono::{
    InstantiationKey, MonoEnum, MonoEnumCase, MonoField, MonoFunction, MonoModule, MonoParam,
    MonoStatic, MonoStruct,
};

use item::enum_def::EnumDef;
use item::function::FunctionDef;
use item::protocol::ProtocolDef;
use item::static_def::StaticDef;
use item::struct_def::StructDef;
use item::witness::WitnessDef;

#[derive(Debug)]
pub struct MirModule {
    pub name: String,
    pub functions: Vec<FunctionDef>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub protocols: Vec<ProtocolDef>,
    pub witnesses: Vec<WitnessDef>,
    pub statics: Vec<StaticDef>,
    pub ty_arena: TyArena,
    pub entity_names: IndexMap<Entity, String>,
}

impl MirModule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            functions: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
            protocols: Vec::new(),
            witnesses: Vec::new(),
            statics: Vec::new(),
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

    pub fn add_function(&mut self, func: FunctionDef) -> FunctionIdx {
        let idx = FunctionIdx::new(self.functions.len());
        self.functions.push(func);
        idx
    }

    pub fn add_struct(&mut self, def: StructDef) -> StructIdx {
        let idx = StructIdx::new(self.structs.len());
        self.structs.push(def);
        idx
    }

    pub fn add_enum(&mut self, def: EnumDef) -> EnumIdx {
        let idx = EnumIdx::new(self.enums.len());
        self.enums.push(def);
        idx
    }

    pub fn add_protocol(&mut self, def: ProtocolDef) -> ProtocolIdx {
        let idx = ProtocolIdx::new(self.protocols.len());
        self.protocols.push(def);
        idx
    }

    pub fn add_witness(&mut self, def: WitnessDef) -> WitnessIdx {
        let idx = WitnessIdx::new(self.witnesses.len());
        self.witnesses.push(def);
        idx
    }

    pub fn add_static(&mut self, def: StaticDef) -> StaticIdx {
        let idx = StaticIdx::new(self.statics.len());
        self.statics.push(def);
        idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_new() {
        let module = MirModule::new("test");
        assert_eq!(module.name, "test");
        assert!(module.functions.is_empty());
        assert!(module.structs.is_empty());
        assert!(module.enums.is_empty());
        assert!(module.protocols.is_empty());
        assert!(module.witnesses.is_empty());
        assert!(module.statics.is_empty());
    }

    #[test]
    fn register_and_resolve_name() {
        let mut module = MirModule::new("test");
        let entity = Entity::from_raw(1);
        module.register_name(entity, "std.Array");
        assert_eq!(module.resolve_name(entity), "std.Array");
    }

    #[test]
    fn resolve_unknown_name() {
        let module = MirModule::new("test");
        assert_eq!(module.resolve_name(Entity::from_raw(999)), "<unknown>");
    }

    #[test]
    fn add_function() {
        let mut module = MirModule::new("test");
        let entity = Entity::from_raw(1);
        let i64_ty = module.ty_arena.i64();
        let func = FunctionDef::new(entity, "main", i64_ty);
        let idx = module.add_function(func);
        assert_eq!(idx.index(), 0);
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "main");
    }

    #[test]
    fn add_struct() {
        let mut module = MirModule::new("test");
        let def = StructDef::new(Entity::from_raw(1), "Point");
        let idx = module.add_struct(def);
        assert_eq!(idx.index(), 0);
        assert_eq!(module.structs.len(), 1);
    }

    #[test]
    fn add_enum() {
        let mut module = MirModule::new("test");
        let def = EnumDef::new(Entity::from_raw(1), "Optional");
        let idx = module.add_enum(def);
        assert_eq!(idx.index(), 0);
        assert_eq!(module.enums.len(), 1);
    }

    #[test]
    fn add_multiple_items() {
        let mut module = MirModule::new("test");
        let i64_ty = module.ty_arena.i64();
        module.add_function(FunctionDef::new(Entity::from_raw(1), "f1", i64_ty));
        module.add_function(FunctionDef::new(Entity::from_raw(2), "f2", i64_ty));
        module.add_struct(StructDef::new(Entity::from_raw(3), "S1"));
        assert_eq!(module.functions.len(), 2);
        assert_eq!(module.structs.len(), 1);
    }
}
