//! Semantic model types for Kestrel compiler
//!
//! This crate provides foundational types for semantic analysis including:
//! - Scope and import representation
//! - Resolution result types
//! - Symbol and extension registries
//! - SemanticModel for querying semantic information

mod extension_registry;
mod model;
pub mod queries;
mod query;
mod registry;
mod resolution;
mod scope;
mod ty_cache_key;
mod type_oracle;
pub mod type_resolution;

pub use extension_registry::ExtensionRegistry;
pub use model::SemanticModel;
pub use queries::{
    AllConformancesFor, AllInitializersFor, AllMethodsFor, AncestorOfKind,
    AssociatedTypeBindingsFor, AssociatedTypeBoundsInContext, ChildByName, ConcreteSelfType,
    ConformancesForSymbol, ConformsToQuery, CopySemanticsFor, DeclaredName, DeclaredNamesInScope,
    DeinitFor, ExecutableBodyFor, ExtensionBoundsForParam, ExtensionMethods, ExtensionTargetFor,
    ExtensionsFor, FunctionsInSymbol, GenericsData, GenericsDataFor, HasBody, ImportsInScope,
    InferenceResultFor, InheritedProtocolMember, IsInsideAny, IsMarkerProtocol, IsVisibleFrom,
    LocalName, PropertyRequirement, ProtocolAssociatedTypesWithDefaults,
    ProtocolConformancesForType, ProtocolInitializersWithDefiner, ProtocolMethodsWithDefiner,
    ProtocolRequiredInitializers, ProtocolRequiredMethods, ProtocolRequiredProperties,
    ResolveModulePath, ResolveName, ResolveTypePath, ResolveValuePath, ResolvedAliasedType,
    ScopeFor, SelfProtocolBounds, StructFieldInfo, StructFieldTypeInfo, StructFieldTypes,
    StructFields, StructMethods, SymbolFor, TypeFor, TypeParameterBounds, VisibilityLevel,
    VisibilityLevelOf, VisibleChildren, VisibleChildrenByName, WhereClausesInScope,
    callable_param_types_for_call,
};
pub use query::Query;
pub use registry::SymbolRegistry;
pub use resolution::{SymbolResolution, TypePathResolution, ValuePathResolution};
pub use scope::{Import, ImportItem, Scope};
pub use ty_cache_key::TyCacheKey;
pub use type_oracle::{ContextualOracle, resolve_all_associated_types};
