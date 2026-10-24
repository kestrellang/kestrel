//! Existential function tables (existentials plan Phase 10).
//!
//! mir-lower writes one [`VTableTemplate`] per table *shape*: a single
//! protocol in a given box, or a composition of bounds. A template is generic
//! over the payload type; [`crate::ImmediateKind::VTable`] names one instance
//! (template + payload), which mono instantiates like a generic function and
//! codegen emits as a read-only global of pointer-sized words.
//!
//! Every table starts with the same header, so `drop`/`size`/`align` work on
//! any table pointer without knowing its shape:
//!
//! | word | entry |
//! |------|-------|
//! | 0 | drop thunk (its address doubles as the payload's type identity) |
//! | 1 | payload size |
//! | 2 | payload align |
//! | 3 | clone thunk, or null when the existential isn't `Cloneable` |
//!
//! A single-protocol table then holds a pointer to each declared ancestor's
//! table (canonical order), then the protocol's own dispatchable
//! requirements in [`kestrel_semantics::ExistentialLayout`] order. A
//! composition table holds a pointer to each bound's table instead.

use kestrel_hecs::Entity;

/// Words before a table's shape-specific part.
pub const VTABLE_HEADER_WORDS: u32 = 4;
/// Header word: drop thunk / type identity.
pub const VTABLE_DROP: u32 = 0;
/// Header word: payload size in bytes.
pub const VTABLE_SIZE: u32 = 1;
/// Header word: payload alignment in bytes.
pub const VTABLE_ALIGN: u32 = 2;
/// Header word: clone thunk (null when not `Cloneable`).
pub const VTABLE_CLONE: u32 = 3;

/// One table shape, generic over the payload.
#[derive(Debug, Clone, PartialEq)]
pub struct VTableTemplate {
    pub entity: Entity,
    pub name: String,
    pub entries: Vec<VTableEntry>,
}

/// One word of a table.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VTableEntry {
    /// A function generic over exactly one type param (the payload),
    /// instantiated at the table's payload.
    Func(Entity),
    /// The payload's size.
    Size,
    /// The payload's alignment.
    Align,
    /// Another template's table over the same payload (an ancestor's or a
    /// composition bound's).
    Table(Entity),
    /// A null word.
    Null,
}
