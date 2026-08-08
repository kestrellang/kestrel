use std::collections::HashMap;

use kestrel_hecs::Entity;

pub use crate::id::TyId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamConvention {
    Borrow,
    MutBorrow,
    Consuming,
}

/// The closure tier a `FuncThick` names — the MIR mirror of
/// `kestrel_ast::FnTypeKind` (this crate does not depend on the AST, exactly
/// like [`ParamConvention`]). Lowering maps the two 1:1.
///
/// The kind is part of the type's interned identity, so it participates in
/// mangling and witness `match_pattern`: two instantiations differing only in
/// kind must not collide.
///
/// - **View tier** (`Normal` / `Mutating`): the environment holds *addresses*
///   into the enclosing frame and owns nothing; the value is frame-bound.
/// - **Owning tier** (`Consuming` / `Escaping`): the environment owns its
///   captures and may outlive the frame (Phase E2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FnKind {
    #[default]
    Normal,
    Mutating,
    Consuming,
    Escaping,
}

impl FnKind {
    /// A view kind holds addresses into the frame and owns no captures.
    pub fn is_view(self) -> bool {
        matches!(self, FnKind::Normal | FnKind::Mutating)
    }

    /// Display/mangle tag — stable, one char per kind.
    pub fn tag(self) -> char {
        match self {
            FnKind::Normal => 'n',
            FnKind::Mutating => 'm',
            FnKind::Consuming => 'c',
            FnKind::Escaping => 'e',
        }
    }

    /// This kind heap-boxes its environment and therefore carries the
    /// type-erased share/release pair inline (4-word layout, `ApplyPartial`
    /// `retain`/`release`, the `expand.rs` erased dispatch).
    ///
    /// BOTH owning kinds are boxed, but with different containers:
    /// `Escaping` uses the SHARED box (`@builtin(.SharedBox)`, many handles,
    /// many calls), `Consuming` a UNIQUE one (`@builtin(.UniqueBox)`, one
    /// owner, one call). The shared box is unusable for `consuming` because
    /// its release drops the WHOLE environment unconditionally, double-freeing
    /// the slots a one-shot body already moved out (plan D5).
    pub fn is_boxed(self) -> bool {
        matches!(self, FnKind::Consuming | FnKind::Escaping)
    }

    /// This kind's environment is SHARED — duplicating the value is the box's
    /// share operation (a retain), so the value is Cloneable and its container
    /// becomes Cloneable too. `Consuming` is boxed but NOT shared: it is
    /// `not Copyable`, so no retain path may ever be reachable for it.
    pub fn is_shared(self) -> bool {
        matches!(self, FnKind::Escaping)
    }

    /// Rendering suffix for `display.rs` — `""` for the unmarked `Normal`.
    pub fn display_suffix(self) -> &'static str {
        match self {
            FnKind::Normal => "",
            FnKind::Mutating => "[mutating]",
            FnKind::Consuming => "[consuming]",
            FnKind::Escaping => "[escaping]",
        }
    }
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
        /// The closure tier (view vs owning). Part of the interned identity:
        /// mangling and witness matching are kind-exact.
        kind: FnKind,
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

/// Machine words in a `FuncThick` value of `kind` — the SINGLE source of the
/// closure-value width, shared by `passes/layout.rs`, both codegen `ty.rs`
/// classifiers and both `compile_apply_partial`s (lockstep 9).
pub fn func_thick_words(kind: FnKind) -> u64 {
    if kind.is_boxed() { 4 } else { 2 }
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
    /// Does this type carry a `&T` by value (stage 2b ref-bearing
    /// aggregates)? Drives the owned-return escape check: a function whose
    /// return type contains a ref gets the root rule applied to the
    /// returned value's provenance.
    ///
    /// Deliberately an OVER-approximation on `Named` (every type arg is
    /// walked, even args the nominal never stores): a false positive only
    /// subjects an untainted (self-rooted) value to a check it passes.
    /// `Pointer` pointees are NOT walked (a raw pointer doesn't carry its
    /// pointee by value — mirrors `Pointer[T]` being Static regardless of
    /// T). Fn types / params / projections are `false`: signature refs are
    /// kept out of value-carried positions by E486/E491, and generic code
    /// is gated by the front-end Static bound (escape checking is pre-mono;
    /// instances are parametric). No cycle guard: interning is acyclic and
    /// the walk never expands a nominal's fields, only its type args.
    pub fn contains_ref(&self, id: TyId) -> bool {
        match self.get(id) {
            MirTy::Ref { .. } => true,
            MirTy::Tuple(elems) => elems.iter().any(|&e| self.contains_ref(e)),
            MirTy::Named { type_args, .. } => type_args.iter().any(|&a| self.contains_ref(a)),
            _ => false,
        }
    }
    /// Does this type carry a closure (`FuncThick`) by value? Drives the
    /// owned-return escape check for capturing closures: a closure's
    /// environment is stack-allocated in the defining frame, so a returned
    /// closure that captured a local cannot outlive the call (#174). Mirror of
    /// `contains_ref`'s shape — `FuncThin` (a bare function pointer, no env) is
    /// NOT a closure and never taints. Like `contains_ref`, `Named` walks only
    /// type args (an over-approximation that a self-rooted value passes), not
    /// stored fields.
    pub fn contains_closure(&self, id: TyId) -> bool {
        match self.get(id) {
            MirTy::FuncThick { .. } => true,
            MirTy::Tuple(elems) => elems.iter().any(|&e| self.contains_closure(e)),
            MirTy::Named { type_args, .. } => type_args.iter().any(|&a| self.contains_closure(a)),
            _ => false,
        }
    }
    /// Like `contains_ref`, but true only when a `&mutating T` is carried —
    /// feeds the E495 (mutable-root) variant of the owned-return check.
    /// A shared `&T` wrapper still recurses (its pointee type may carry a
    /// `&mutating` component, e.g. `-> &Optional[&mutating U]`).
    pub fn contains_mutating_ref(&self, id: TyId) -> bool {
        match self.get(id) {
            MirTy::Ref { mutating: true, .. } => true,
            MirTy::Ref {
                pointee,
                mutating: false,
            } => self.contains_mutating_ref(*pointee),
            MirTy::Tuple(elems) => elems.iter().any(|&e| self.contains_mutating_ref(e)),
            MirTy::Named { type_args, .. } => {
                type_args.iter().any(|&a| self.contains_mutating_ref(a))
            },
            _ => false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_ref_finds_nested_args_and_tuples() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        let r = a.ref_ty(i64, false);
        let opt_ref = a.named(Entity::from_raw(7), vec![r]);
        let tup = a.tuple(vec![i64, opt_ref]);
        assert!(a.contains_ref(r));
        assert!(a.contains_ref(opt_ref));
        assert!(a.contains_ref(tup));
        assert!(!a.contains_ref(i64));
        let plain = a.tuple(vec![i64, i64]);
        assert!(!a.contains_ref(plain));
    }

    #[test]
    fn contains_ref_skips_pointer_and_fn_types() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        let r = a.ref_ty(i64, true);
        let ptr = a.pointer(r);
        assert!(!a.contains_ref(ptr));
        let f = a.intern(MirTy::FuncThick {
            kind: FnKind::Normal,
            params: vec![(r, ParamConvention::Consuming)],
            ret: r,
        });
        assert!(!a.contains_ref(f));
    }

    #[test]
    fn contains_mutating_ref_distinguishes_bits() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        let shared = a.ref_ty(i64, false);
        let muta = a.ref_ty(i64, true);
        assert!(!a.contains_mutating_ref(shared));
        assert!(a.contains_mutating_ref(muta));
        // A shared ref over a mutating-ref-bearing pointee still counts.
        let inner = a.named(Entity::from_raw(7), vec![muta]);
        let outer = a.ref_ty(inner, false);
        assert!(a.contains_mutating_ref(outer));
        assert!(a.contains_ref(outer));
    }
}
