//! Values (places or immediates).

use crate::MirContext;
use crate::function::{Immediate, Place};
use std::fmt;

/// A value is either a place, an immediate, or unreachable (diverged).
#[derive(Debug, Clone)]
pub enum Value {
    /// A memory location.
    Place(Place),
    /// A constant value.
    Immediate(Immediate),
    /// The expression diverged (return/break/continue) and never produces a value.
    /// This should not be used in assignments - callers should check for this variant.
    Unreachable,
}

impl Value {
    /// Create a display wrapper for printing this value.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        ValueDisplay { value: self, ctx }
    }

    /// Check if this is a place.
    pub fn is_place(&self) -> bool {
        matches!(self, Value::Place(_))
    }

    /// Check if this is an immediate.
    pub fn is_immediate(&self) -> bool {
        matches!(self, Value::Immediate(_))
    }

    /// Check if this is an unreachable value (expression diverged).
    pub fn is_unreachable(&self) -> bool {
        matches!(self, Value::Unreachable)
    }

    /// Get the place if this is a place value.
    pub fn as_place(&self) -> Option<&Place> {
        match self {
            Value::Place(p) => Some(p),
            _ => None,
        }
    }

    /// Get the immediate if this is an immediate value.
    pub fn as_immediate(&self) -> Option<&Immediate> {
        match self {
            Value::Immediate(i) => Some(i),
            _ => None,
        }
    }
}

impl From<Place> for Value {
    fn from(p: Place) -> Self {
        Value::Place(p)
    }
}

impl From<Immediate> for Value {
    fn from(i: Immediate) -> Self {
        Value::Immediate(i)
    }
}

struct ValueDisplay<'a> {
    value: &'a Value,
    ctx: &'a MirContext,
}

impl fmt::Display for ValueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value {
            Value::Place(p) => write!(f, "{}", p.display(self.ctx)),
            Value::Immediate(i) => write!(f, "{}", i.display(self.ctx)),
            Value::Unreachable => write!(f, "<unreachable>"),
        }
    }
}
