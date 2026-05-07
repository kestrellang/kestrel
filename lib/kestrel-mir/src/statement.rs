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
use std::fmt;

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
        args: Vec<CallArg>,
    },

    /// `deinit <place>` — unconditionally run destructor.
    Deinit { place: Place },

    /// `deinit <place> if <flag>` — conditionally run destructor.
    /// The flag is a Bool local tracking whether the value is still live.
    DeinitIf { place: Place, flag: LocalId },

    /// `<flag> = true/false` — set a deinit tracking flag.
    SetDeinitFlag { flag: LocalId, value: bool },

    /// `scope_live %local` — marks a local as entering scope.
    /// The drop elaboration pass reads this as "init-state resets to dead here."
    /// Emitted at loop headers so loop-scoped locals start each iteration uninitialized.
    ScopeLive(LocalId),
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

/// An argument to a function call, with its value and passing mode.
#[derive(Debug, Clone)]
pub struct CallArg {
    pub value: Value,
    pub mode: PassingMode,
}

impl CallArg {
    pub fn new(value: Value, mode: PassingMode) -> Self {
        Self { value, mode }
    }

    pub fn borrow(value: Value) -> Self {
        Self::new(value, PassingMode::Ref)
    }

    pub fn mutating(value: Value) -> Self {
        Self::new(value, PassingMode::MutRef)
    }

    pub fn copy(value: Value) -> Self {
        Self::new(value, PassingMode::Copy)
    }

    pub fn moving(value: Value) -> Self {
        Self::new(value, PassingMode::Move)
    }
}

/// How an argument is passed to a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassingMode {
    /// Immutable borrow (default).
    Ref,
    /// Mutable borrow.
    MutRef,
    /// Value copied, original retained.
    Copy,
    /// Value moved, original invalidated.
    Move,
}

impl PassingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            PassingMode::Ref => "ref",
            PassingMode::MutRef => "mut",
            PassingMode::Copy => "copy",
            PassingMode::Move => "move",
        }
    }
}

impl fmt::Display for PassingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
