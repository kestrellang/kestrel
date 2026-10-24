use indexmap::IndexMap;
use kestrel_hecs::Entity;

use crate::item::TypeInfo;
use crate::item::function::ExternInfo;
use crate::op::IntBits;
use crate::ty::{ParamConvention, TyArena};
use crate::{MonoFuncId, TyId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstantiationKey {
    pub func_entity: Entity,
    pub type_args: Vec<TyId>,
    pub self_type: Option<TyId>,
}

impl InstantiationKey {
    pub fn new(func_entity: Entity, type_args: Vec<TyId>, self_type: Option<TyId>) -> Self {
        Self {
            func_entity,
            type_args,
            self_type,
        }
    }

    pub fn concrete(func_entity: Entity) -> Self {
        Self {
            func_entity,
            type_args: Vec::new(),
            self_type: None,
        }
    }
}

/// Key for monomorphized struct/enum: (source entity, concrete type args).
pub type MonoTypeKey = (Entity, Vec<TyId>);

#[derive(Debug)]
pub struct MonoModule {
    pub functions: Vec<MonoFunction>,
    /// Existential function tables, instantiated from
    /// `MirModule::vtable_templates` (`item::vtable` has the layout).
    pub vtables: Vec<MonoVTable>,
    pub structs: IndexMap<MonoTypeKey, MonoStruct>,
    pub enums: IndexMap<MonoTypeKey, MonoEnum>,
    pub statics: IndexMap<Entity, crate::item::static_def::StaticDef>,
    pub ty_arena: TyArena,
    pub entity_names: IndexMap<Entity, String>,
}

impl MonoModule {
    pub fn new(ty_arena: TyArena) -> Self {
        Self {
            functions: Vec::new(),
            vtables: Vec::new(),
            structs: IndexMap::new(),
            enums: IndexMap::new(),
            statics: IndexMap::new(),
            ty_arena,
            entity_names: IndexMap::new(),
        }
    }

    /// Is a value of type `ty` a two-word (data, table) value? See
    /// [`crate::ty::is_fat`]; struct tails come from this module's instances.
    pub fn is_fat(&self, ty: crate::TyId) -> bool {
        crate::ty::is_fat(&mut MonoTails(self), ty)
    }

    pub fn resolve_name(&self, entity: Entity) -> &str {
        self.entity_names
            .get(&entity)
            .map(|s| s.as_str())
            .unwrap_or("<unknown>")
    }

    pub fn add_function(&mut self, func: MonoFunction) -> MonoFuncId {
        let id = MonoFuncId::new(self.functions.len());
        self.functions.push(func);
        id
    }
}

use crate::body::OssaBody;

#[derive(Debug, Clone)]
pub struct MonoFunction {
    pub name: String,
    pub source: Entity,
    pub type_args: Vec<TyId>,
    pub self_type: Option<TyId>,
    pub params: Vec<MonoParam>,
    pub ret: TyId,
    pub body: Option<OssaBody>,
    pub extern_info: Option<ExternInfo>,
    /// True for the `@main` entry point (propagated from `FunctionDef.is_main`).
    /// Consumed by the codegen backend to select the entry and export C `main`.
    pub is_main: bool,
    /// True when `ret` is a `MirTy::Ref` (`-> &T`): the function returns the
    /// raw pointer (`ret_borrow` ABI). Derived once at creation via
    /// `item::function::ret_convention`; codegen reads the bit, never
    /// re-matches the type.
    pub ret_borrow: bool,
    /// `@inline`, copied from the generic definition to every instance.
    pub inline: Option<crate::item::function::InlineMode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonoParam {
    pub name: String,
    pub ty: TyId,
    pub convention: ParamConvention,
    pub label: Option<String>,
}

impl MonoParam {
    pub fn new(name: impl Into<String>, ty: TyId, convention: ParamConvention) -> Self {
        Self {
            name: name.into(),
            ty,
            convention,
            label: None,
        }
    }

    pub fn with_label(
        name: impl Into<String>,
        ty: TyId,
        convention: ParamConvention,
        label: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            ty,
            convention,
            label,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonoField {
    pub name: String,
    pub ty: TyId,
}

impl MonoField {
    pub fn new(name: impl Into<String>, ty: TyId) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MonoStruct {
    pub source: Entity,
    pub type_args: Vec<TyId>,
    pub fields: Vec<MonoField>,
    pub type_info: TypeInfo,
}

impl MonoStruct {
    pub fn new(source: Entity, type_args: Vec<TyId>) -> Self {
        Self {
            source,
            type_args,
            fields: Vec::new(),
            type_info: TypeInfo::none(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonoEnumCase {
    pub name: String,
    pub discriminant: u32,
    pub payload_fields: Vec<MonoField>,
}

impl MonoEnumCase {
    pub fn new(name: impl Into<String>, discriminant: u32) -> Self {
        Self {
            name: name.into(),
            discriminant,
            payload_fields: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MonoEnum {
    pub source: Entity,
    pub type_args: Vec<TyId>,
    pub cases: Vec<MonoEnumCase>,
    pub type_info: TypeInfo,
    pub discriminant_width: IntBits,
}

impl MonoEnum {
    pub fn new(source: Entity, type_args: Vec<TyId>, discriminant_width: IntBits) -> Self {
        Self {
            source,
            type_args,
            cases: Vec::new(),
            type_info: TypeInfo::none(),
            discriminant_width,
        }
    }

    pub fn payload_offset(&self) -> u64 {
        match &self.type_info.layout {
            Some(crate::item::Layout::Enum(el)) => el.payload_offset,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    #[test]
    fn instantiation_key_concrete() {
        let key = InstantiationKey::concrete(entity(1));
        assert_eq!(key.func_entity, entity(1));
        assert!(key.type_args.is_empty());
        assert!(key.self_type.is_none());
    }

    #[test]
    fn instantiation_key_equality() {
        let k1 = InstantiationKey::new(entity(1), vec![TyId::new(0), TyId::new(1)], None);
        let k2 = InstantiationKey::new(entity(1), vec![TyId::new(0), TyId::new(1)], None);
        let k3 = InstantiationKey::new(entity(1), vec![TyId::new(0), TyId::new(2)], None);
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn instantiation_key_self_type_matters() {
        let k1 = InstantiationKey::new(entity(1), vec![], Some(TyId::new(5)));
        let k2 = InstantiationKey::new(entity(1), vec![], Some(TyId::new(6)));
        let k3 = InstantiationKey::new(entity(1), vec![], None);
        assert_ne!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn instantiation_key_hash_consistent() {
        use std::collections::HashSet;
        let k1 = InstantiationKey::new(entity(1), vec![TyId::new(0)], None);
        let k2 = InstantiationKey::new(entity(1), vec![TyId::new(0)], None);
        let mut set = HashSet::new();
        set.insert(k1);
        assert!(set.contains(&k2));
    }

    #[test]
    fn mono_module_new() {
        let arena = TyArena::new();
        let module = MonoModule::new(arena);
        assert!(module.functions.is_empty());
        assert!(module.structs.is_empty());
        assert!(module.enums.is_empty());
        assert!(module.statics.is_empty());
    }

    #[test]
    fn mono_module_add_function() {
        let mut arena = TyArena::new();
        let ret = arena.unit();
        let mut module = MonoModule::new(arena);
        let func = MonoFunction {
            name: "_K04_main".into(),
            source: entity(1),
            type_args: vec![],
            self_type: None,
            params: vec![],
            ret,
            body: None,
            extern_info: None,
            is_main: false,
            ret_borrow: false,
            inline: None,
        };
        let id = module.add_function(func);
        assert_eq!(id.index(), 0);
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "_K04_main");
    }

    #[test]
    fn mono_module_resolve_name() {
        let arena = TyArena::new();
        let mut module = MonoModule::new(arena);
        module
            .entity_names
            .insert(entity(1), "std.Array".to_string());
        assert_eq!(module.resolve_name(entity(1)), "std.Array");
        assert_eq!(module.resolve_name(entity(999)), "<unknown>");
    }

    #[test]
    fn mono_param_new() {
        let param = MonoParam::new("x", TyId::new(0), ParamConvention::Consuming);
        assert_eq!(param.name, "x");
        assert_eq!(param.convention, ParamConvention::Consuming);
    }

    #[test]
    fn mono_field_new() {
        let field = MonoField::new("count", TyId::new(3));
        assert_eq!(field.name, "count");
        assert_eq!(field.ty, TyId::new(3));
    }

    #[test]
    fn mono_struct_new() {
        let s = MonoStruct::new(entity(1), vec![TyId::new(0)]);
        assert_eq!(s.source, entity(1));
        assert_eq!(s.type_args.len(), 1);
        assert!(s.fields.is_empty());
        assert!(s.type_info.layout.is_none());
    }

    #[test]
    fn mono_enum_new() {
        let e = MonoEnum::new(entity(1), vec![], IntBits::I8);
        assert_eq!(e.discriminant_width, IntBits::I8);
        assert!(e.cases.is_empty());
        assert!(e.type_info.layout.is_none());
    }

    #[test]
    fn mono_enum_case_new() {
        let mut c = MonoEnumCase::new("Some", 1);
        c.payload_fields.push(MonoField::new("value", TyId::new(0)));
        assert_eq!(c.name, "Some");
        assert_eq!(c.discriminant, 1);
        assert_eq!(c.payload_fields.len(), 1);
    }
}

/// [`crate::ty::TailLookup`] over finished mono instances.
struct MonoTails<'a>(&'a MonoModule);

impl crate::ty::TailLookup for MonoTails<'_> {
    fn arena(&self) -> &TyArena {
        &self.0.ty_arena
    }
    fn last_field(&mut self, entity: Entity, args: &[crate::TyId]) -> Option<crate::TyId> {
        self.0
            .structs
            .get(&(entity, args.to_vec()))
            .and_then(|s| s.fields.last())
            .map(|f| f.ty)
    }
}

/// One existential function table: a template at a concrete payload. Codegen
/// emits it as a read-only array of pointer-sized words.
#[derive(Debug, Clone)]
pub struct MonoVTable {
    pub name: String,
    pub entries: Vec<MonoVTableEntry>,
}

/// One resolved table word.
#[derive(Debug, Clone, PartialEq)]
pub enum MonoVTableEntry {
    Func(MonoFuncId),
    /// Size of this payload type.
    Size(crate::TyId),
    /// Alignment of this payload type.
    Align(crate::TyId),
    Table(crate::MonoVTableId),
    Null,
}
