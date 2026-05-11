//! Statements — operations within basic blocks.

use crate::WitnessMethodKey;
use crate::id::LocalId;
use crate::immediate::Immediate;
use crate::op::Op;
use crate::place::Place;
use crate::ty::MirTy;
use crate::value::Value;
use kestrel_hecs::Entity;
use kestrel_span::Span;

/// A statement in a basic block.
#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Option<Span>,
}

impl Statement {
    pub fn new(kind: StatementKind) -> Self {
        Self { kind, span: None }
    }

    pub fn with_span(kind: StatementKind, span: Span) -> Self {
        Self {
            kind,
            span: Some(span),
        }
    }
}

/// The different kinds of statements.
#[derive(Debug, Clone)]
pub enum StatementKind {
    /// `dest = rvalue`
    Assign { dest: Place, rvalue: Rvalue },

    /// `call callee(args...)` with optional return destination.
    /// Calls are always statements (not rvalues) because they have side effects.
    Call {
        dest: Option<Place>,
        callee: Callee,
        args: Vec<Value>,
    },

    /// `drop <place>` — unconditionally run destructor.
    ///
    /// Emitted exclusively by `kestrel-ownership::drop_elab`. Lowering must
    /// never emit this. The verifier (Stage 6+) enforces both invariants.
    Drop { place: Place },

    /// `drop <place> if <flag>` — conditionally run destructor only when the
    /// per-local `_init_*: Bool` flag (also maintained by drop-elab) is
    /// `true`. The flag is initialised to `false` at function entry, set
    /// `true` after each gen of the underlying path, and `false` after each
    /// kill.
    DropIf { place: Place, flag: LocalId },
}

/// The right-hand side of an assignment.
#[derive(Debug, Clone)]
pub enum Rvalue {
    // === Ownership/reference semantics ===
    /// `move <place>` — take ownership, invalidate source
    Move(Place),
    /// `copy <place>` — copy value, source remains valid
    Copy(Place),
    /// `ref <place>` — immutable borrow
    Ref(Place),
    /// `ref var <place>` — mutable borrow
    RefMut(Place),

    // === Constants ===
    /// Load a constant value
    Const(Immediate),

    // === Operations (arity-typed) ===
    /// Unary operation
    Op1 { op: Op, arg: Value },
    /// Binary operation
    Op2 { op: Op, lhs: Value, rhs: Value },
    /// Ternary operation (e.g., FloatFma)
    Op3 {
        op: Op,
        a: Value,
        b: Value,
        c: Value,
    },

    // === Composite construction ===
    /// `construct Type { field: value, ... }`
    Construct {
        ty: MirTy,
        fields: Vec<(String, Value)>,
    },
    /// `tuple (v0, v1, ...)`
    Tuple(Vec<Value>),
    /// `apply partial func(captures...)` — create a thick callable from a function + captures
    ApplyPartial { func: Entity, captures: Vec<Value> },

    /// `enum Enum.Variant` or `enum Enum.Variant(payload...)`
    EnumVariant {
        enum_ty: MirTy,
        variant: String,
        payload: Vec<Value>,
    },

    /// `array[T] [v0, v1, ...]` — array literal with homogeneous element type.
    ///
    /// Also used for dictionary literals: `[k: v, ...]` lowers to
    /// `ArrayLiteral { element_ty: Tuple(K, V), values: [tuple0, tuple1, ...] }`.
    ArrayLiteral {
        element_ty: MirTy,
        values: Vec<Value>,
    },
}

/// What's being called. Self-type is always explicit — no inference at codegen time.
#[derive(Debug, Clone)]
pub enum Callee {
    /// Direct call to a known function.
    Direct {
        func: Entity,
        type_args: Vec<MirTy>,
        /// Explicit self-type for methods (None for free functions).
        self_type: Option<MirTy>,
    },

    /// Thin function pointer call (no environment).
    Thin(Place),

    /// Thick callable call (has environment pointer).
    Thick(Place),

    /// Witness method dispatch — resolved at monomorphization time.
    Witness {
        protocol: Entity,
        method: WitnessMethodKey,
        self_type: MirTy,
        method_type_args: Vec<MirTy>,
    },
}

impl Callee {
    /// Create a direct callee (non-generic, non-method).
    pub fn direct(func: Entity) -> Self {
        Callee::Direct {
            func,
            type_args: Vec::new(),
            self_type: None,
        }
    }

    /// Create a direct callee with type arguments.
    pub fn direct_generic(func: Entity, type_args: Vec<MirTy>) -> Self {
        Callee::Direct {
            func,
            type_args,
            self_type: None,
        }
    }

    /// Create a direct method callee with explicit self-type.
    pub fn method(func: Entity, type_args: Vec<MirTy>, self_type: MirTy) -> Self {
        Callee::Direct {
            func,
            type_args,
            self_type: Some(self_type),
        }
    }

    /// Create a witness method callee.
    pub fn witness(
        protocol: Entity,
        method: impl Into<WitnessMethodKey>,
        self_type: MirTy,
        method_type_args: Vec<MirTy>,
    ) -> Self {
        Callee::Witness {
            protocol,
            method: method.into(),
            self_type,
            method_type_args,
        }
    }
}

// `CallArg` and `PassingMode` removed as part of the Stage 3 greenfield
// memory-model rewrite. Call arguments are now `Vec<Value>`, and the four
// pass-by modes are expressed directly via the `Value::{Copy, Move, Ref,
// RefMut}` variants on the operand.
