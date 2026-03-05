//! Marker behaviors for HECS kind groups.
//!
//! These are empty struct behaviors whose presence on a symbol indicates
//! membership in a conceptual group. They replace `match` arms on
//! `KestrelSymbolKind` combinations with component-based queries.

use semantic_tree::behavior::Behavior;

use crate::behavior::KestrelBehaviorKind;
use crate::language::KestrelLanguage;

/// Marker: symbol is a concrete instantiable type (Struct, Enum).
#[derive(Debug, Clone)]
pub struct ConcreteTypeMarker;

impl Behavior<KestrelLanguage> for ConcreteTypeMarker {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::ConcreteType
    }
}

/// Marker: symbol can have instance members (Struct, Protocol).
#[derive(Debug, Clone)]
pub struct HasMembersMarker;

impl Behavior<KestrelLanguage> for HasMembersMarker {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::HasMembers
    }
}

/// Marker: symbol is a namespace scope (Module, SourceFile).
#[derive(Debug, Clone)]
pub struct NamespaceScopeMarker;

impl Behavior<KestrelLanguage> for NamespaceScopeMarker {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::NamespaceScope
    }
}

/// Marker: symbol is a property accessor (Getter, Setter).
#[derive(Debug, Clone)]
pub struct AccessorMarker;

impl Behavior<KestrelLanguage> for AccessorMarker {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Accessor
    }
}

/// Marker: symbol is a scope that can contain callable declarations
/// (Module, Struct, SourceFile, Protocol, Enum, Extension).
#[derive(Debug, Clone)]
pub struct CallableScopeMarker;

impl Behavior<KestrelLanguage> for CallableScopeMarker {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::CallableScope
    }
}

/// Marker: symbol can contain instance methods
/// (Struct, Enum, Protocol, Extension).
#[derive(Debug, Clone)]
pub struct MethodContainerMarker;

impl Behavior<KestrelLanguage> for MethodContainerMarker {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::MethodContainer
    }
}

/// Marker: symbol can be the parent of a property accessor
/// (Struct, Enum, Extension).
#[derive(Debug, Clone)]
pub struct AccessorParentMarker;

impl Behavior<KestrelLanguage> for AccessorParentMarker {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::AccessorParent
    }
}
