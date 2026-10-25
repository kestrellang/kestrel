//! Operators: the enums body lowering produces and HIR's operator tables
//! key on (`kestrel_hir::body::BINARY_OP_PROTOCOLS` and friends).
//!
//! Each enum's `symbol()` is the one place that operator's source spelling
//! is written; a test in `kestrel-hir-lower` re-lexes every spelling and
//! requires it to produce exactly the token lowering maps back to it.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    BitNot,
    LogicalNot,
    Pos,
    RangeUpTo,
    RangeThrough,
    /// Prefix `&` — legal only as a `let` initializer (named ref binding,
    /// stage 1.5 item 2); rejected everywhere else at HIR lowering (E488).
    Borrow,
    /// Prefix `&mutating` — the mutable twin of `Borrow`; same gating.
    BorrowMutating,
}

impl UnaryOp {
    /// How this operator is spelled in Kestrel source. See [`BinaryOp::symbol`].
    /// Bare — no trailing space (`not` and `&mutating` need one before their
    /// operand).
    pub fn symbol(&self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::BitNot => "!",
            UnaryOp::LogicalNot => "not",
            UnaryOp::Pos => "+",
            UnaryOp::RangeUpTo => "..<",
            UnaryOp::RangeThrough => "..=",
            UnaryOp::Borrow => "&",
            UnaryOp::BorrowMutating => "&mutating",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PostfixOp {
    Unwrap,
    RangeFrom,
}

impl PostfixOp {
    /// How this operator is spelled in Kestrel source. See [`BinaryOp::symbol`].
    pub fn symbol(&self) -> &'static str {
        match self {
            PostfixOp::Unwrap => "!",
            PostfixOp::RangeFrom => "..",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Coalesce,
    RangeInclusive,
    RangeExclusive,
}

impl BinaryOp {
    /// How this operator is spelled in Kestrel source.
    ///
    /// **The only place operator spellings live.** There was a second copy in
    /// `kestrel-hir-lower::desugar`, and it was wrong in three rows — it wrote
    /// `&&`, `||` and `...` where Kestrel writes `and`, `or` and `..=` — for
    /// text that goes straight into a diagnostic message. Pretty-printing adds
    /// its own spacing; the spelling itself is bare.
    pub fn symbol(&self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Gt => ">",
            BinaryOp::Le => "<=",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
            BinaryOp::Coalesce => "??",
            BinaryOp::RangeInclusive => "..=",
            BinaryOp::RangeExclusive => "..<",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CompoundAssignOp {
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
}

impl CompoundAssignOp {
    /// How this operator is spelled in Kestrel source. See [`BinaryOp::symbol`].
    pub fn symbol(&self) -> &'static str {
        match self {
            CompoundAssignOp::AddAssign => "+=",
            CompoundAssignOp::SubAssign => "-=",
            CompoundAssignOp::MulAssign => "*=",
            CompoundAssignOp::DivAssign => "/=",
            CompoundAssignOp::RemAssign => "%=",
            CompoundAssignOp::BitAndAssign => "&=",
            CompoundAssignOp::BitOrAssign => "|=",
            CompoundAssignOp::BitXorAssign => "^=",
            CompoundAssignOp::ShlAssign => "<<=",
            CompoundAssignOp::ShrAssign => ">>=",
        }
    }
}
