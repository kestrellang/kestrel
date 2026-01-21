//! Expression data types for the semantic tree.
//!
//! Expressions are plain data structures (not symbols) that represent
//! resolved expressions in function bodies. They are created during
//! the bind phase after path resolution.

use std::sync::atomic::{AtomicU64, Ordering};

use kestrel_span::Span;
use semantic_tree::symbol::SymbolId;

use crate::stmt::{Statement, StatementKind};
use crate::symbol::local::LocalId;
use crate::ty::{IntBits, Substitutions, Ty};

/// Globally unique expression identifier.
/// Every `Expression` instance has a unique `ExprId` assigned at construction.
/// Used by the type inference system to track value resolutions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId(u64);

impl ExprId {
    /// Create a new unique expression ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        ExprId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value (useful for debugging)
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl Default for ExprId {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute the type of a block (statements + optional trailing value).
///
/// This handles:
/// - If there's a trailing value, use its type
/// - If the last statement is an expression statement, use its type
///   (this handles cases like `if a { inner_if }` where inner_if has no semicolon
///   but gets parsed as an expression statement)
/// - Otherwise return Unit
///
/// Note: This is used for computing if branch types, where the "trailing value"
/// distinction matters less than getting the actual type of what the block evaluates to.
pub fn compute_block_type(statements: &[Statement], value: Option<&Expression>, span: &Span) -> Ty {
    // If there's a trailing value, use its type
    if let Some(v) = value {
        return v.ty.clone();
    }

    // Check if the last statement is an expression statement - use its type
    if let Some(last_stmt) = statements.last()
        && let StatementKind::Expr(expr) = &last_stmt.kind
    {
        return expr.ty.clone();
    }

    // Default to Unit
    Ty::unit(span.clone())
}

/// Unique identifier for a loop within a function body.
///
/// Used to track which loop a break/continue refers to.
/// Each while/loop expression gets a unique LoopId when resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoopId(pub u32);

impl LoopId {
    /// Create a new LoopId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A call argument with optional label.
///
/// Supports Swift-style labeled arguments:
/// - `foo(42)` → label = None
/// - `foo(x: 42)` → label = Some("x")
#[derive(Debug, Clone)]
pub struct CallArgument {
    /// Optional label for the argument
    pub label: Option<String>,
    /// The argument value expression
    pub value: Expression,
    /// The span of the entire argument (including label if present)
    pub span: Span,
}

impl CallArgument {
    /// Create an unlabeled argument.
    pub fn unlabeled(value: Expression, span: Span) -> Self {
        Self {
            label: None,
            value,
            span,
        }
    }

    /// Create a labeled argument.
    pub fn labeled(label: String, value: Expression, span: Span) -> Self {
        Self {
            label: Some(label),
            value,
            span,
        }
    }
}

/// Return type category for primitive methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnType {
    /// Returns the same type as the receiver (Int -> Int, Float -> Float)
    SameAsReceiver,
    /// Always returns Bool
    Bool,
    /// Always returns String
    String,
    /// Always returns Int64
    Int,
    /// Returns lang.ptr[I8] (pointer to bytes)
    PointerToI8,
}

/// Macro to define primitive methods with their metadata in one place.
///
/// Each entry specifies:
/// - The enum variant name
/// - The primitive type it applies to (Int, Float, Bool, String)
/// - The method name string
/// - The return type category
macro_rules! define_primitive_methods {
    (
        $(
            $variant:ident {
                ty: $prim_ty:ident,
                name: $name:literal,
                returns: $returns:ident
            }
        ),* $(,)?
    ) => {
        /// Built-in methods on primitive types.
        ///
        /// These methods have no symbol representation - the compiler has
        /// built-in knowledge of their signatures and semantics.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum PrimitiveMethod {
            $(
                $variant,
            )*
        }

        impl PrimitiveMethod {
            /// Get the method name.
            pub fn name(&self) -> &'static str {
                match self {
                    $(
                        PrimitiveMethod::$variant => $name,
                    )*
                }
            }

            /// Get the return type category.
            pub fn return_type_category(&self) -> ReturnType {
                match self {
                    $(
                        PrimitiveMethod::$variant => ReturnType::$returns,
                    )*
                }
            }

            /// Look up a method on an Int type.
            pub fn lookup_int(name: &str) -> Option<PrimitiveMethod> {
                match name {
                    $(
                        $name if stringify!($prim_ty) == "Int" => Some(PrimitiveMethod::$variant),
                    )*
                    _ => None,
                }
            }

            /// Look up a method on a Float type.
            pub fn lookup_float(name: &str) -> Option<PrimitiveMethod> {
                match name {
                    $(
                        $name if stringify!($prim_ty) == "Float" => Some(PrimitiveMethod::$variant),
                    )*
                    _ => None,
                }
            }

            /// Look up a method on a Bool type.
            pub fn lookup_bool(name: &str) -> Option<PrimitiveMethod> {
                match name {
                    $(
                        $name if stringify!($prim_ty) == "Bool" => Some(PrimitiveMethod::$variant),
                    )*
                    _ => None,
                }
            }

            /// Look up a method on a String type.
            pub fn lookup_string(name: &str) -> Option<PrimitiveMethod> {
                match name {
                    $(
                        $name if stringify!($prim_ty) == "String" => Some(PrimitiveMethod::$variant),
                    )*
                    _ => None,
                }
            }
        }
    };
}

define_primitive_methods! {
    // Int methods
    IntToString   { ty: Int, name: "toString", returns: String },
    IntAbs        { ty: Int, name: "abs", returns: SameAsReceiver },

    // Int arithmetic
    IntAdd        { ty: Int, name: "add", returns: SameAsReceiver },
    IntSub        { ty: Int, name: "subtract", returns: SameAsReceiver },
    IntMul        { ty: Int, name: "multiply", returns: SameAsReceiver },
    IntDiv        { ty: Int, name: "divide", returns: SameAsReceiver },
    IntRem        { ty: Int, name: "modulo", returns: SameAsReceiver },
    IntNeg        { ty: Int, name: "negate", returns: SameAsReceiver },

    // Int comparison
    IntEq         { ty: Int, name: "equals", returns: Bool },
    IntNe         { ty: Int, name: "notEquals", returns: Bool },
    IntLt         { ty: Int, name: "lessThan", returns: Bool },
    IntLe         { ty: Int, name: "lessThanOrEqual", returns: Bool },
    IntGt         { ty: Int, name: "greaterThan", returns: Bool },
    IntGe         { ty: Int, name: "greaterThanOrEqual", returns: Bool },

    // Int bitwise
    IntBitAnd     { ty: Int, name: "bitwiseAnd", returns: SameAsReceiver },
    IntBitOr      { ty: Int, name: "bitwiseOr", returns: SameAsReceiver },
    IntBitXor     { ty: Int, name: "bitwiseXor", returns: SameAsReceiver },
    IntBitNot     { ty: Int, name: "bitwiseNot", returns: SameAsReceiver },
    IntShl        { ty: Int, name: "shiftLeft", returns: SameAsReceiver },
    IntShr        { ty: Int, name: "shiftRight", returns: SameAsReceiver },

    // Float arithmetic
    FloatAdd      { ty: Float, name: "add", returns: SameAsReceiver },
    FloatSub      { ty: Float, name: "subtract", returns: SameAsReceiver },
    FloatMul      { ty: Float, name: "multiply", returns: SameAsReceiver },
    FloatDiv      { ty: Float, name: "divide", returns: SameAsReceiver },
    FloatNeg      { ty: Float, name: "negate", returns: SameAsReceiver },

    // Float comparison
    FloatEq       { ty: Float, name: "equals", returns: Bool },
    FloatNe       { ty: Float, name: "notEquals", returns: Bool },
    FloatLt       { ty: Float, name: "lessThan", returns: Bool },
    FloatLe       { ty: Float, name: "lessThanOrEqual", returns: Bool },
    FloatGt       { ty: Float, name: "greaterThan", returns: Bool },
    FloatGe       { ty: Float, name: "greaterThanOrEqual", returns: Bool },

    // Bool operators
    BoolAnd       { ty: Bool, name: "logicalAnd", returns: Bool },
    BoolOr        { ty: Bool, name: "logicalOr", returns: Bool },
    BoolNot       { ty: Bool, name: "logicalNot", returns: Bool },
    BoolEq        { ty: Bool, name: "equals", returns: Bool },
    BoolNe        { ty: Bool, name: "notEquals", returns: Bool },

    // String methods
    StringLength    { ty: String, name: "length", returns: Int },
    StringIsEmpty   { ty: String, name: "isEmpty", returns: Bool },
    StringEq        { ty: String, name: "equals", returns: Bool },
    StringNe        { ty: String, name: "notEquals", returns: Bool },
    StringUnsafePtr { ty: String, name: "unsafePtr", returns: PointerToI8 },
}

impl PrimitiveMethod {
    /// Get the number of arguments this primitive method takes (not counting the receiver).
    pub fn arity(&self) -> usize {
        match self {
            // Unary methods (0 arguments, just receiver)
            PrimitiveMethod::IntToString
            | PrimitiveMethod::IntAbs
            | PrimitiveMethod::IntNeg
            | PrimitiveMethod::IntBitNot
            | PrimitiveMethod::FloatNeg
            | PrimitiveMethod::BoolNot
            | PrimitiveMethod::StringLength
            | PrimitiveMethod::StringIsEmpty
            | PrimitiveMethod::StringUnsafePtr => 0,

            // Binary methods (1 argument besides receiver)
            PrimitiveMethod::IntAdd
            | PrimitiveMethod::IntSub
            | PrimitiveMethod::IntMul
            | PrimitiveMethod::IntDiv
            | PrimitiveMethod::IntRem
            | PrimitiveMethod::IntEq
            | PrimitiveMethod::IntNe
            | PrimitiveMethod::IntLt
            | PrimitiveMethod::IntLe
            | PrimitiveMethod::IntGt
            | PrimitiveMethod::IntGe
            | PrimitiveMethod::IntBitAnd
            | PrimitiveMethod::IntBitOr
            | PrimitiveMethod::IntBitXor
            | PrimitiveMethod::IntShl
            | PrimitiveMethod::IntShr
            | PrimitiveMethod::FloatAdd
            | PrimitiveMethod::FloatSub
            | PrimitiveMethod::FloatMul
            | PrimitiveMethod::FloatDiv
            | PrimitiveMethod::FloatEq
            | PrimitiveMethod::FloatNe
            | PrimitiveMethod::FloatLt
            | PrimitiveMethod::FloatLe
            | PrimitiveMethod::FloatGt
            | PrimitiveMethod::FloatGe
            | PrimitiveMethod::BoolAnd
            | PrimitiveMethod::BoolOr
            | PrimitiveMethod::BoolEq
            | PrimitiveMethod::BoolNe
            | PrimitiveMethod::StringEq
            | PrimitiveMethod::StringNe => 1,
        }
    }

    /// Get the return type of this primitive method.
    pub fn return_type(&self, receiver_ty: &Ty, span: Span) -> Ty {
        use crate::ty::{IntBits, TyKind};
        match self.return_type_category() {
            ReturnType::SameAsReceiver => match receiver_ty.kind() {
                TyKind::Int(bits) => Ty::int(*bits, span),
                TyKind::Float(bits) => Ty::float(*bits, span),
                TyKind::Bool => Ty::bool(span),
                TyKind::String => Ty::string(span),
                // Fallback for error cases
                _ => Ty::error(span),
            },
            ReturnType::Bool => Ty::bool(span),
            ReturnType::String => Ty::string(span),
            ReturnType::Int => Ty::int(IntBits::I64, span),
            ReturnType::PointerToI8 => Ty::pointer(Ty::int(IntBits::I8, span.clone()), span),
        }
    }

    /// Look up a method on a primitive type.
    pub fn lookup(ty: &Ty, name: &str) -> Option<PrimitiveMethod> {
        use crate::ty::TyKind;
        match ty.kind() {
            TyKind::Int(_) => Self::lookup_int(name),
            TyKind::Float(_) => Self::lookup_float(name),
            TyKind::Bool => Self::lookup_bool(name),
            TyKind::String => Self::lookup_string(name),
            _ => None,
        }
    }
}

/// Represents a literal value in an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    /// Unit literal: `()`
    Unit,
    /// Integer literal: `42`, `0xFF`, `0b1010`, `0o17`
    Integer(i64),
    /// Float literal: `3.14`, `1.0e10`
    Float(f64),
    /// String literal: `"hello"`
    String(String),
    /// Boolean literal: `true`, `false`
    Bool(bool),
}

/// Primitive types available in the `lang` namespace.
/// These are the low-level types that map directly to machine types.
/// Signedness is determined by operations, not types (2's complement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LangPrimitive {
    /// 1-bit integer (boolean)
    I1,
    /// 8-bit integer
    I8,
    /// 16-bit integer
    I16,
    /// 32-bit integer
    I32,
    /// 64-bit integer
    I64,
    /// 16-bit floating point
    F16,
    /// 32-bit floating point
    F32,
    /// 64-bit floating point
    F64,
}

impl LangPrimitive {
    /// Parse a primitive type from a string (e.g., "i32", "f64").
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "i1" => Some(LangPrimitive::I1),
            "i8" => Some(LangPrimitive::I8),
            "i16" => Some(LangPrimitive::I16),
            "i32" => Some(LangPrimitive::I32),
            "i64" => Some(LangPrimitive::I64),
            "f16" => Some(LangPrimitive::F16),
            "f32" => Some(LangPrimitive::F32),
            "f64" => Some(LangPrimitive::F64),
            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            LangPrimitive::I1 => "i1",
            LangPrimitive::I8 => "i8",
            LangPrimitive::I16 => "i16",
            LangPrimitive::I32 => "i32",
            LangPrimitive::I64 => "i64",
            LangPrimitive::F16 => "f16",
            LangPrimitive::F32 => "f32",
            LangPrimitive::F64 => "f64",
        }
    }

    /// Check if this is an integer type (includes i1 for booleans).
    pub fn is_int(&self) -> bool {
        matches!(
            self,
            LangPrimitive::I1
                | LangPrimitive::I8
                | LangPrimitive::I16
                | LangPrimitive::I32
                | LangPrimitive::I64
        )
    }

    /// Check if this is a floating point type.
    pub fn is_float(&self) -> bool {
        matches!(
            self,
            LangPrimitive::F16 | LangPrimitive::F32 | LangPrimitive::F64
        )
    }

    /// Get the bit width of this primitive type.
    pub fn bit_width(&self) -> u32 {
        match self {
            LangPrimitive::I1 => 1,
            LangPrimitive::I8 => 8,
            LangPrimitive::I16 => 16,
            LangPrimitive::F16 => 16,
            LangPrimitive::I32 | LangPrimitive::F32 => 32,
            LangPrimitive::I64 | LangPrimitive::F64 => 64,
        }
    }

    /// Convert this primitive to a semantic type.
    pub fn to_ty(&self, span: Span) -> crate::ty::Ty {
        use crate::ty::{FloatBits, IntBits, Ty};
        match self {
            LangPrimitive::I1 => Ty::bool(span),
            LangPrimitive::I8 => Ty::int(IntBits::I8, span),
            LangPrimitive::I16 => Ty::int(IntBits::I16, span),
            LangPrimitive::I32 => Ty::int(IntBits::I32, span),
            LangPrimitive::I64 => Ty::int(IntBits::I64, span),
            LangPrimitive::F16 => Ty::float(FloatBits::F16, span),
            LangPrimitive::F32 => Ty::float(FloatBits::F32, span),
            LangPrimitive::F64 => Ty::float(FloatBits::F64, span),
        }
    }
}

/// Integer binary operations (signedness-agnostic, 2's complement).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntBinaryOp {
    /// Addition
    Add,
    /// Subtraction
    Sub,
    /// Multiplication
    Mul,
    /// Equality comparison
    Eq,
    /// Inequality comparison
    Ne,
    /// Bitwise AND
    And,
    /// Bitwise OR
    Or,
    /// Bitwise XOR
    Xor,
    /// Left shift
    Shl,
}

/// Signed/unsigned-specific integer operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedOp {
    /// Division (signed or unsigned)
    Div,
    /// Remainder (signed or unsigned)
    Rem,
    /// Right shift (arithmetic for signed, logical for unsigned)
    Shr,
    /// Less than
    Lt,
    /// Less than or equal
    Le,
    /// Greater than
    Gt,
    /// Greater than or equal
    Ge,
}

/// Integer unary operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntUnaryOp {
    /// Negation (2's complement)
    Neg,
    /// Bitwise NOT
    Not,
}

/// Float binary operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Float unary operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatUnaryOp {
    Neg,
}

/// Float constants (arity 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatConstant {
    /// Positive infinity
    Infinity,
    /// NaN (Not a Number)
    Nan,
}

/// Float predicates (arity 1, returns bool).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatPredicate {
    /// Check if value is NaN
    IsNan,
    /// Check if value is infinite
    IsInfinite,
}

/// Float math operations (unary, arity 1).
/// Only includes operations supported natively by Cranelift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatMathOp {
    Floor,
    Ceil,
    Round,
    Trunc,
    Sqrt,
}

/// Language intrinsics available in the `lang` namespace.
/// These are compiler-provided functions that lower to special MIR constructs.
#[derive(Debug, Clone)]
pub enum LangIntrinsic {
    /// `lang.panic_unwind(message: String) -> Never`
    /// Terminates the program with the given panic message.
    PanicUnwind,

    /// `lang.cast_<from>_<to>(value: From) -> To`
    /// Type cast between primitive types.
    Cast {
        from: LangPrimitive,
        to: LangPrimitive,
    },

    /// Integer binary operations (signedness-agnostic).
    /// e.g., `lang.i64_add(a, b)`, `lang.i32_mul(a, b)`
    IntBinary {
        primitive: LangPrimitive,
        op: IntBinaryOp,
    },

    /// Integer binary operations (signed).
    /// e.g., `lang.i64_signed_div(a, b)`, `lang.i32_signed_lt(a, b)`
    IntBinarySigned {
        primitive: LangPrimitive,
        op: SignedOp,
    },

    /// Integer binary operations (unsigned).
    /// e.g., `lang.i64_unsigned_div(a, b)`, `lang.i32_unsigned_lt(a, b)`
    IntBinaryUnsigned {
        primitive: LangPrimitive,
        op: SignedOp,
    },

    /// Integer unary operations.
    /// e.g., `lang.i64_neg(a)`, `lang.i32_not(a)`
    IntUnary {
        primitive: LangPrimitive,
        op: IntUnaryOp,
    },

    /// Float binary operations.
    /// e.g., `lang.f64_add(a, b)`, `lang.f32_lt(a, b)`
    FloatBinary {
        primitive: LangPrimitive,
        op: FloatBinaryOp,
    },

    /// Float unary operations.
    /// e.g., `lang.f64_neg(a)`
    FloatUnary {
        primitive: LangPrimitive,
        op: FloatUnaryOp,
    },

    /// Float constants (arity 0).
    /// e.g., `lang.f64_infinity()`, `lang.f32_nan()`
    FloatConst {
        primitive: LangPrimitive,
        constant: FloatConstant,
    },

    /// Float predicates (arity 1, returns bool).
    /// e.g., `lang.f64_is_nan(x)`, `lang.f32_is_infinite(x)`
    FloatPred {
        primitive: LangPrimitive,
        pred: FloatPredicate,
    },

    /// Float math operations (unary, arity 1).
    /// e.g., `lang.f64_floor(x)`, `lang.f32_sqrt(x)`
    FloatMath {
        primitive: LangPrimitive,
        op: FloatMathOp,
    },

    // === Pointer intrinsics ===
    /// `lang.ptr_null[T]()` - Create null pointer of type T
    PtrNull { pointee_ty: Ty },

    /// `lang.ptr_from_address[T](addr: UInt)` - Create pointer from address
    PtrFromAddress { pointee_ty: Ty },

    /// `lang.ptr_to_address(ptr: lang.ptr[T])` - Get address as UInt
    PtrToAddress,

    /// `lang.ptr_to[T](value: T)` - Create pointer to value (stack alloc)
    PtrTo { pointee_ty: Ty },

    /// `lang.ptr_read[T](ptr: lang.ptr[T])` - Dereference pointer
    PtrRead { pointee_ty: Ty },

    /// `lang.ptr_write[T](ptr: lang.ptr[T], value: T)` - Write through pointer
    PtrWrite { pointee_ty: Ty },

    /// `lang.ptr_offset(ptr: lang.ptr[T], offset: Int)` - Byte offset
    PtrOffset,

    /// `lang.ptr_is_null(ptr: lang.ptr[T])` - Null check
    PtrIsNull,

    /// `lang.cast_ptr[T](ptr: lang.ptr[U])` - Cast pointer to type T
    CastPtr { target_ty: Ty },

    /// `lang.sizeof[T]()` - Size of type T in bytes
    SizeOf { ty: Ty },

    /// `lang.alignof[T]()` - Alignment of type T in bytes
    AlignOf { ty: Ty },

    // === Boolean (i1) intrinsics ===
    /// `lang.i1_eq(a, b)` - Boolean equality
    I1Eq,

    /// `lang.i1_and(a, b)` - Boolean AND
    I1And,

    /// `lang.i1_or(a, b)` - Boolean OR
    I1Or,

    /// `lang.i1_not(a)` - Boolean NOT
    I1Not,

    // === Atomic intrinsics ===
    /// `lang.atomic_add(place, delta)` - Atomic fetch-add, returns old value
    AtomicAdd,

    /// `lang.atomic_sub(place, delta)` - Atomic fetch-sub, returns old value
    AtomicSub,
}

impl LangIntrinsic {
    /// Get the number of arguments this intrinsic expects.
    pub fn arity(&self) -> usize {
        match self {
            LangIntrinsic::PanicUnwind => 1,
            LangIntrinsic::Cast { .. } => 1,
            LangIntrinsic::IntBinary { .. } => 2,
            LangIntrinsic::IntBinarySigned { .. } => 2,
            LangIntrinsic::IntBinaryUnsigned { .. } => 2,
            LangIntrinsic::IntUnary { .. } => 1,
            LangIntrinsic::FloatBinary { .. } => 2,
            LangIntrinsic::FloatUnary { .. } => 1,
            LangIntrinsic::FloatConst { .. } => 0,
            LangIntrinsic::FloatPred { .. } => 1,
            LangIntrinsic::FloatMath { .. } => 1,
            // Pointer intrinsics
            LangIntrinsic::PtrNull { .. } => 0,
            LangIntrinsic::PtrFromAddress { .. } => 1,
            LangIntrinsic::PtrToAddress => 1,
            LangIntrinsic::PtrTo { .. } => 1,
            LangIntrinsic::PtrRead { .. } => 1,
            LangIntrinsic::PtrWrite { .. } => 2,
            LangIntrinsic::PtrOffset => 2,
            LangIntrinsic::PtrIsNull => 1,
            LangIntrinsic::CastPtr { .. } => 1,
            LangIntrinsic::SizeOf { .. } => 0,
            LangIntrinsic::AlignOf { .. } => 0,
            // Boolean (i1) intrinsics
            LangIntrinsic::I1Eq => 2,
            LangIntrinsic::I1And => 2,
            LangIntrinsic::I1Or => 2,
            LangIntrinsic::I1Not => 1,
            // Atomic intrinsics
            LangIntrinsic::AtomicAdd => 2,
            LangIntrinsic::AtomicSub => 2,
        }
    }

    /// Get a display name for error messages.
    pub fn name(&self) -> String {
        match self {
            LangIntrinsic::PanicUnwind => "lang.panic_unwind".to_string(),
            LangIntrinsic::Cast { from, to } => {
                format!("lang.cast_{}_{}", from.as_str(), to.as_str())
            },
            LangIntrinsic::IntBinary { primitive, op } => {
                let op_str = match op {
                    IntBinaryOp::Add => "add",
                    IntBinaryOp::Sub => "sub",
                    IntBinaryOp::Mul => "mul",
                    IntBinaryOp::Eq => "eq",
                    IntBinaryOp::Ne => "ne",
                    IntBinaryOp::And => "and",
                    IntBinaryOp::Or => "or",
                    IntBinaryOp::Xor => "xor",
                    IntBinaryOp::Shl => "shl",
                };
                format!("lang.{}_{}", primitive.as_str(), op_str)
            },
            LangIntrinsic::IntBinarySigned { primitive, op } => {
                let op_str = match op {
                    SignedOp::Div => "signed_div",
                    SignedOp::Rem => "signed_rem",
                    SignedOp::Shr => "signed_shr",
                    SignedOp::Lt => "signed_lt",
                    SignedOp::Le => "signed_le",
                    SignedOp::Gt => "signed_gt",
                    SignedOp::Ge => "signed_ge",
                };
                format!("lang.{}_{}", primitive.as_str(), op_str)
            },
            LangIntrinsic::IntBinaryUnsigned { primitive, op } => {
                let op_str = match op {
                    SignedOp::Div => "unsigned_div",
                    SignedOp::Rem => "unsigned_rem",
                    SignedOp::Shr => "unsigned_shr",
                    SignedOp::Lt => "unsigned_lt",
                    SignedOp::Le => "unsigned_le",
                    SignedOp::Gt => "unsigned_gt",
                    SignedOp::Ge => "unsigned_ge",
                };
                format!("lang.{}_{}", primitive.as_str(), op_str)
            },
            LangIntrinsic::IntUnary { primitive, op } => {
                let op_str = match op {
                    IntUnaryOp::Neg => "neg",
                    IntUnaryOp::Not => "not",
                };
                format!("lang.{}_{}", primitive.as_str(), op_str)
            },
            LangIntrinsic::FloatBinary { primitive, op } => {
                let op_str = match op {
                    FloatBinaryOp::Add => "add",
                    FloatBinaryOp::Sub => "sub",
                    FloatBinaryOp::Mul => "mul",
                    FloatBinaryOp::Div => "div",
                    FloatBinaryOp::Eq => "eq",
                    FloatBinaryOp::Ne => "ne",
                    FloatBinaryOp::Lt => "lt",
                    FloatBinaryOp::Le => "le",
                    FloatBinaryOp::Gt => "gt",
                    FloatBinaryOp::Ge => "ge",
                };
                format!("lang.{}_{}", primitive.as_str(), op_str)
            },
            LangIntrinsic::FloatUnary { primitive, op } => {
                let op_str = match op {
                    FloatUnaryOp::Neg => "neg",
                };
                format!("lang.{}_{}", primitive.as_str(), op_str)
            },
            LangIntrinsic::FloatConst {
                primitive,
                constant,
            } => {
                let const_str = match constant {
                    FloatConstant::Infinity => "infinity",
                    FloatConstant::Nan => "nan",
                };
                format!("lang.{}_{}", primitive.as_str(), const_str)
            },
            LangIntrinsic::FloatPred { primitive, pred } => {
                let pred_str = match pred {
                    FloatPredicate::IsNan => "is_nan",
                    FloatPredicate::IsInfinite => "is_infinite",
                };
                format!("lang.{}_{}", primitive.as_str(), pred_str)
            },
            LangIntrinsic::FloatMath { primitive, op } => {
                let op_str = match op {
                    FloatMathOp::Floor => "floor",
                    FloatMathOp::Ceil => "ceil",
                    FloatMathOp::Round => "round",
                    FloatMathOp::Trunc => "trunc",
                    FloatMathOp::Sqrt => "sqrt",
                };
                format!("lang.{}_{}", primitive.as_str(), op_str)
            },
            // Pointer intrinsics
            LangIntrinsic::PtrNull { .. } => "lang.ptr_null".to_string(),
            LangIntrinsic::PtrFromAddress { .. } => "lang.ptr_from_address".to_string(),
            LangIntrinsic::PtrToAddress => "lang.ptr_to_address".to_string(),
            LangIntrinsic::PtrTo { .. } => "lang.ptr_to".to_string(),
            LangIntrinsic::PtrRead { .. } => "lang.ptr_read".to_string(),
            LangIntrinsic::PtrWrite { .. } => "lang.ptr_write".to_string(),
            LangIntrinsic::PtrOffset => "lang.ptr_offset".to_string(),
            LangIntrinsic::PtrIsNull => "lang.ptr_is_null".to_string(),
            LangIntrinsic::CastPtr { .. } => "lang.cast_ptr".to_string(),
            LangIntrinsic::SizeOf { .. } => "lang.sizeof".to_string(),
            LangIntrinsic::AlignOf { .. } => "lang.alignof".to_string(),
            // Boolean (i1) intrinsics
            LangIntrinsic::I1Eq => "lang.i1_eq".to_string(),
            LangIntrinsic::I1And => "lang.i1_and".to_string(),
            LangIntrinsic::I1Or => "lang.i1_or".to_string(),
            LangIntrinsic::I1Not => "lang.i1_not".to_string(),
            // Atomic intrinsics
            LangIntrinsic::AtomicAdd => "lang.atomic_add".to_string(),
            LangIntrinsic::AtomicSub => "lang.atomic_sub".to_string(),
        }
    }

    /// Check if this intrinsic returns a boolean (i1).
    pub fn returns_bool(&self) -> bool {
        match self {
            LangIntrinsic::IntBinary { op, .. } => {
                matches!(op, IntBinaryOp::Eq | IntBinaryOp::Ne)
            },
            LangIntrinsic::IntBinarySigned { op, .. }
            | LangIntrinsic::IntBinaryUnsigned { op, .. } => {
                matches!(
                    op,
                    SignedOp::Lt | SignedOp::Le | SignedOp::Gt | SignedOp::Ge
                )
            },
            LangIntrinsic::FloatBinary { op, .. } => {
                matches!(
                    op,
                    FloatBinaryOp::Eq
                        | FloatBinaryOp::Ne
                        | FloatBinaryOp::Lt
                        | FloatBinaryOp::Le
                        | FloatBinaryOp::Gt
                        | FloatBinaryOp::Ge
                )
            },
            // FloatPred always returns bool (is_nan, is_infinite)
            LangIntrinsic::FloatPred { .. } => true,
            // PtrIsNull returns bool
            LangIntrinsic::PtrIsNull => true,
            // Boolean (i1) intrinsics all return bool
            LangIntrinsic::I1Eq
            | LangIntrinsic::I1And
            | LangIntrinsic::I1Or
            | LangIntrinsic::I1Not => true,
            _ => false,
        }
    }
}

/// Represents the kind of expression.
///
/// All variants represent resolved expressions - there is no `Path` variant
/// because expressions are only created after path resolution during bind phase.
#[derive(Debug, Clone)]
pub enum ExprKind {
    // Literals
    /// Literal expression (integer, float, string, bool, unit)
    Literal(LiteralValue),
    /// Array literal: `[1, 2, 3]`
    Array(Vec<Expression>),
    /// Tuple literal: `(1, 2, 3)`
    Tuple(Vec<Expression>),
    /// Grouping expression: `(expr)`
    Grouping(Box<Expression>),

    // Resolved references
    /// Reference to a local variable (resolved from a path).
    /// The LocalId references the function's locals vector.
    LocalRef(LocalId),
    /// Reference to a symbol with ValueBehavior (resolved from a path).
    /// Used for module-level functions, fields, globals, etc.
    SymbolRef(SymbolId),
    /// Overloaded function reference (pending call resolution).
    /// Stores candidates that will be disambiguated by call arguments.
    OverloadedRef(Vec<SymbolId>),
    /// Reference to a type (struct, protocol, etc.) - used in call expressions.
    /// When calling a type name like `Point(x: 1, y: 2)`, this represents the struct.
    TypeRef(SymbolId),
    /// Reference to a type parameter for static method/init calls.
    /// When calling `T()` or `T.create()` where T is a type parameter.
    /// Stores the SymbolId of the TypeParameterSymbol.
    TypeParameterRef(SymbolId),
    /// Reference to a qualified associated type for static member access.
    /// e.g., `T.Next` where `Next` is an associated type in T's protocol bounds.
    /// The actual type (`Ty::qualified_associated_type`) is stored in the expression's `ty` field.
    /// This is used for chained associated type access like `T.Next.Next.staticMethod()`.
    AssociatedTypeRef,

    // Member access
    /// Field access: `obj.field`
    /// The object expression and the field name.
    FieldAccess {
        object: Box<Expression>,
        field: String,
    },

    /// Tuple index: `tuple.0`, `tuple.1`
    /// Accesses an element of a tuple by its position.
    TupleIndex {
        tuple: Box<Expression>,
        index: usize,
    },

    // Method references
    /// Method reference: `receiver.method`
    /// Represents a method lookup on a receiver before being called.
    /// The candidates list may have multiple entries for overloaded methods.
    MethodRef {
        receiver: Box<Expression>,
        candidates: Vec<SymbolId>,
        method_name: String,
    },

    // Calls
    /// Function or method call: `foo(1, 2)` or `obj.method(1, 2)`
    /// The callee can be SymbolRef, MethodRef, OverloadedRef, or any callable expression.
    Call {
        callee: Box<Expression>,
        arguments: Vec<CallArgument>,
        /// Type argument substitutions for generic calls (e.g., `identity[Int](42)`).
        /// Maps the callee's type parameters to concrete types.
        /// Empty for non-generic calls.
        substitutions: Substitutions,
    },

    /// Subscript call: `array(0)`, `dict(key: "foo")`, `buffer(unchecked: 5)`
    /// Calls a subscript getter on a value. The receiver is the value being subscripted.
    SubscriptCall {
        /// The receiver expression (the value being subscripted)
        receiver: Box<Expression>,
        /// The subscript getter function to call
        getter: SymbolId,
        /// Arguments to the subscript (e.g., index, key)
        arguments: Vec<CallArgument>,
    },

    /// Primitive method call: `5.toString()`, `"hello".length()`
    /// No symbol exists - compiler has built-in knowledge.
    PrimitiveMethodCall {
        receiver: Box<Expression>,
        method: PrimitiveMethod,
        arguments: Vec<CallArgument>,
    },

    /// Primitive method reference: `5.toString`, `"hello".length` (not yet called).
    /// This is created when a primitive method is accessed but not immediately called.
    /// Primitive methods cannot be used as first-class values, so if this expression
    /// is not immediately called, an error will be emitted during call resolution.
    PrimitiveMethodRef {
        receiver: Box<Expression>,
        method: PrimitiveMethod,
    },

    /// Deferred method call: method call on a receiver with inferred type.
    /// Created when the receiver's type is `Infer` and method resolution must be
    /// deferred until type inference resolves the receiver's actual type.
    /// Type inference will resolve this to a concrete method call.
    DeferredMethodCall {
        receiver: Box<Expression>,
        method_name: String,
        arguments: Vec<CallArgument>,
    },

    /// Deferred static method call on a type that may contain inference variables.
    /// Used for `try` expressions where we need to call `R.fromResidual(early)`
    /// and R is the function's return type (which may have inference variables).
    ///
    /// Type inference will resolve this when the target type becomes concrete.
    DeferredStaticCall {
        /// The type to call the static method on (may contain inference variables)
        target_ty: Ty,
        /// The static method name
        method_name: String,
        /// Arguments to the method call
        arguments: Vec<CallArgument>,
    },

    /// Implicit struct initialization: `Point(x: 1, y: 2)` when no explicit init exists.
    /// The compiler generates a memberwise initializer that assigns each argument to
    /// the corresponding field in declaration order.
    ///
    /// This is used when:
    /// - The struct has no explicit initializers
    /// - All fields are visible from the call site
    ImplicitStructInit {
        /// The struct type being initialized
        struct_type: Ty,
        /// Arguments matching struct fields in declaration order
        arguments: Vec<CallArgument>,
    },

    /// Delegating initializer call: `self.init(...)` from within an initializer.
    /// Calls another initializer on the same struct, passing the current `self`.
    ///
    /// This is used when:
    /// - Inside an initializer body
    /// - Calling another initializer with `self.init(...)`
    ///
    /// After this call, all fields are considered initialized by the delegated initializer.
    DelegatingInit {
        /// The initializer being called
        initializer: SymbolId,
        /// Arguments to the initializer
        arguments: Vec<CallArgument>,
        /// Type argument substitutions for generic structs
        substitutions: Substitutions,
    },

    /// Assignment expression: target = value
    /// Type is Never (assignment doesn't produce a usable value)
    Assignment {
        target: Box<Expression>,
        value: Box<Expression>,
    },

    /// If expression: `if condition { then } else { else }`
    /// Also supports if-let: `if let pattern = expr { then } else { else }`
    /// And if-let chains: `if let .Some(x) = a, let .Some(y) = b { ... }`
    ///
    /// Type is:
    /// - `()` if there's no else branch
    /// - The common type of both branches if there's an else branch
    ///   (type checking deferred - currently just uses then_branch type)
    If {
        /// List of conditions (at least one). Each is either a boolean expression
        /// or a let-binding pattern match.
        conditions: Vec<IfCondition>,
        then_branch: Vec<crate::stmt::Statement>,
        /// Optional trailing expression in then branch (determines its value)
        then_value: Option<Box<Expression>>,
        /// Optional else branch - can be statements + optional value, or another if
        else_branch: Option<ElseBranch>,
    },

    /// While loop expression: `label: while condition { body }`
    ///
    /// Type is `()` (unit) - while loops never produce a value.
    While {
        /// Unique identifier for this loop (for break/continue resolution)
        loop_id: LoopId,
        /// Optional label for named break/continue
        label: Option<LabelInfo>,
        /// The condition expression (must be Bool)
        condition: Box<Expression>,
        /// Statements in the loop body
        body: Vec<crate::stmt::Statement>,
    },

    /// While-let loop expression: `label: while let pattern = expr, ... { body }`
    ///
    /// Loops while all conditions are true (patterns match and bool conditions are true).
    /// Supports chains: `while let .Some(x) = a, let .Some(y) = b, x > 0 { ... }`
    /// Pattern bindings are visible in subsequent conditions and the loop body.
    /// Type is `()` (unit) - while-let loops never produce a value.
    WhileLet {
        /// Unique identifier for this loop (for break/continue resolution)
        loop_id: LoopId,
        /// Optional label for named break/continue
        label: Option<LabelInfo>,
        /// The conditions to check (at least one must be a Let condition)
        conditions: Vec<IfCondition>,
        /// Statements in the loop body
        body: Vec<crate::stmt::Statement>,
    },

    /// Infinite loop expression: `label: loop { body }`
    ///
    /// Type is `()` (unit) when it has a break, or Never if it loops forever.
    /// Currently always typed as `()` for simplicity.
    Loop {
        /// Unique identifier for this loop (for break/continue resolution)
        loop_id: LoopId,
        /// Optional label for named break/continue
        label: Option<LabelInfo>,
        /// Statements in the loop body
        body: Vec<crate::stmt::Statement>,
    },

    /// Break expression: `break` or `break label`
    ///
    /// Type is `Never` - control transfers out of the loop.
    Break {
        /// The target loop to break from (resolved from label or innermost)
        loop_id: LoopId,
        /// Original label (for diagnostics), None if unlabeled
        label: Option<LabelInfo>,
    },

    /// Continue expression: `continue` or `continue label`
    ///
    /// Type is `Never` - control transfers to the loop condition.
    Continue {
        /// The target loop to continue (resolved from label or innermost)
        loop_id: LoopId,
        /// Original label (for diagnostics), None if unlabeled
        label: Option<LabelInfo>,
    },

    /// Return expression: `return` or `return expr`
    ///
    /// Type is `Never` - control transfers out of the function.
    Return {
        /// The optional value to return (None means return unit)
        value: Option<Box<Expression>>,
    },

    /// Closure expression: `{ params in body }` or `{ body }`
    ///
    /// Closures are anonymous functions that can capture variables from their enclosing scope.
    /// Type is a function type based on parameters and return type.
    Closure {
        /// Explicit parameters, if any. None means implicit `it` style.
        params: Option<Vec<ClosureParam>>,
        /// Statements in the closure body
        body: Vec<crate::stmt::Statement>,
        /// Final expression (implicit return value)
        tail_expr: Option<Box<Expression>>,
        /// Variables captured from enclosing scope (filled by capture analysis)
        captures: Vec<Capture>,
        /// Whether `it` was actually referenced in the body (only meaningful when params is None)
        uses_it: bool,
        /// The implicit `it` parameter, if present (when params is None)
        implicit_param: Option<(LocalId, Ty, Span)>,
    },

    /// Reference to a resolved enum case (simple case without arguments).
    /// Used for enum cases like `Option.None` or `.None`.
    EnumCase {
        /// The symbol ID of the enum case
        case_id: SymbolId,
    },

    /// Unresolved implicit member access: `.Case` or `.Case(args)`.
    /// Type inference resolves this to EnumCase or validates against expected type.
    /// Used for Swift-style shorthand enum syntax.
    ImplicitMemberAccess {
        /// The name of the member (case) being accessed
        member_name: String,
        /// Optional arguments for associated values
        arguments: Option<Vec<CallArgument>>,
    },

    /// Match expression: `match scrutinee { pattern => body, ... }`
    ///
    /// Type is the common type of all arm bodies.
    /// Must be exhaustive - all possible values of the scrutinee must be covered.
    Match {
        /// The expression being matched against
        scrutinee: Box<Expression>,
        /// The match arms (pattern => body pairs)
        arms: Vec<MatchArm>,
    },

    /// Block expression: `{ statements; value }`
    ///
    /// Used for match arm bodies that contain statements.
    /// NOT a closure - does not capture variables, has no parameters.
    /// Pattern bindings from the match arm remain visible in the block.
    Block {
        /// Statements in the block
        statements: Vec<crate::stmt::Statement>,
        /// Optional trailing expression (the block's value)
        value: Option<Box<Expression>>,
    },

    /// Error expression (poison value).
    /// Used when expression resolution fails - prevents cascading errors.
    Error,

    /// Language intrinsic call.
    /// These are special `lang.*` functions that are handled directly by the compiler
    /// rather than being real function calls.
    LangIntrinsic {
        /// The intrinsic being called
        intrinsic: LangIntrinsic,
        /// Arguments to the intrinsic
        arguments: Vec<CallArgument>,
    },

    /// Reference to a language intrinsic function (before being called).
    /// This is similar to SymbolRef but for intrinsics that don't have real symbols.
    LangIntrinsicRef(LangIntrinsic),
}

/// A closure parameter.
#[derive(Debug, Clone)]
pub struct ClosureParam {
    /// Parameter name
    pub name: String,
    /// Parameter type (may be inferred initially)
    pub ty: Ty,
    /// Whether the type was explicitly annotated
    pub is_type_annotated: bool,
    /// Source span
    pub span: Span,
    /// The local variable ID for this parameter
    pub local_id: LocalId,
}

/// A captured variable from an enclosing scope.
#[derive(Debug, Clone)]
pub struct Capture {
    /// The local variable ID being captured
    pub local_id: LocalId,
    /// Name of the captured variable
    pub name: String,
    /// Type of the captured variable
    pub ty: Ty,
    /// How the variable is captured
    pub kind: CaptureKind,
    /// Span where the capture occurs
    pub span: Span,
}

/// How a variable is captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    /// Immutable copy (only option currently)
    Value,
}

/// Information about a loop label.
#[derive(Debug, Clone)]
pub struct LabelInfo {
    /// The label name
    pub name: String,
    /// The span of the label in source
    pub span: Span,
}

/// Represents an else branch of an if expression.
#[derive(Debug, Clone)]
pub enum ElseBranch {
    /// else { statements; value }
    Block {
        statements: Vec<crate::stmt::Statement>,
        value: Option<Box<Expression>>,
    },
    /// else if condition { ... } else { ... }
    ElseIf(Box<Expression>),
}

/// A single condition in an if or if-let expression.
///
/// In an if-let chain like `if let .Some(x) = a, let .Some(y) = b, x > 0 { ... }`,
/// each part is an IfCondition.
#[derive(Debug, Clone)]
pub enum IfCondition {
    /// Boolean expression condition: `x > 0`
    Expr(Expression),
    /// Let binding condition: `let pattern = expr`
    /// Pattern bindings are visible in subsequent conditions and the then-branch.
    Let {
        /// The pattern to match against
        pattern: crate::pattern::Pattern,
        /// The expression to match the pattern against (the scrutinee)
        value: Expression,
        /// The span of the entire condition
        span: Span,
    },
}

/// A match arm in a match expression.
///
/// Represents `pattern [if guard] => body` in a match expression.
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// The pattern to match against
    pub pattern: crate::pattern::Pattern,
    /// Optional guard condition (the `if expr` part)
    pub guard: Option<Expression>,
    /// The body expression to evaluate if the pattern matches
    pub body: Expression,
    /// The span of the entire arm
    pub span: Span,
}

impl MatchArm {
    /// Create a new match arm without a guard.
    pub fn new(pattern: crate::pattern::Pattern, body: Expression, span: Span) -> Self {
        MatchArm {
            pattern,
            guard: None,
            body,
            span,
        }
    }

    /// Create a new match arm with a guard.
    pub fn with_guard(
        pattern: crate::pattern::Pattern,
        guard: Expression,
        body: Expression,
        span: Span,
    ) -> Self {
        MatchArm {
            pattern,
            guard: Some(guard),
            body,
            span,
        }
    }
}

impl ElseBranch {
    /// Get the type of this else branch.
    ///
    /// For a block:
    /// - If there's a trailing value, returns its type
    /// - If the last statement is an expression with Never type, returns Never
    /// - Otherwise returns Unit
    ///
    /// For an else-if, returns the type of the nested if expression.
    pub fn ty(&self, span: &Span) -> Ty {
        match self {
            ElseBranch::Block { statements, value } => {
                compute_block_type(statements, value.as_ref().map(|v| v.as_ref()), span)
            },
            ElseBranch::ElseIf(if_expr) => if_expr.ty.clone(),
        }
    }
}

/// A resolved expression in the semantic tree.
///
/// Unlike symbols, expressions are plain data structures without SymbolId.
/// They are created during the bind phase after path resolution.
/// Every expression has a unique `ExprId` for use in type inference.
#[derive(Debug, Clone)]
pub struct Expression {
    /// Unique identifier for this expression
    pub id: ExprId,
    /// The kind of expression
    pub kind: ExprKind,
    /// The resolved type of this expression
    pub ty: Ty,
    /// The source span of this expression
    pub span: Span,
    /// Whether this expression refers to a mutable location (lvalue).
    ///
    /// This is true for:
    /// - LocalRef to a `var` variable
    /// - FieldAccess where the field is `var` AND the object is mutable
    /// - SymbolRef to a mutable module-level variable
    ///
    /// This is false for:
    /// - All literals
    /// - Call expressions (return values are not lvalues)
    /// - LocalRef to a `let` variable
    /// - FieldAccess where the field is `let` OR the object is immutable
    pub mutable: bool,
}

impl Expression {
    /// Create a new expression with explicit mutability.
    /// A fresh `ExprId` is automatically assigned.
    pub fn new(kind: ExprKind, ty: Ty, span: Span, mutable: bool) -> Self {
        Expression {
            id: ExprId::new(),
            kind,
            ty,
            span,
            mutable,
        }
    }

    /// Create a new immutable expression (convenience for most cases).
    /// A fresh `ExprId` is automatically assigned.
    pub fn new_immutable(kind: ExprKind, ty: Ty, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind,
            ty,
            span,
            mutable: false,
        }
    }

    /// Get the unique identifier of this expression
    pub fn id(&self) -> ExprId {
        self.id
    }

    /// Return a compact debug representation of this expression.
    pub fn debug_compact(&self) -> String {
        match &self.kind {
            ExprKind::Literal(lit) => match lit {
                LiteralValue::Unit => "()".to_string(),
                LiteralValue::Integer(n) => n.to_string(),
                LiteralValue::Float(f) => f.to_string(),
                LiteralValue::String(s) => format!("\"{}\"", s),
                LiteralValue::Bool(b) => b.to_string(),
            },
            ExprKind::Array(elements) => {
                let items: Vec<_> = elements.iter().map(|e| e.debug_compact()).collect();
                format!("[{}]", items.join(", "))
            },
            ExprKind::Tuple(elements) => {
                let items: Vec<_> = elements.iter().map(|e| e.debug_compact()).collect();
                format!("({})", items.join(", "))
            },
            ExprKind::Grouping(inner) => format!("({})", inner.debug_compact()),
            ExprKind::LocalRef(id) => format!("local_{}", id.0),
            ExprKind::SymbolRef(id) => format!("symbol_{:?}", id),
            ExprKind::OverloadedRef(_) => "overloaded".to_string(),
            ExprKind::TypeRef(id) => format!("type_{:?}", id),
            ExprKind::TypeParameterRef(_) => "<type_param>".to_string(),
            ExprKind::AssociatedTypeRef => "<assoc_type>".to_string(),
            ExprKind::FieldAccess { object, field } => {
                format!("{}.{}", object.debug_compact(), field)
            },
            ExprKind::TupleIndex { tuple, index } => {
                format!("{}.{}", tuple.debug_compact(), index)
            },
            ExprKind::MethodRef {
                receiver,
                method_name,
                ..
            } => format!("{}.{}", receiver.debug_compact(), method_name),
            ExprKind::Call {
                callee, arguments, ..
            } => {
                let args: Vec<String> = arguments
                    .iter()
                    .map(|a| {
                        if let Some(ref label) = a.label {
                            format!("{}: {}", label, a.value.debug_compact())
                        } else {
                            a.value.debug_compact()
                        }
                    })
                    .collect();
                format!("{}({})", callee.debug_compact(), args.join(", "))
            },
            ExprKind::PrimitiveMethodCall {
                receiver,
                method,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| a.value.debug_compact()).collect();
                format!(
                    "{}.{}({})",
                    receiver.debug_compact(),
                    method.name(),
                    args.join(", ")
                )
            },
            ExprKind::PrimitiveMethodRef { receiver, method } => {
                format!("{}.{}", receiver.debug_compact(), method.name())
            },
            ExprKind::DeferredMethodCall {
                receiver,
                method_name,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| a.value.debug_compact()).collect();
                format!(
                    "{}.{}({})",
                    receiver.debug_compact(),
                    method_name,
                    args.join(", ")
                )
            },
            ExprKind::DeferredStaticCall {
                target_ty,
                method_name,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| a.value.debug_compact()).collect();
                format!("{}.{}({})", target_ty, method_name, args.join(", "))
            },
            ExprKind::ImplicitStructInit {
                struct_type,
                arguments,
            } => {
                let args: Vec<String> = arguments
                    .iter()
                    .map(|a| {
                        if let Some(ref label) = a.label {
                            format!("{}: {}", label, a.value.debug_compact())
                        } else {
                            a.value.debug_compact()
                        }
                    })
                    .collect();
                format!("{}({})", struct_type, args.join(", "))
            },
            ExprKind::DelegatingInit { arguments, .. } => {
                let args: Vec<String> = arguments
                    .iter()
                    .map(|a| {
                        if let Some(ref label) = a.label {
                            format!("{}: {}", label, a.value.debug_compact())
                        } else {
                            a.value.debug_compact()
                        }
                    })
                    .collect();
                format!("self.init({})", args.join(", "))
            },
            ExprKind::Assignment { target, value } => {
                format!("{} = {}", target.debug_compact(), value.debug_compact())
            },
            ExprKind::If {
                conditions,
                then_value,
                else_branch,
                ..
            } => {
                let cond_strs: Vec<String> = conditions
                    .iter()
                    .map(|c| match c {
                        IfCondition::Expr(e) => e.debug_compact(),
                        IfCondition::Let { pattern, value, .. } => {
                            format!("let {:?} = {}", pattern, value.debug_compact())
                        },
                    })
                    .collect();
                let cond_str = cond_strs.join(", ");
                let then_str = if let Some(v) = then_value {
                    v.debug_compact()
                } else {
                    "...".to_string()
                };
                let else_str = if let Some(else_b) = else_branch {
                    match else_b {
                        ElseBranch::Block { value, .. } => {
                            if let Some(v) = value {
                                format!(" else {{ {} }}", v.debug_compact())
                            } else {
                                " else { ... }".to_string()
                            }
                        },
                        ElseBranch::ElseIf(_) => " else if ...".to_string(),
                    }
                } else {
                    String::new()
                };
                format!("if {} {{ {} }}{}", cond_str, then_str, else_str)
            },
            ExprKind::While { condition, .. } => {
                format!("while {} {{ ... }}", condition.debug_compact())
            },
            ExprKind::WhileLet { conditions, .. } => {
                let conds: Vec<_> = conditions
                    .iter()
                    .map(|c| match c {
                        IfCondition::Let { pattern, value, .. } => {
                            format!("let {:?} = {}", pattern, value.debug_compact())
                        },
                        IfCondition::Expr(e) => e.debug_compact(),
                    })
                    .collect();
                format!("while {} {{ ... }}", conds.join(", "))
            },
            ExprKind::Loop { .. } => "loop { ... }".to_string(),
            ExprKind::Break { label, .. } => {
                if let Some(l) = label {
                    format!("break {}", l.name)
                } else {
                    "break".to_string()
                }
            },
            ExprKind::Continue { label, .. } => {
                if let Some(l) = label {
                    format!("continue {}", l.name)
                } else {
                    "continue".to_string()
                }
            },
            ExprKind::Return { value } => {
                if let Some(v) = value {
                    format!("return {}", v.debug_compact())
                } else {
                    "return".to_string()
                }
            },
            ExprKind::Closure {
                params,
                tail_expr,
                uses_it: _,
                ..
            } => {
                let params_str = match params {
                    Some(ps) => {
                        let p: Vec<_> = ps.iter().map(|p| p.name.clone()).collect();
                        format!("({}) in ", p.join(", "))
                    },
                    None => String::new(), // No explicit params (may use `it` implicitly)
                };
                let body_str = tail_expr
                    .as_ref()
                    .map(|e| e.debug_compact())
                    .unwrap_or_else(|| "...".to_string());
                format!("{{ {}{} }}", params_str, body_str)
            },
            ExprKind::EnumCase { case_id } => format!("case_{:?}", case_id),
            ExprKind::ImplicitMemberAccess {
                member_name,
                arguments,
            } => {
                if let Some(args) = arguments {
                    let args_str: Vec<String> = args
                        .iter()
                        .map(|a| {
                            if let Some(ref label) = a.label {
                                format!("{}: {}", label, a.value.debug_compact())
                            } else {
                                a.value.debug_compact()
                            }
                        })
                        .collect();
                    format!(".{}({})", member_name, args_str.join(", "))
                } else {
                    format!(".{}", member_name)
                }
            },
            ExprKind::Match { scrutinee, arms } => {
                format!(
                    "match {} {{ {} arms }}",
                    scrutinee.debug_compact(),
                    arms.len()
                )
            },
            ExprKind::Block { value, .. } => {
                let body_str = value
                    .as_ref()
                    .map(|e| e.debug_compact())
                    .unwrap_or_else(|| "...".to_string());
                format!("{{ {} }}", body_str)
            },
            ExprKind::Error => "<error>".to_string(),
            ExprKind::LangIntrinsic {
                intrinsic,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| a.value.debug_compact()).collect();
                format!("{}({})", intrinsic.name(), args.join(", "))
            },
            ExprKind::LangIntrinsicRef(intrinsic) => intrinsic.name(),
            ExprKind::SubscriptCall {
                receiver,
                getter: _,
                arguments,
            } => {
                let args: Vec<String> = arguments
                    .iter()
                    .map(|a| {
                        if let Some(ref label) = a.label {
                            format!("{}: {}", label, a.value.debug_compact())
                        } else {
                            a.value.debug_compact()
                        }
                    })
                    .collect();
                format!("{}({})", receiver.debug_compact(), args.join(", "))
            },
        }
    }

    /// Create a unit literal expression.
    pub fn unit(span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::Unit),
            ty: Ty::unit(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create an integer literal expression with default type Int64.
    ///
    /// For protocol-based literal inference, use `integer_infer` instead.
    pub fn integer(value: i64, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::Integer(value)),
            ty: Ty::int(crate::ty::IntBits::I64, span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create an integer literal expression with inference type.
    ///
    /// During type inference, an ExpressibleByIntLiteral constraint will be added
    /// and the type will be resolved based on context (defaulting to Int64 if ambiguous).
    pub fn integer_infer(value: i64, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::Integer(value)),
            ty: Ty::infer(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a float literal expression with default type Float64.
    ///
    /// For protocol-based literal inference, use `float_infer` instead.
    pub fn float(value: f64, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::Float(value)),
            ty: Ty::float(crate::ty::FloatBits::F64, span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a float literal expression with inference type.
    ///
    /// During type inference, an ExpressibleByFloatLiteral constraint will be added
    /// and the type will be resolved based on context (defaulting to Float64 if ambiguous).
    pub fn float_infer(value: f64, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::Float(value)),
            ty: Ty::infer(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a string literal expression with default type String.
    ///
    /// For protocol-based literal inference, use `string_infer` instead.
    pub fn string(value: String, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::String(value)),
            ty: Ty::string(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a string literal expression with inference type.
    ///
    /// During type inference, an ExpressibleByStringLiteral constraint will be added
    /// and the type will be resolved based on context (defaulting to String if ambiguous).
    pub fn string_infer(value: String, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::String(value)),
            ty: Ty::infer(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a boolean literal expression with default type Bool.
    ///
    /// For protocol-based literal inference, use `bool_infer` instead.
    pub fn bool(value: bool, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::Bool(value)),
            ty: Ty::bool(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a boolean literal expression with inference type.
    ///
    /// During type inference, an ExpressibleByBoolLiteral constraint will be added
    /// and the type will be resolved based on context (defaulting to Bool if ambiguous).
    pub fn bool_infer(value: bool, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::Bool(value)),
            ty: Ty::infer(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create an array literal expression.
    pub fn array(elements: Vec<Expression>, element_ty: Ty, span: Span) -> Self {
        let ty = Ty::array(element_ty, span.clone());
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Array(elements),
            ty,
            span,
            mutable: false,
        }
    }

    /// Create a tuple literal expression.
    pub fn tuple(elements: Vec<Expression>, span: Span) -> Self {
        let element_types: Vec<Ty> = elements.iter().map(|e| e.ty.clone()).collect();
        let ty = Ty::tuple(element_types, span.clone());
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Tuple(elements),
            ty,
            span,
            mutable: false,
        }
    }

    /// Create a grouping expression.
    /// Preserves the mutability of the inner expression.
    pub fn grouping(inner: Expression, span: Span) -> Self {
        let ty = inner.ty.clone();
        let mutable = inner.mutable;
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Grouping(Box::new(inner)),
            ty,
            span,
            mutable,
        }
    }

    /// Create a local variable reference expression.
    /// Mutability must be provided by the caller (from the Local's is_mutable()).
    pub fn local_ref(local_id: LocalId, ty: Ty, mutable: bool, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::LocalRef(local_id),
            ty,
            span,
            mutable,
        }
    }

    /// Create a symbol reference expression.
    /// Mutability must be provided by the caller (from the symbol's declaration).
    pub fn symbol_ref(symbol_id: SymbolId, ty: Ty, mutable: bool, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::SymbolRef(symbol_id),
            ty,
            span,
            mutable,
        }
    }

    /// Create an overloaded function reference expression.
    /// Type is inferred later when call resolution disambiguates the overload.
    /// Functions are not mutable lvalues.
    pub fn overloaded_ref(candidates: Vec<SymbolId>, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::OverloadedRef(candidates),
            ty: Ty::infer(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a type reference expression.
    /// Used when a path resolves to a type (e.g., struct name in `Point(x: 1, y: 2)`).
    /// Types are not mutable lvalues.
    pub fn type_ref(symbol_id: SymbolId, ty: Ty, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::TypeRef(symbol_id),
            ty,
            span,
            mutable: false,
        }
    }

    /// Create a type parameter reference expression.
    /// Used when a path resolves to a type parameter for static method/init calls.
    /// E.g., `T()` or `T.create()` where T is constrained by protocol bounds.
    /// Type parameters used this way are not mutable lvalues.
    /// The `ty` parameter should be the type parameter type (Ty::type_parameter(...))
    /// so that Self substitution works correctly.
    pub fn type_parameter_ref(symbol_id: SymbolId, ty: Ty, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::TypeParameterRef(symbol_id),
            ty,
            span,
            mutable: false,
        }
    }

    /// Create an associated type reference expression.
    /// Used when accessing an associated type on a type parameter for static member access.
    /// E.g., `T.Next` where T: Level1 and Level1 has `type Next: Level2`.
    /// The `ty` parameter should be a `Ty::qualified_associated_type(...)`.
    pub fn associated_type_ref(ty: Ty, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::AssociatedTypeRef,
            ty,
            span,
            mutable: false,
        }
    }

    /// Create a field access expression.
    /// Mutability is computed as: field_mutable AND object.mutable
    pub fn field_access(
        object: Expression,
        field: String,
        field_mutable: bool,
        ty: Ty,
        span: Span,
    ) -> Self {
        let mutable = field_mutable && object.mutable;
        Expression {
            id: ExprId::new(),
            kind: ExprKind::FieldAccess {
                object: Box::new(object),
                field,
            },
            ty,
            span,
            mutable,
        }
    }

    /// Create a tuple index expression.
    /// Mutability depends on the tuple's mutability.
    pub fn tuple_index(tuple: Expression, index: usize, element_ty: Ty, span: Span) -> Self {
        let mutable = tuple.mutable;
        Expression {
            id: ExprId::new(),
            kind: ExprKind::TupleIndex {
                tuple: Box::new(tuple),
                index,
            },
            ty: element_ty,
            span,
            mutable,
        }
    }

    /// Create a method reference expression.
    /// Type is inferred later when call resolution happens.
    /// Method references are not mutable lvalues.
    pub fn method_ref(
        receiver: Expression,
        candidates: Vec<SymbolId>,
        method_name: String,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::MethodRef {
                receiver: Box::new(receiver),
                candidates,
                method_name,
            },
            ty: Ty::infer(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a call expression.
    /// Return values are not mutable lvalues.
    pub fn call(
        callee: Expression,
        arguments: Vec<CallArgument>,
        return_ty: Ty,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Call {
                callee: Box::new(callee),
                arguments,
                substitutions: Substitutions::new(),
            },
            ty: return_ty,
            span,
            mutable: false,
        }
    }

    /// Create a call expression with type argument substitutions for generic calls.
    /// Return values are not mutable lvalues.
    pub fn generic_call(
        callee: Expression,
        arguments: Vec<CallArgument>,
        substitutions: Substitutions,
        return_ty: Ty,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Call {
                callee: Box::new(callee),
                arguments,
                substitutions,
            },
            ty: return_ty,
            span,
            mutable: false,
        }
    }

    /// Create a subscript call expression.
    /// Used for `array(0)`, `dict(key: "foo")`, etc.
    /// Return values are not mutable lvalues.
    pub fn subscript_call(
        receiver: Expression,
        getter: SymbolId,
        arguments: Vec<CallArgument>,
        return_ty: Ty,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::SubscriptCall {
                receiver: Box::new(receiver),
                getter,
                arguments,
            },
            ty: return_ty,
            span,
            mutable: false,
        }
    }

    /// Create a primitive method call expression.
    /// Return values are not mutable lvalues.
    pub fn primitive_method_call(
        receiver: Expression,
        method: PrimitiveMethod,
        arguments: Vec<CallArgument>,
        span: Span,
    ) -> Self {
        let return_ty = method.return_type(&receiver.ty, span.clone());
        Expression {
            id: ExprId::new(),
            kind: ExprKind::PrimitiveMethodCall {
                receiver: Box::new(receiver),
                method,
                arguments,
            },
            ty: return_ty,
            span,
            mutable: false,
        }
    }

    /// Create a primitive method call expression with an explicit result type.
    /// Used when the receiver type is Infer and we can't determine the return type yet.
    pub fn primitive_method_call_with_type(
        receiver: Expression,
        method: PrimitiveMethod,
        arguments: Vec<CallArgument>,
        result_ty: Ty,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::PrimitiveMethodCall {
                receiver: Box::new(receiver),
                method,
                arguments,
            },
            ty: result_ty,
            span,
            mutable: false,
        }
    }

    /// Create a primitive method reference expression.
    /// This is used when a primitive method is accessed but not immediately called.
    /// Primitive methods cannot be used as first-class values, so call resolution
    /// will emit an error if this reference is not converted to a PrimitiveMethodCall.
    pub fn primitive_method_ref(receiver: Expression, method: PrimitiveMethod, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::PrimitiveMethodRef {
                receiver: Box::new(receiver),
                method,
            },
            // Type is infer since primitive methods can't be first-class values.
            // Call resolution will convert this to a proper call with the return type.
            ty: Ty::infer(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a deferred method call expression.
    /// Used when the receiver type is Infer and method resolution must be deferred
    /// until type inference resolves the receiver's actual type.
    pub fn deferred_method_call(
        receiver: Expression,
        method_name: String,
        arguments: Vec<CallArgument>,
        result_ty: Ty,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::DeferredMethodCall {
                receiver: Box::new(receiver),
                method_name,
                arguments,
            },
            ty: result_ty,
            span,
            mutable: false,
        }
    }

    /// Create a deferred static method call expression.
    /// Used for `try` expressions where we call `R.fromResidual(early)` and R is
    /// the function's return type (which may have inference variables).
    pub fn deferred_static_call(
        target_ty: Ty,
        method_name: String,
        arguments: Vec<CallArgument>,
        result_ty: Ty,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::DeferredStaticCall {
                target_ty,
                method_name,
                arguments,
            },
            ty: result_ty,
            span,
            mutable: false,
        }
    }

    /// Create an error expression (poison value).
    pub fn error(span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Error,
            ty: Ty::error(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create an implicit struct initialization expression.
    ///
    /// Used when calling a struct type without explicit initializers.
    /// Struct initialization results are not mutable lvalues.
    pub fn implicit_struct_init(struct_type: Ty, arguments: Vec<CallArgument>, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::ImplicitStructInit {
                struct_type: struct_type.clone(),
                arguments,
            },
            ty: struct_type,
            span,
            mutable: false,
        }
    }

    /// Create a delegating initializer call expression.
    ///
    /// Used when calling `self.init(...)` from within an initializer.
    /// The type is unit since the initializer modifies self in place.
    pub fn delegating_init(
        initializer: SymbolId,
        arguments: Vec<CallArgument>,
        substitutions: Substitutions,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::DelegatingInit {
                initializer,
                arguments,
                substitutions,
            },
            ty: Ty::unit(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a resolved enum case expression.
    ///
    /// Used when a path resolves to an enum case without associated values.
    /// Enum cases are not mutable lvalues.
    pub fn enum_case(case_id: SymbolId, ty: Ty, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::EnumCase { case_id },
            ty,
            span,
            mutable: false,
        }
    }

    /// Create an implicit member access with `Ty::infer()`.
    ///
    /// Type inference will resolve the actual type based on context.
    /// Used for Swift-style shorthand: `.None` instead of `Option.None`.
    pub fn implicit_member_access(
        member_name: String,
        arguments: Option<Vec<CallArgument>>,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::ImplicitMemberAccess {
                member_name,
                arguments,
            },
            ty: Ty::infer(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create an assignment expression.
    ///
    /// The type of an assignment expression is Never, meaning the value
    /// cannot be used. This prevents chaining like `x = y = z`.
    /// Assignments are not mutable lvalues.
    pub fn assignment(target: Expression, value: Expression, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Assignment {
                target: Box::new(target),
                value: Box::new(value),
            },
            ty: Ty::unit(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create an if expression with a single boolean condition.
    ///
    /// Type computation with Never propagation:
    /// - No else branch: type is `()`
    /// - With else branch: join the types of both branches
    ///   - If one branch is Never (return, break, etc.), use the other branch's type
    ///   - If both branches are Never, the type is Never
    ///   - Otherwise, use the then branch's type (type checking validates compatibility)
    ///
    /// If expressions are not mutable lvalues.
    pub fn if_expr(
        condition: Expression,
        then_branch: Vec<crate::stmt::Statement>,
        then_value: Option<Expression>,
        else_branch: Option<ElseBranch>,
        span: Span,
    ) -> Self {
        Self::if_expr_with_conditions(
            vec![IfCondition::Expr(condition)],
            then_branch,
            then_value,
            else_branch,
            span,
        )
    }

    /// Create an if expression with multiple conditions (if-let chains).
    ///
    /// Type computation with Never propagation:
    /// - No else branch: type is `()`
    /// - With else branch: join the types of both branches
    ///   - If one branch is Never (return, break, etc.), use the other branch's type
    ///   - If both branches are Never, the type is Never
    ///   - Otherwise, use the then branch's type (type checking validates compatibility)
    ///
    /// If expressions are not mutable lvalues.
    pub fn if_expr_with_conditions(
        conditions: Vec<IfCondition>,
        then_branch: Vec<crate::stmt::Statement>,
        then_value: Option<Expression>,
        else_branch: Option<ElseBranch>,
        span: Span,
    ) -> Self {
        // Compute the type with Never propagation:
        // - No else: type is ()
        // - With else: join the then and else branch types
        let ty = match &else_branch {
            Some(else_br) => {
                let then_ty = compute_block_type(&then_branch, then_value.as_ref(), &span);
                let else_ty = else_br.ty(&span);

                // Join the types - handles Never propagation
                then_ty.join(&else_ty)
            },
            None => Ty::unit(span.clone()),
        };

        Expression {
            id: ExprId::new(),
            kind: ExprKind::If {
                conditions,
                then_branch,
                then_value: then_value.map(Box::new),
                else_branch,
            },
            ty,
            span,
            mutable: false,
        }
    }

    /// Create a while loop expression.
    ///
    /// Type is always `()` (unit).
    pub fn while_loop(
        loop_id: LoopId,
        label: Option<LabelInfo>,
        condition: Expression,
        body: Vec<crate::stmt::Statement>,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::While {
                loop_id,
                label,
                condition: Box::new(condition),
                body,
            },
            ty: Ty::unit(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a while-let loop expression.
    ///
    /// Loops while all conditions are true.
    /// Type is always `()` (unit).
    pub fn while_let(
        loop_id: LoopId,
        label: Option<LabelInfo>,
        conditions: Vec<IfCondition>,
        body: Vec<crate::stmt::Statement>,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::WhileLet {
                loop_id,
                label,
                conditions,
                body,
            },
            ty: Ty::unit(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create an infinite loop expression.
    ///
    /// Type is always `()` (unit).
    pub fn loop_expr(
        loop_id: LoopId,
        label: Option<LabelInfo>,
        body: Vec<crate::stmt::Statement>,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Loop {
                loop_id,
                label,
                body,
            },
            ty: Ty::unit(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a break expression.
    ///
    /// Type is `Never` - control transfers out of the loop.
    pub fn break_expr(loop_id: LoopId, label: Option<LabelInfo>, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Break { loop_id, label },
            ty: Ty::never(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a continue expression.
    ///
    /// Type is `Never` - control transfers to the loop condition.
    pub fn continue_expr(loop_id: LoopId, label: Option<LabelInfo>, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Continue { loop_id, label },
            ty: Ty::never(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a return expression.
    ///
    /// Type is `Never` - control transfers out of the function.
    pub fn return_expr(value: Option<Expression>, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Return {
                value: value.map(Box::new),
            },
            ty: Ty::never(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a match expression.
    ///
    /// Type is computed from the arm bodies - they should all have compatible types.
    /// If all arms have Never type, the match expression has Never type.
    /// Otherwise, the type is inferred and will be resolved during type inference.
    pub fn match_expr(scrutinee: Expression, arms: Vec<MatchArm>, span: Span) -> Self {
        // Compute the type by joining all arm body types
        // This handles Never propagation correctly
        let ty = if arms.is_empty() {
            Ty::never(span.clone())
        } else {
            arms.iter()
                .map(|arm| arm.body.ty.clone())
                .reduce(|a, b| a.join(&b))
                .unwrap_or_else(|| Ty::infer(span.clone()))
        };

        Expression {
            id: ExprId::new(),
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            ty,
            span,
            mutable: false,
        }
    }

    /// Create a closure expression.
    ///
    /// Closures are anonymous functions that can capture variables from their enclosing scope.
    /// The type should be a function type matching the parameters and return type.
    pub fn closure(
        params: Option<Vec<ClosureParam>>,
        body: Vec<crate::stmt::Statement>,
        tail_expr: Option<Expression>,
        captures: Vec<Capture>,
        uses_it: bool,
        implicit_param: Option<(LocalId, Ty, Span)>,
        ty: Ty,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Closure {
                params,
                body,
                tail_expr: tail_expr.map(Box::new),
                captures,
                uses_it,
                implicit_param,
            },
            ty,
            span,
            mutable: false,
        }
    }

    /// Create a block expression.
    ///
    /// Block expressions contain statements and an optional trailing value.
    /// Used for match arm bodies that have statements.
    /// Unlike closures, blocks do not capture variables - they execute inline.
    pub fn block(
        statements: Vec<crate::stmt::Statement>,
        value: Option<Expression>,
        ty: Ty,
        span: Span,
    ) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Block {
                statements,
                value: value.map(Box::new),
            },
            ty,
            span,
            mutable: false,
        }
    }

    /// Check if this expression refers to a mutable location.
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }

    /// Check if this is a literal expression.
    pub fn is_literal(&self) -> bool {
        matches!(self.kind, ExprKind::Literal(_))
    }

    /// Check if this is an error expression.
    pub fn is_error(&self) -> bool {
        matches!(self.kind, ExprKind::Error)
    }

    /// Get the literal value if this is a literal expression.
    pub fn as_literal(&self) -> Option<&LiteralValue> {
        match &self.kind {
            ExprKind::Literal(val) => Some(val),
            _ => None,
        }
    }

    /// Create a language intrinsic call expression.
    ///
    /// Used for `lang.*` intrinsic functions that are handled specially by the compiler.
    /// The return type is `Never` for panic_unwind, or the target type for casts.
    pub fn lang_intrinsic(
        intrinsic: LangIntrinsic,
        arguments: Vec<CallArgument>,
        span: Span,
    ) -> Self {
        let ty = match &intrinsic {
            LangIntrinsic::PanicUnwind => Ty::never(span.clone()),
            LangIntrinsic::Cast { to, .. } => to.to_ty(span.clone()),
            LangIntrinsic::IntBinary { primitive, op } => {
                // Comparison ops return bool (i1), others return the same type
                if matches!(op, IntBinaryOp::Eq | IntBinaryOp::Ne) {
                    Ty::bool(span.clone())
                } else {
                    primitive.to_ty(span.clone())
                }
            },
            LangIntrinsic::IntBinarySigned { primitive, op }
            | LangIntrinsic::IntBinaryUnsigned { primitive, op } => {
                // Comparison ops return bool (i1), others return the same type
                if matches!(
                    op,
                    SignedOp::Lt | SignedOp::Le | SignedOp::Gt | SignedOp::Ge
                ) {
                    Ty::bool(span.clone())
                } else {
                    primitive.to_ty(span.clone())
                }
            },
            LangIntrinsic::IntUnary { primitive, .. } => primitive.to_ty(span.clone()),
            LangIntrinsic::FloatBinary { primitive, op } => {
                // Comparison ops return bool (i1), others return the same type
                if matches!(
                    op,
                    FloatBinaryOp::Eq
                        | FloatBinaryOp::Ne
                        | FloatBinaryOp::Lt
                        | FloatBinaryOp::Le
                        | FloatBinaryOp::Gt
                        | FloatBinaryOp::Ge
                ) {
                    Ty::bool(span.clone())
                } else {
                    primitive.to_ty(span.clone())
                }
            },
            LangIntrinsic::FloatUnary { primitive, .. } => primitive.to_ty(span.clone()),
            // FloatConst returns the float type
            LangIntrinsic::FloatConst { primitive, .. } => primitive.to_ty(span.clone()),
            // FloatPred returns bool
            LangIntrinsic::FloatPred { .. } => Ty::bool(span.clone()),
            // FloatMath returns the float type
            LangIntrinsic::FloatMath { primitive, .. } => primitive.to_ty(span.clone()),
            // Pointer intrinsics
            LangIntrinsic::PtrNull { pointee_ty } => Ty::pointer(pointee_ty.clone(), span.clone()),
            LangIntrinsic::PtrFromAddress { pointee_ty } => {
                Ty::pointer(pointee_ty.clone(), span.clone())
            },
            LangIntrinsic::PtrToAddress => Ty::int(IntBits::I64, span.clone()),
            LangIntrinsic::PtrTo { pointee_ty } => Ty::pointer(pointee_ty.clone(), span.clone()),
            LangIntrinsic::PtrRead { pointee_ty } => pointee_ty.clone(),
            LangIntrinsic::PtrWrite { .. } => Ty::unit(span.clone()),
            LangIntrinsic::PtrOffset => {
                // Return type matches first argument (pointer type)
                // For now, use an inference variable
                Ty::pointer(Ty::infer(span.clone()), span.clone())
            },
            LangIntrinsic::PtrIsNull => Ty::bool(span.clone()),
            LangIntrinsic::CastPtr { target_ty } => Ty::pointer(target_ty.clone(), span.clone()),
            LangIntrinsic::SizeOf { .. } | LangIntrinsic::AlignOf { .. } => {
                Ty::int(IntBits::I64, span.clone())
            },
            // Boolean (i1) intrinsics all return bool
            LangIntrinsic::I1Eq
            | LangIntrinsic::I1And
            | LangIntrinsic::I1Or
            | LangIntrinsic::I1Not => Ty::bool(span.clone()),
            // Atomic intrinsics return the type of the first argument (the value being modified)
            LangIntrinsic::AtomicAdd | LangIntrinsic::AtomicSub => {
                // Return type inferred from first argument
                Ty::infer(span.clone())
            },
        };
        Expression {
            id: ExprId::new(),
            kind: ExprKind::LangIntrinsic {
                intrinsic,
                arguments,
            },
            ty,
            span,
            mutable: false,
        }
    }

    /// Create a reference to a language intrinsic function (not yet called).
    ///
    /// This is similar to `symbol_ref` but for intrinsics that don't have real symbols.
    /// The type is the function signature of the intrinsic.
    pub fn lang_intrinsic_ref(intrinsic: LangIntrinsic, span: Span) -> Self {
        // Create a function type for the intrinsic
        let ty = match &intrinsic {
            LangIntrinsic::PanicUnwind => {
                // (String) -> Never
                Ty::function(
                    vec![Ty::string(span.clone())],
                    Ty::never(span.clone()),
                    span.clone(),
                )
            },
            LangIntrinsic::Cast { from, to } => {
                // (From) -> To
                Ty::function(
                    vec![from.to_ty(span.clone())],
                    to.to_ty(span.clone()),
                    span.clone(),
                )
            },
            LangIntrinsic::IntBinary { primitive, op } => {
                // (T, T) -> T or (T, T) -> Bool for comparisons
                let prim_ty = primitive.to_ty(span.clone());
                let ret_ty = if matches!(op, IntBinaryOp::Eq | IntBinaryOp::Ne) {
                    Ty::bool(span.clone())
                } else {
                    prim_ty.clone()
                };
                Ty::function(vec![prim_ty.clone(), prim_ty], ret_ty, span.clone())
            },
            LangIntrinsic::IntBinarySigned { primitive, op }
            | LangIntrinsic::IntBinaryUnsigned { primitive, op } => {
                // (T, T) -> T or (T, T) -> Bool for comparisons
                let prim_ty = primitive.to_ty(span.clone());
                let ret_ty = if matches!(
                    op,
                    SignedOp::Lt | SignedOp::Le | SignedOp::Gt | SignedOp::Ge
                ) {
                    Ty::bool(span.clone())
                } else {
                    prim_ty.clone()
                };
                Ty::function(vec![prim_ty.clone(), prim_ty], ret_ty, span.clone())
            },
            LangIntrinsic::IntUnary { primitive, .. } => {
                // (T) -> T
                let prim_ty = primitive.to_ty(span.clone());
                Ty::function(vec![prim_ty.clone()], prim_ty, span.clone())
            },
            LangIntrinsic::FloatBinary { primitive, op } => {
                // (T, T) -> T or (T, T) -> Bool for comparisons
                let prim_ty = primitive.to_ty(span.clone());
                let ret_ty = if matches!(
                    op,
                    FloatBinaryOp::Eq
                        | FloatBinaryOp::Ne
                        | FloatBinaryOp::Lt
                        | FloatBinaryOp::Le
                        | FloatBinaryOp::Gt
                        | FloatBinaryOp::Ge
                ) {
                    Ty::bool(span.clone())
                } else {
                    prim_ty.clone()
                };
                Ty::function(vec![prim_ty.clone(), prim_ty], ret_ty, span.clone())
            },
            LangIntrinsic::FloatUnary { primitive, .. } => {
                // (T) -> T
                let prim_ty = primitive.to_ty(span.clone());
                Ty::function(vec![prim_ty.clone()], prim_ty, span.clone())
            },
            LangIntrinsic::FloatConst { primitive, .. } => {
                // () -> T
                let prim_ty = primitive.to_ty(span.clone());
                Ty::function(vec![], prim_ty, span.clone())
            },
            LangIntrinsic::FloatPred { primitive, .. } => {
                // (T) -> Bool
                let prim_ty = primitive.to_ty(span.clone());
                Ty::function(vec![prim_ty], Ty::bool(span.clone()), span.clone())
            },
            LangIntrinsic::FloatMath { primitive, .. } => {
                // (T) -> T
                let prim_ty = primitive.to_ty(span.clone());
                Ty::function(vec![prim_ty.clone()], prim_ty, span.clone())
            },
            // Pointer intrinsics
            LangIntrinsic::PtrNull { pointee_ty } => {
                // () -> lang.ptr[T]
                let ptr_ty = Ty::pointer(pointee_ty.clone(), span.clone());
                Ty::function(vec![], ptr_ty, span.clone())
            },
            LangIntrinsic::PtrFromAddress { pointee_ty } => {
                // (UInt) -> lang.ptr[T]
                let uint_ty = Ty::int(IntBits::I64, span.clone());
                let ptr_ty = Ty::pointer(pointee_ty.clone(), span.clone());
                Ty::function(vec![uint_ty], ptr_ty, span.clone())
            },
            LangIntrinsic::PtrToAddress => {
                // (lang.ptr[_]) -> UInt
                let ptr_ty = Ty::pointer(Ty::infer(span.clone()), span.clone());
                let uint_ty = Ty::int(IntBits::I64, span.clone());
                Ty::function(vec![ptr_ty], uint_ty, span.clone())
            },
            LangIntrinsic::PtrTo { pointee_ty } => {
                // (T) -> lang.ptr[T]
                let ptr_ty = Ty::pointer(pointee_ty.clone(), span.clone());
                Ty::function(vec![pointee_ty.clone()], ptr_ty, span.clone())
            },
            LangIntrinsic::PtrRead { pointee_ty } => {
                // (lang.ptr[T]) -> T
                let ptr_ty = Ty::pointer(pointee_ty.clone(), span.clone());
                Ty::function(vec![ptr_ty], pointee_ty.clone(), span.clone())
            },
            LangIntrinsic::PtrWrite { pointee_ty } => {
                // (lang.ptr[T], T) -> ()
                let ptr_ty = Ty::pointer(pointee_ty.clone(), span.clone());
                Ty::function(
                    vec![ptr_ty, pointee_ty.clone()],
                    Ty::unit(span.clone()),
                    span.clone(),
                )
            },
            LangIntrinsic::PtrOffset => {
                // (lang.ptr[_], Int) -> lang.ptr[_]
                let ptr_ty = Ty::pointer(Ty::infer(span.clone()), span.clone());
                let int_ty = Ty::int(IntBits::I64, span.clone());
                Ty::function(vec![ptr_ty.clone(), int_ty], ptr_ty, span.clone())
            },
            LangIntrinsic::PtrIsNull => {
                // (lang.ptr[_]) -> Bool
                let ptr_ty = Ty::pointer(Ty::infer(span.clone()), span.clone());
                Ty::function(vec![ptr_ty], Ty::bool(span.clone()), span.clone())
            },
            LangIntrinsic::CastPtr { target_ty } => {
                // (lang.ptr[_]) -> lang.ptr[T]
                let src_ptr_ty = Ty::pointer(Ty::infer(span.clone()), span.clone());
                let dst_ptr_ty = Ty::pointer(target_ty.clone(), span.clone());
                Ty::function(vec![src_ptr_ty], dst_ptr_ty, span.clone())
            },
            LangIntrinsic::SizeOf { .. } | LangIntrinsic::AlignOf { .. } => {
                // () -> Int
                let int_ty = Ty::int(IntBits::I64, span.clone());
                Ty::function(vec![], int_ty, span.clone())
            },
            // Boolean (i1) intrinsics
            LangIntrinsic::I1Eq | LangIntrinsic::I1And | LangIntrinsic::I1Or => {
                // (lang.i1, lang.i1) -> lang.i1
                let bool_ty = Ty::bool(span.clone());
                Ty::function(
                    vec![bool_ty.clone(), bool_ty.clone()],
                    bool_ty,
                    span.clone(),
                )
            },
            LangIntrinsic::I1Not => {
                // (lang.i1) -> lang.i1
                let bool_ty = Ty::bool(span.clone());
                Ty::function(vec![bool_ty.clone()], bool_ty, span.clone())
            },
            // Atomic intrinsics: (T, T) -> T where T is integer type
            LangIntrinsic::AtomicAdd | LangIntrinsic::AtomicSub => {
                // (infer, infer) -> infer - type inferred from arguments
                let infer_ty = Ty::infer(span.clone());
                Ty::function(
                    vec![infer_ty.clone(), infer_ty.clone()],
                    infer_ty,
                    span.clone(),
                )
            },
        };
        Expression {
            id: ExprId::new(),
            kind: ExprKind::LangIntrinsicRef(intrinsic),
            ty,
            span,
            mutable: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_span::Span;

    #[test]
    fn test_integer_literal() {
        let expr = Expression::integer(42, Span::new(0, 0..2));
        assert!(expr.is_literal());
        assert_eq!(expr.as_literal(), Some(&LiteralValue::Integer(42)));
        assert!(expr.ty.is_int());
    }

    #[test]
    fn test_string_literal() {
        let expr = Expression::string("hello".to_string(), Span::new(0, 0..7));
        assert!(expr.is_literal());
        assert_eq!(
            expr.as_literal(),
            Some(&LiteralValue::String("hello".to_string()))
        );
        assert!(expr.ty.is_string());
    }

    #[test]
    fn test_tuple_expression() {
        let elements = vec![
            Expression::integer(1, Span::new(0, 1..2)),
            Expression::integer(2, Span::new(0, 4..5)),
        ];
        let expr = Expression::tuple(elements, Span::new(0, 0..6));
        assert!(expr.ty.is_tuple());
    }

    #[test]
    fn test_error_expression() {
        let expr = Expression::error(Span::new(0, 0..5));
        assert!(expr.is_error());
        assert!(expr.ty.is_error());
    }
}
