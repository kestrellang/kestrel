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
use crate::ty::{Substitutions, Ty};

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
    if let Some(last_stmt) = statements.last() {
        if let StatementKind::Expr(expr) = &last_stmt.kind {
            return expr.ty.clone();
        }
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
    IntSub        { ty: Int, name: "sub", returns: SameAsReceiver },
    IntMul        { ty: Int, name: "mul", returns: SameAsReceiver },
    IntDiv        { ty: Int, name: "div", returns: SameAsReceiver },
    IntRem        { ty: Int, name: "rem", returns: SameAsReceiver },
    IntNeg        { ty: Int, name: "neg", returns: SameAsReceiver },
    IntIdentity   { ty: Int, name: "identity", returns: SameAsReceiver },

    // Int comparison
    IntEq         { ty: Int, name: "eq", returns: Bool },
    IntNe         { ty: Int, name: "ne", returns: Bool },
    IntLt         { ty: Int, name: "lt", returns: Bool },
    IntLe         { ty: Int, name: "le", returns: Bool },
    IntGt         { ty: Int, name: "gt", returns: Bool },
    IntGe         { ty: Int, name: "ge", returns: Bool },

    // Int bitwise
    IntBitAnd     { ty: Int, name: "bitAnd", returns: SameAsReceiver },
    IntBitOr      { ty: Int, name: "bitOr", returns: SameAsReceiver },
    IntBitXor     { ty: Int, name: "bitXor", returns: SameAsReceiver },
    IntBitNot     { ty: Int, name: "bitNot", returns: SameAsReceiver },
    IntShl        { ty: Int, name: "shl", returns: SameAsReceiver },
    IntShr        { ty: Int, name: "shr", returns: SameAsReceiver },

    // Float arithmetic
    FloatAdd      { ty: Float, name: "add", returns: SameAsReceiver },
    FloatSub      { ty: Float, name: "sub", returns: SameAsReceiver },
    FloatMul      { ty: Float, name: "mul", returns: SameAsReceiver },
    FloatDiv      { ty: Float, name: "div", returns: SameAsReceiver },
    FloatNeg      { ty: Float, name: "neg", returns: SameAsReceiver },
    FloatIdentity { ty: Float, name: "identity", returns: SameAsReceiver },

    // Float comparison
    FloatEq       { ty: Float, name: "eq", returns: Bool },
    FloatNe       { ty: Float, name: "ne", returns: Bool },
    FloatLt       { ty: Float, name: "lt", returns: Bool },
    FloatLe       { ty: Float, name: "le", returns: Bool },
    FloatGt       { ty: Float, name: "gt", returns: Bool },
    FloatGe       { ty: Float, name: "ge", returns: Bool },

    // Bool operators
    BoolAnd       { ty: Bool, name: "logicalAnd", returns: Bool },
    BoolOr        { ty: Bool, name: "logicalOr", returns: Bool },
    BoolNot       { ty: Bool, name: "logicalNot", returns: Bool },
    BoolEq        { ty: Bool, name: "eq", returns: Bool },
    BoolNe        { ty: Bool, name: "ne", returns: Bool },

    // String methods
    StringLength  { ty: String, name: "length", returns: Int },
    StringIsEmpty { ty: String, name: "isEmpty", returns: Bool },
    StringEq      { ty: String, name: "eq", returns: Bool },
    StringNe      { ty: String, name: "ne", returns: Bool },
}

impl PrimitiveMethod {
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

    /// Primitive method call: `5.toString()`, `"hello".length()`
    /// No symbol exists - compiler has built-in knowledge.
    PrimitiveMethodCall {
        receiver: Box<Expression>,
        method: PrimitiveMethod,
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

    /// Assignment expression: target = value
    /// Type is Never (assignment doesn't produce a usable value)
    Assignment {
        target: Box<Expression>,
        value: Box<Expression>,
    },

    /// If expression: `if condition { then } else { else }`
    ///
    /// Type is:
    /// - `()` if there's no else branch
    /// - The common type of both branches if there's an else branch
    ///   (type checking deferred - currently just uses then_branch type)
    If {
        condition: Box<Expression>,
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

    /// Error expression (poison value).
    /// Used when expression resolution fails - prevents cascading errors.
    Error,
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
            }
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
            }
            ExprKind::Tuple(elements) => {
                let items: Vec<_> = elements.iter().map(|e| e.debug_compact()).collect();
                format!("({})", items.join(", "))
            }
            ExprKind::Grouping(inner) => format!("({})", inner.debug_compact()),
            ExprKind::LocalRef(id) => format!("local_{}", id.0),
            ExprKind::SymbolRef(id) => format!("symbol_{:?}", id),
            ExprKind::OverloadedRef(_) => "overloaded".to_string(),
            ExprKind::TypeRef(id) => format!("type_{:?}", id),
            ExprKind::TypeParameterRef(_) => "<type_param>".to_string(),
            ExprKind::AssociatedTypeRef => "<assoc_type>".to_string(),
            ExprKind::FieldAccess { object, field } => {
                format!("{}.{}", object.debug_compact(), field)
            }
            ExprKind::TupleIndex { tuple, index } => {
                format!("{}.{}", tuple.debug_compact(), index)
            }
            ExprKind::MethodRef {
                receiver,
                method_name,
                ..
            } => format!("{}.{}", receiver.debug_compact(), method_name),
            ExprKind::Call { callee, arguments, .. } => {
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
            }
            ExprKind::PrimitiveMethodCall {
                receiver,
                method,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| a.value.debug_compact()).collect();
                format!("{}.{}({})", receiver.debug_compact(), method.name(), args.join(", "))
            }
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
            }
            ExprKind::Assignment { target, value } => {
                format!("{} = {}", target.debug_compact(), value.debug_compact())
            }
            ExprKind::If {
                condition,
                then_value,
                else_branch,
                ..
            } => {
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
                        }
                        ElseBranch::ElseIf(_) => " else if ...".to_string(),
                    }
                } else {
                    String::new()
                };
                format!(
                    "if {} {{ {} }}{}",
                    condition.debug_compact(),
                    then_str,
                    else_str
                )
            }
            ExprKind::While { condition, .. } => {
                format!("while {} {{ ... }}", condition.debug_compact())
            }
            ExprKind::Loop { .. } => "loop { ... }".to_string(),
            ExprKind::Break { label, .. } => {
                if let Some(l) = label {
                    format!("break {}", l.name)
                } else {
                    "break".to_string()
                }
            }
            ExprKind::Continue { label, .. } => {
                if let Some(l) = label {
                    format!("continue {}", l.name)
                } else {
                    "continue".to_string()
                }
            }
            ExprKind::Return { value } => {
                if let Some(v) = value {
                    format!("return {}", v.debug_compact())
                } else {
                    "return".to_string()
                }
            }
            ExprKind::Closure { params, tail_expr, uses_it, .. } => {
                let params_str = match params {
                    Some(ps) => {
                        let p: Vec<_> = ps.iter().map(|p| p.name.clone()).collect();
                        format!("({}) in ", p.join(", "))
                    }
                    None => {
                        if *uses_it {
                            String::new() // Implicit `it` style
                        } else {
                            String::new() // No params, no `it`
                        }
                    }
                };
                let body_str = tail_expr
                    .as_ref()
                    .map(|e| e.debug_compact())
                    .unwrap_or_else(|| "...".to_string());
                format!("{{ {}{} }}", params_str, body_str)
            }
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
            }
            ExprKind::Error => "<error>".to_string(),
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

    /// Create an integer literal expression.
    pub fn integer(value: i64, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::Integer(value)),
            ty: Ty::int(crate::ty::IntBits::I64, span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a float literal expression.
    pub fn float(value: f64, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::Float(value)),
            ty: Ty::float(crate::ty::FloatBits::F64, span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a string literal expression.
    pub fn string(value: String, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::String(value)),
            ty: Ty::string(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create a boolean literal expression.
    pub fn bool(value: bool, span: Span) -> Self {
        Expression {
            id: ExprId::new(),
            kind: ExprKind::Literal(LiteralValue::Bool(value)),
            ty: Ty::bool(span.clone()),
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
            ty: Ty::never(span.clone()),
            span,
            mutable: false,
        }
    }

    /// Create an if expression.
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
        // Compute the type with Never propagation:
        // - No else: type is ()
        // - With else: join the then and else branch types
        let ty = match &else_branch {
            Some(else_br) => {
                let then_ty = compute_block_type(&then_branch, then_value.as_ref(), &span);
                let else_ty = else_br.ty(&span);

                // Join the types - handles Never propagation
                then_ty.join(&else_ty)
            }
            None => Ty::unit(span.clone()),
        };

        Expression {
            id: ExprId::new(),
            kind: ExprKind::If {
                condition: Box::new(condition),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_span::Span;

    #[test]
    fn test_integer_literal() {
        let expr = Expression::integer(42, Span::from(0..2));
        assert!(expr.is_literal());
        assert_eq!(expr.as_literal(), Some(&LiteralValue::Integer(42)));
        assert!(expr.ty.is_int());
    }

    #[test]
    fn test_string_literal() {
        let expr = Expression::string("hello".to_string(), Span::from(0..7));
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
            Expression::integer(1, Span::from(1..2)),
            Expression::integer(2, Span::from(4..5)),
        ];
        let expr = Expression::tuple(elements, Span::from(0..6));
        assert!(expr.ty.is_tuple());
    }

    #[test]
    fn test_error_expression() {
        let expr = Expression::error(Span::from(0..5));
        assert!(expr.is_error());
        assert!(expr.ty.is_error());
    }
}
