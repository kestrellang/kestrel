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
pub use id::{BlockId, FieldIdx, MonoFuncId, TyId, ValueId, VariantIdx};
pub use immediate::{Immediate, ImmediateKind};
pub use item::WitnessMethodKey;
pub use item::{CopyBehavior, DropBehavior, Layout, TargetConfig, TypeInfo, TypeParamDef};
pub use layout::{EnumLayout, StructLayout};
pub use op::{DivGuard, FloatBits, FloatMathKind, FloatPredicateKind, IntBits, Op, Signedness};
pub use substitute::{SubstMap, substitute};
pub use terminator::SwitchCase;
pub use ty::{MirTy, ParamConvention, TyArena};
pub use value::Ownership;

use item::enum_def::EnumDef;
use item::function::FunctionDef;
use item::protocol::ProtocolDef;
use item::static_def::StaticDef;
use item::struct_def::StructDef;
use item::witness::WitnessDef;

/// Result of `MirModule::escape_carry` — which escape-relevant components a type
/// carries by value (deep, through nominal fields). `any_ref`/`mutating_ref`
/// drive the ref-carrier escape modes (E494/E495); `closure` drives the
/// capturing-closure escape mode (#174).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EscapeCarry {
    pub any_ref: bool,
    pub mutating_ref: bool,
    pub closure: bool,
}

impl EscapeCarry {
    /// True when the type carries any escape-relevant component (ref or closure).
    pub fn any(&self) -> bool {
        self.any_ref || self.closure
    }
}

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

    /// Deep escape-carry analysis: does a value of `ty` carry a reference or a
    /// (capturing) closure BY VALUE *anywhere*, including inside nominal STORED
    /// FIELDS / enum payloads? Unlike `TyArena::contains_ref`/`contains_closure`
    /// (which walk only type args — they miss a concrete `&Int64` field or a
    /// closure field on a non-generic struct), this recurses fields too, so the
    /// return escape check (`verify::check_escapes`) is gated correctly when a
    /// ref or capturing closure is laundered through a struct field (#174).
    /// Over-approximation is safe: an untainted (self-rooted) value just passes
    /// a check it would have passed anyway. Cycle-guarded for recursive types.
    /// `Pointer` pointees are NOT walked (a raw pointer doesn't carry its
    /// pointee by value — mirrors `contains_ref`).
    pub fn escape_carry(&self, ty: TyId) -> EscapeCarry {
        let mut out = EscapeCarry::default();
        let mut visited = std::collections::HashSet::new();
        self.escape_carry_into(ty, &mut out, &mut visited);
        out
    }

    fn escape_carry_into(
        &self,
        ty: TyId,
        out: &mut EscapeCarry,
        visited: &mut std::collections::HashSet<TyId>,
    ) {
        if out.any_ref && out.mutating_ref && out.closure {
            return; // saturated — nothing more to learn
        }
        if !visited.insert(ty) {
            return;
        }
        match self.ty_arena.get(ty) {
            MirTy::Ref { mutating, pointee } => {
                out.any_ref = true;
                out.mutating_ref |= *mutating;
                // A shared `&T` may wrap a `&mutating U` (mirror contains_mutating_ref).
                let pointee = *pointee;
                self.escape_carry_into(pointee, out, visited);
            },
            MirTy::FuncThick { .. } => out.closure = true,
            MirTy::Tuple(elems) => {
                for e in elems.clone() {
                    self.escape_carry_into(e, out, visited);
                }
            },
            MirTy::Named { entity, type_args } => {
                let entity = *entity;
                for a in type_args.clone() {
                    self.escape_carry_into(a, out, visited);
                }
                if let Some(def) = self.structs.get(&entity) {
                    for fty in def.fields.iter().map(|f| f.ty).collect::<Vec<_>>() {
                        self.escape_carry_into(fty, out, visited);
                    }
                } else if let Some(def) = self.enums.get(&entity) {
                    for pty in def
                        .cases
                        .iter()
                        .flat_map(|c| c.payload_fields.iter().map(|f| f.ty))
                        .collect::<Vec<_>>()
                    {
                        self.escape_carry_into(pty, out, visited);
                    }
                }
            },
            _ => {},
        }
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
