//! Values — operand-level reads of places, with ownership and borrow modes
//! folded directly into the variants.
//!
//! Stage 3 of the greenfield memory model: the previous `Value::Place(Place)`
//! was ownership-agnostic, with passing modes carried separately via
//! `CallArg.mode`. Now every value-producing position in the IR records its
//! own mode at the leaf, and `CallArg` / `PassingMode` are gone.
//!
//! Variant choice at each site is driven by the source type's
//! `CopyBehavior`. The verifier (Stage 6) enforces the legality rules; the
//! summary is:
//!
//! - `Value::Move(p)` — ownership transfer, source is dead after.
//! - `Value::Copy(p)` — bitwise copy, source remains valid.
//! - `Value::Ref(p)` / `Value::RefMut(p)` — borrow without transferring
//!   ownership.
//! - `Value::Const(_)` — literal, no place involved.

use crate::immediate::Immediate;
use crate::place::Place;

/// Operand-level read of a place (or a constant).
///
/// Reads carry their ownership/borrow mode inline. See the module docs for
/// the legality rules.
#[derive(Debug, Clone)]
pub enum Value {
    /// `copy <place>` — bitwise read without invalidating the source.
    Copy(Place),
    /// `move <place>` — take ownership of `place`'s value, invalidating the
    /// source. Legal only when the type's `CopyBehavior` is `None`.
    Move(Place),
    /// `&<place>` — immutable borrow.
    Ref(Place),
    /// `&var <place>` — mutable borrow.
    RefMut(Place),
    /// A constant value.
    Const(Immediate),
}

impl Value {
    /// True iff this value reads from a place (any variant except `Const`).
    pub fn is_place_read(&self) -> bool {
        !matches!(self, Value::Const(_))
    }

    /// True iff this value is a constant.
    pub fn is_const(&self) -> bool {
        matches!(self, Value::Const(_))
    }

    /// Return the underlying place for any place-reading variant, else
    /// `None` for `Const`.
    pub fn as_place(&self) -> Option<&Place> {
        match self {
            Value::Copy(p) | Value::Move(p) | Value::Ref(p) | Value::RefMut(p) => Some(p),
            Value::Const(_) => None,
        }
    }

    /// Return the underlying immediate, if this is a `Const`.
    pub fn as_immediate(&self) -> Option<&Immediate> {
        match self {
            Value::Const(i) => Some(i),
            _ => None,
        }
    }

    /// Re-mode a place-reading value as a `Copy`. Constants are unchanged.
    pub fn into_copy(self) -> Value {
        match self {
            Value::Copy(p) | Value::Move(p) | Value::Ref(p) | Value::RefMut(p) => Value::Copy(p),
            Value::Const(_) => self,
        }
    }

    /// Re-mode a place-reading value as a `Move`. Constants are unchanged
    /// (and constructing a Move of a constant is meaningless; the verifier
    /// (Stage 6) will reject Move on non-affine types).
    pub fn into_move(self) -> Value {
        match self {
            Value::Copy(p) | Value::Move(p) | Value::Ref(p) | Value::RefMut(p) => Value::Move(p),
            Value::Const(_) => self,
        }
    }

    /// Re-mode a place-reading value as a `Ref`. Constants are unchanged.
    pub fn into_ref(self) -> Value {
        match self {
            Value::Copy(p) | Value::Move(p) | Value::Ref(p) | Value::RefMut(p) => Value::Ref(p),
            Value::Const(_) => self,
        }
    }

    /// Re-mode a place-reading value as a `RefMut`. Constants are unchanged.
    pub fn into_ref_mut(self) -> Value {
        match self {
            Value::Copy(p) | Value::Move(p) | Value::Ref(p) | Value::RefMut(p) => Value::RefMut(p),
            Value::Const(_) => self,
        }
    }
}

impl From<Immediate> for Value {
    fn from(i: Immediate) -> Self {
        Value::Const(i)
    }
}

/// Convenience: a bare `Place` defaults to `Value::Copy(place)`.
///
/// Used by the MIR builder helpers (`assign_op*`, `branch`, `ret`) where the
/// caller has a `Place` in hand and the operand is a pure read on a
/// Bitwise-copyable type (primitive op args, branch conditions, etc.). Sites
/// that need a different mode must construct the `Value` variant explicitly.
impl From<Place> for Value {
    fn from(p: Place) -> Self {
        Value::Copy(p)
    }
}
