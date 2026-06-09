use std::collections::HashMap;

use kestrel_hecs::Entity;

pub use crate::id::TyId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamConvention {
    Borrow,
    MutBorrow,
    Consuming,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirTy {
    I8,
    I16,
    I32,
    I64,
    F16,
    F32,
    F64,
    Bool,
    Never,
    Str,

    Pointer(TyId),

    Tuple(Vec<TyId>),
    Named {
        entity: Entity,
        type_args: Vec<TyId>,
    },

    TypeParam(Entity),
    AssociatedProjection {
        base: TyId,
        protocol: Entity,
        assoc_type: Entity,
    },

    FuncThin {
        params: Vec<(TyId, ParamConvention)>,
        ret: TyId,
    },
    FuncThick {
        params: Vec<(TyId, ParamConvention)>,
        ret: TyId,
    },

    /// Second-class reference `&T` / `&mutating T` (stage 1). Appears ONLY
    /// on signatures (`FunctionDef.ret` / `MonoFunction.ret`) — never as a
    /// `ValueDef.ty`: a ref-typed call result registers as an ordinary
    /// `@guaranteed` value of the *pointee* type ("a borrowed param that
    /// travels"). Layout is a pointer scalar.
    Ref {
        pointee: TyId,
        mutating: bool,
    },

    Error,
}

#[derive(Debug, Clone)]
pub struct TyArena {
    types: Vec<MirTy>,
    intern_map: HashMap<MirTy, TyId>,
}

impl TyArena {
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            intern_map: HashMap::new(),
        }
    }

    pub fn intern(&mut self, ty: MirTy) -> TyId {
        if let Some(&id) = self.intern_map.get(&ty) {
            return id;
        }
        let id = TyId::new(self.types.len());
        self.types.push(ty.clone());
        self.intern_map.insert(ty, id);
        id
    }

    pub fn get(&self, id: TyId) -> &MirTy {
        &self.types[id.index()]
    }

    pub fn find(&self, predicate: impl Fn(&MirTy) -> bool) -> Option<TyId> {
        self.types.iter().position(predicate).map(TyId::new)
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    pub fn i8(&mut self) -> TyId {
        self.intern(MirTy::I8)
    }
    pub fn i16(&mut self) -> TyId {
        self.intern(MirTy::I16)
    }
    pub fn i32(&mut self) -> TyId {
        self.intern(MirTy::I32)
    }
    pub fn i64(&mut self) -> TyId {
        self.intern(MirTy::I64)
    }
    pub fn f16(&mut self) -> TyId {
        self.intern(MirTy::F16)
    }
    pub fn f32(&mut self) -> TyId {
        self.intern(MirTy::F32)
    }
    pub fn f64(&mut self) -> TyId {
        self.intern(MirTy::F64)
    }
    pub fn bool(&mut self) -> TyId {
        self.intern(MirTy::Bool)
    }
    pub fn never(&mut self) -> TyId {
        self.intern(MirTy::Never)
    }
    pub fn str_ty(&mut self) -> TyId {
        self.intern(MirTy::Str)
    }
    pub fn unit(&mut self) -> TyId {
        self.intern(MirTy::Tuple(vec![]))
    }
    pub fn pointer(&mut self, pointee: TyId) -> TyId {
        self.intern(MirTy::Pointer(pointee))
    }
    pub fn tuple(&mut self, elems: Vec<TyId>) -> TyId {
        self.intern(MirTy::Tuple(elems))
    }
    pub fn named(&mut self, entity: Entity, type_args: Vec<TyId>) -> TyId {
        self.intern(MirTy::Named { entity, type_args })
    }
    pub fn ref_ty(&mut self, pointee: TyId, mutating: bool) -> TyId {
        self.intern(MirTy::Ref { pointee, mutating })
    }
    /// The pointee of a `Ref`, or the type itself — signature consumers use
    /// this to recover the value type a ref-typed return registers as.
    pub fn peel_ref(&self, id: TyId) -> TyId {
        match self.get(id) {
            MirTy::Ref { pointee, .. } => *pointee,
            _ => id,
        }
    }
    pub fn error(&mut self) -> TyId {
        self.intern(MirTy::Error)
    }
}

impl Default for TyArena {
    fn default() -> Self {
        Self::new()
    }
}
