//! Values — either places or immediates.

use crate::immediate::Immediate;
use crate::place::Place;

/// A value is either a place (memory location) or an immediate (constant).
///
/// No `Unreachable` variant — divergence is represented at the terminator level.
#[derive(Debug, Clone)]
pub enum Value {
    /// A memory location.
    Place(Place),
    /// A constant value.
    Immediate(Immediate),
}

impl Value {
    pub fn is_place(&self) -> bool {
        matches!(self, Value::Place(_))
    }

    pub fn is_immediate(&self) -> bool {
        matches!(self, Value::Immediate(_))
    }

    pub fn as_place(&self) -> Option<&Place> {
        match self {
            Value::Place(p) => Some(p),
            _ => None,
        }
    }

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
