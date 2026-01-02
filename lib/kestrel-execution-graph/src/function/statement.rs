//! Statements (operations within basic blocks).

use crate::function::{Immediate, Place, Value};
use crate::id::{Id, Local, QualifiedName, Ty};
use crate::metadata::{Metadata, Prior};
use crate::MirContext;
use std::fmt;

/// How an argument is passed to a function.
///
/// This enum captures the calling convention semantics for each argument,
/// based on the parameter's access mode and the argument type's copy semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassingMode {
    /// Borrow - immutable reference (default access mode).
    /// The callee receives a reference and cannot modify the value.
    Ref,

    /// Mutating - mutable reference.
    /// The callee receives a mutable reference and can modify the value in place.
    MutRef,

    /// Copy - value is copied, original retained.
    /// Used for `consuming` parameters when the type is `Copyable`.
    Copy,

    /// Move - value is moved, original invalidated.
    /// Used for `consuming` parameters when the type is `not Copyable`.
    Move,
}

impl PassingMode {
    /// Get the string representation for printing.
    pub fn as_str(&self) -> &'static str {
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
        write!(f, "{}", self.as_str())
    }
}

/// A statement in a basic block.
#[derive(Debug, Clone)]
pub struct Statement {
    pub meta: Metadata,
    pub priors: Vec<Prior<Statement>>,
    pub kind: StatementKind,
}

/// An argument to a function call, with its value and passing mode.
#[derive(Debug, Clone)]
pub struct CallArg {
    /// The argument value.
    pub value: Value,
    /// How the argument is passed.
    pub mode: PassingMode,
}

impl CallArg {
    /// Create a new call argument.
    pub fn new(value: Value, mode: PassingMode) -> Self {
        Self { value, mode }
    }

    /// Create a ref (borrow) argument.
    pub fn borrow(value: Value) -> Self {
        Self::new(value, PassingMode::Ref)
    }

    /// Create a mut ref (mutating) argument.
    pub fn mutating(value: Value) -> Self {
        Self::new(value, PassingMode::MutRef)
    }

    /// Create a copy argument.
    pub fn copy(value: Value) -> Self {
        Self::new(value, PassingMode::Copy)
    }

    /// Create a move argument.
    pub fn moving(value: Value) -> Self {
        Self::new(value, PassingMode::Move)
    }
}

/// The different kinds of statements.
#[derive(Debug, Clone)]
pub enum StatementKind {
    /// `<place> = <rvalue>`
    Assign { dest: Place, rvalue: Rvalue },

    /// `call func(args...)` (unit return, no assignment)
    Call { callee: Callee, args: Vec<CallArg> },

    /// `deinit <place>` - unconditionally run destructor
    ///
    /// Used when a value is definitely valid at scope exit.
    Deinit { place: Place },

    /// `deinit <place> if <flag>` - conditionally run destructor
    ///
    /// Used when a value may have been moved in one branch but not another.
    /// The flag is a Bool local that is true if the value needs to be deinited.
    DeinitIf { place: Place, flag: Id<Local> },

    /// `<flag> = true/false` - set a deinit flag
    ///
    /// Used to track whether a value was moved in a branch.
    SetDeinitFlag { flag: Id<Local>, value: bool },
}

/// The right-hand side of an assignment.
#[derive(Debug, Clone)]
pub enum Rvalue {
    /// `move <place>`
    Move(Place),

    /// `copy <place>`
    Copy(Place),

    /// `ref <place>`
    Ref(Place),

    /// `ref var <place>`
    RefMut(Place),

    /// `<immediate>`
    Use(Immediate),

    /// Binary operation
    BinaryOp { op: BinOp, lhs: Value, rhs: Value },

    /// Unary operation
    UnaryOp { op: UnOp, operand: Value },

    /// `construct Type { field: value, ... }`
    Construct {
        ty: Id<Ty>,
        fields: Vec<(String, Value)>,
    },

    /// `tuple (v0, v1, ...)`
    Tuple(Vec<Value>),

    /// `array [v0, v1, ...]`
    Array {
        element_ty: Id<Ty>,
        elements: Vec<Value>,
    },

    /// `enum Enum.Variant` or `enum Enum.Variant(payload...)`
    EnumVariant {
        /// The enum type
        enum_ty: Id<Ty>,
        /// The variant name
        variant: String,
        /// Payload values for associated data (empty for simple variants)
        payload: Vec<Value>,
    },

    /// `call func(args...)` with return value
    Call { callee: Callee, args: Vec<CallArg> },

    /// Type cast
    Cast {
        kind: CastKind,
        operand: Value,
        target: Id<Ty>,
    },

    // === String operations ===
    /// `str.ptr <value>`
    StrPtr(Value),
    /// `str.len <value>`
    StrLen(Value),
    /// `str.from_parts <ptr>, <len>`
    StrFromParts { ptr: Value, len: Value },
    /// `int.to_string <value>` - convert integer to string
    IntToString(Value),

    // === Pointer operations ===
    /// `ptr.offset <ptr>, <offset>`
    PtrOffset { ptr: Value, offset: Value },
    /// `ptr.to.ref <value>`
    PtrToRef(Value),
    /// `ptr.to.ref_var <value>`
    PtrToRefMut(Value),
    /// `ref.to.ptr <value>`
    RefToPtr(Value),

    // === Callable operations ===
    /// `func.to.escaping path.to.function`
    FuncToEscaping(Id<QualifiedName>),
    /// `apply partial func(captures...)`
    ApplyPartial {
        func: Id<QualifiedName>,
        captures: Vec<Value>,
    },
}

/// What's being called.
#[derive(Debug, Clone)]
pub enum Callee {
    /// Direct call: `call path.to.func(...)`
    Direct {
        name: Id<QualifiedName>,
        type_args: Vec<Id<Ty>>,
    },

    /// Thin function pointer: `call %fn_ptr(...)`
    Thin(Place),

    /// Thick callable: `call escaping %closure(...)`
    Thick(Place),

    /// Witness method lookup: `call witness_method Protocol.method for Type(...)`
    ///
    /// Used when calling methods on type parameters. The actual implementation
    /// is looked up at monomorphization time from the witness table.
    Witness {
        /// The protocol that defines the method
        protocol: Id<QualifiedName>,
        /// The method name within the protocol
        method: String,
        /// The type parameter we're calling on (e.g., `T`)
        for_type: Id<Ty>,
    },
}

/// Binary operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Integer (signed)
    AddSigned,
    SubSigned,
    MulSigned,
    DivSigned,
    RemSigned,
    // Integer (unsigned)
    AddUnsigned,
    SubUnsigned,
    MulUnsigned,
    DivUnsigned,
    RemUnsigned,
    // Float
    FAdd,
    FSub,
    FMul,
    FDiv,
    // Bitwise
    And,
    Or,
    Xor,
    Shl,
    ShrSigned,
    ShrUnsigned,
    // Comparison (integer)
    Eq,
    Ne,
    LtSigned,
    LeSigned,
    GtSigned,
    GeSigned,
    LtUnsigned,
    LeUnsigned,
    GtUnsigned,
    GeUnsigned,
    // Comparison (float)
    FEq,
    FNe,
    FLt,
    FLe,
    FGt,
    FGe,
    // Boolean
    BoolAnd,
    BoolOr,
}

/// Unary operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// Integer negation
    Neg,
    /// Float negation
    FNeg,
    /// Bitwise not
    Not,
    /// Boolean not
    BoolNot,
}

/// Type cast kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    /// `i64.to.f64`
    IntToFloat,
    /// `f64.to.i64`
    FloatToInt,
    /// `i32.to.i64`
    IntWiden,
    /// `i64.to.i32`
    IntTruncate,
    /// `f32.to.f64`
    FloatWiden,
    /// `f64.to.f32`
    FloatTruncate,
    /// `ptr.bitcast[p[T]]`
    PtrBitcast,
    /// `ref.to.immut`
    RefToImmut,
}

impl Statement {
    /// Create a new assignment statement.
    pub fn assign(dest: Place, rvalue: Rvalue) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            kind: StatementKind::Assign { dest, rvalue },
        }
    }

    /// Create a new call statement (unit return).
    pub fn call(callee: Callee, args: Vec<CallArg>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            kind: StatementKind::Call { callee, args },
        }
    }

    /// Create a display wrapper for printing this statement.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        StatementDisplay { stmt: self, ctx }
    }
}

struct StatementDisplay<'a> {
    stmt: &'a Statement,
    ctx: &'a MirContext,
}

impl fmt::Display for StatementDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.stmt.kind {
            StatementKind::Assign { dest, rvalue } => {
                write!(
                    f,
                    "{} = {}",
                    dest.display(self.ctx),
                    rvalue.display(self.ctx)
                )
            }
            StatementKind::Call { callee, args } => {
                write!(f, "call {}", callee.display(self.ctx))?;
                write!(f, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} {}", arg.mode, arg.value.display(self.ctx))?;
                }
                write!(f, ")")
            }
            StatementKind::Deinit { place } => {
                write!(f, "deinit {}", place.display(self.ctx))
            }
            StatementKind::DeinitIf { place, flag } => {
                let flag_name = &self.ctx.local(*flag).name;
                write!(f, "deinit {} if %{}", place.display(self.ctx), flag_name)
            }
            StatementKind::SetDeinitFlag { flag, value } => {
                let flag_name = &self.ctx.local(*flag).name;
                write!(f, "%{} = {}", flag_name, value)
            }
        }
    }
}

impl Rvalue {
    /// Create a display wrapper for printing this rvalue.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        RvalueDisplay { rvalue: self, ctx }
    }
}

struct RvalueDisplay<'a> {
    rvalue: &'a Rvalue,
    ctx: &'a MirContext,
}

impl fmt::Display for RvalueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.rvalue {
            Rvalue::Move(p) => write!(f, "move {}", p.display(self.ctx)),
            Rvalue::Copy(p) => write!(f, "copy {}", p.display(self.ctx)),
            Rvalue::Ref(p) => write!(f, "ref {}", p.display(self.ctx)),
            Rvalue::RefMut(p) => write!(f, "ref var {}", p.display(self.ctx)),
            Rvalue::Use(i) => write!(f, "{}", i.display(self.ctx)),
            Rvalue::BinaryOp { op, lhs, rhs } => {
                write!(
                    f,
                    "{} {}, {}",
                    op.as_str(),
                    lhs.display(self.ctx),
                    rhs.display(self.ctx)
                )
            }
            Rvalue::UnaryOp { op, operand } => {
                write!(f, "{} {}", op.as_str(), operand.display(self.ctx))
            }
            Rvalue::Construct { ty, fields } => {
                write!(f, "construct {} {{ ", self.ctx.ty(*ty).display(self.ctx))?;
                for (i, (name, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", name, value.display(self.ctx))?;
                }
                write!(f, " }}")
            }
            Rvalue::Call { callee, args } => {
                write!(f, "call {}", callee.display(self.ctx))?;
                write!(f, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} {}", arg.mode, arg.value.display(self.ctx))?;
                }
                write!(f, ")")
            }
            Rvalue::Cast {
                kind,
                operand,
                target,
            } => {
                write!(
                    f,
                    "{} {} -> {}",
                    kind.as_str(),
                    operand.display(self.ctx),
                    self.ctx.ty(*target).display(self.ctx)
                )
            }
            Rvalue::StrPtr(v) => write!(f, "str.ptr {}", v.display(self.ctx)),
            Rvalue::StrLen(v) => write!(f, "str.len {}", v.display(self.ctx)),
            Rvalue::StrFromParts { ptr, len } => {
                write!(
                    f,
                    "str.from_parts {}, {}",
                    ptr.display(self.ctx),
                    len.display(self.ctx)
                )
            }
            Rvalue::IntToString(v) => write!(f, "int.to_string {}", v.display(self.ctx)),
            Rvalue::PtrOffset { ptr, offset } => {
                write!(
                    f,
                    "ptr.offset {}, {}",
                    ptr.display(self.ctx),
                    offset.display(self.ctx)
                )
            }
            Rvalue::PtrToRef(v) => write!(f, "ptr.to.ref {}", v.display(self.ctx)),
            Rvalue::PtrToRefMut(v) => write!(f, "ptr.to.ref_var {}", v.display(self.ctx)),
            Rvalue::RefToPtr(v) => write!(f, "ref.to.ptr {}", v.display(self.ctx)),
            Rvalue::FuncToEscaping(name) => {
                write!(f, "func.to.escaping {}", self.ctx.name(*name))
            }
            Rvalue::ApplyPartial { func, captures } => {
                write!(f, "apply partial {}(", self.ctx.name(*func))?;
                for (i, cap) in captures.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", cap.display(self.ctx))?;
                }
                write!(f, ")")
            }
            Rvalue::Tuple(elements) => {
                write!(f, "tuple (")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem.display(self.ctx))?;
                }
                write!(f, ")")
            }
            Rvalue::Array {
                element_ty,
                elements,
            } => {
                write!(f, "array[{}] [", self.ctx.ty(*element_ty).display(self.ctx))?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem.display(self.ctx))?;
                }
                write!(f, "]")
            }
            Rvalue::EnumVariant {
                enum_ty,
                variant,
                payload,
            } => {
                write!(
                    f,
                    "enum {}.{}",
                    self.ctx.ty(*enum_ty).display(self.ctx),
                    variant
                )?;
                if !payload.is_empty() {
                    write!(f, "(")?;
                    for (i, val) in payload.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", val.display(self.ctx))?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
        }
    }
}

impl Callee {
    /// Create a direct callee.
    pub fn direct(name: Id<QualifiedName>) -> Self {
        Callee::Direct {
            name,
            type_args: Vec::new(),
        }
    }

    /// Create a direct callee with type arguments.
    pub fn direct_generic(name: Id<QualifiedName>, type_args: Vec<Id<Ty>>) -> Self {
        Callee::Direct { name, type_args }
    }

    /// Create a witness method callee.
    ///
    /// Used for calling methods on type parameters where the concrete
    /// implementation is resolved via witness table lookup.
    pub fn witness(
        protocol: Id<QualifiedName>,
        method: impl Into<String>,
        for_type: Id<Ty>,
    ) -> Self {
        Callee::Witness {
            protocol,
            method: method.into(),
            for_type,
        }
    }

    /// Create a display wrapper for printing this callee.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        CalleeDisplay { callee: self, ctx }
    }
}

struct CalleeDisplay<'a> {
    callee: &'a Callee,
    ctx: &'a MirContext,
}

impl fmt::Display for CalleeDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.callee {
            Callee::Direct { name, type_args } => {
                write!(f, "{}", self.ctx.name(*name))?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", self.ctx.ty(*arg).display(self.ctx))?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
            Callee::Thin(p) => write!(f, "{}", p.display(self.ctx)),
            Callee::Thick(p) => write!(f, "escaping {}", p.display(self.ctx)),
            Callee::Witness {
                protocol,
                method,
                for_type,
            } => {
                write!(
                    f,
                    "witness_method {}.{} for {}",
                    self.ctx.name(*protocol),
                    method,
                    self.ctx.ty(*for_type).display(self.ctx)
                )
            }
        }
    }
}

impl BinOp {
    /// Get the string representation for printing.
    pub fn as_str(&self) -> &'static str {
        match self {
            BinOp::AddSigned => "i64.add.signed",
            BinOp::SubSigned => "i64.sub.signed",
            BinOp::MulSigned => "i64.mul.signed",
            BinOp::DivSigned => "i64.div.signed",
            BinOp::RemSigned => "i64.rem.signed",
            BinOp::AddUnsigned => "i64.add.unsigned",
            BinOp::SubUnsigned => "i64.sub.unsigned",
            BinOp::MulUnsigned => "i64.mul.unsigned",
            BinOp::DivUnsigned => "i64.div.unsigned",
            BinOp::RemUnsigned => "i64.rem.unsigned",
            BinOp::FAdd => "f64.add",
            BinOp::FSub => "f64.sub",
            BinOp::FMul => "f64.mul",
            BinOp::FDiv => "f64.div",
            BinOp::And => "i64.and",
            BinOp::Or => "i64.or",
            BinOp::Xor => "i64.xor",
            BinOp::Shl => "i64.shl",
            BinOp::ShrSigned => "i64.shr.signed",
            BinOp::ShrUnsigned => "i64.shr.unsigned",
            BinOp::Eq => "i64.eq",
            BinOp::Ne => "i64.ne",
            BinOp::LtSigned => "i64.lt.signed",
            BinOp::LeSigned => "i64.le.signed",
            BinOp::GtSigned => "i64.gt.signed",
            BinOp::GeSigned => "i64.ge.signed",
            BinOp::LtUnsigned => "i64.lt.unsigned",
            BinOp::LeUnsigned => "i64.le.unsigned",
            BinOp::GtUnsigned => "i64.gt.unsigned",
            BinOp::GeUnsigned => "i64.ge.unsigned",
            BinOp::FEq => "f64.eq",
            BinOp::FNe => "f64.ne",
            BinOp::FLt => "f64.lt",
            BinOp::FLe => "f64.le",
            BinOp::FGt => "f64.gt",
            BinOp::FGe => "f64.ge",
            BinOp::BoolAnd => "bool.and",
            BinOp::BoolOr => "bool.or",
        }
    }
}

impl UnOp {
    /// Get the string representation for printing.
    pub fn as_str(&self) -> &'static str {
        match self {
            UnOp::Neg => "i64.neg",
            UnOp::FNeg => "f64.neg",
            UnOp::Not => "i64.not",
            UnOp::BoolNot => "bool.not",
        }
    }
}

impl CastKind {
    /// Get the string representation for printing.
    pub fn as_str(&self) -> &'static str {
        match self {
            CastKind::IntToFloat => "i64.to.f64",
            CastKind::FloatToInt => "f64.to.i64",
            CastKind::IntWiden => "int.widen",
            CastKind::IntTruncate => "int.truncate",
            CastKind::FloatWiden => "float.widen",
            CastKind::FloatTruncate => "float.truncate",
            CastKind::PtrBitcast => "ptr.bitcast",
            CastKind::RefToImmut => "ref.to.immut",
        }
    }
}
