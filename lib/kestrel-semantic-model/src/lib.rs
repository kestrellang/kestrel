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
mod type_oracle;

pub use extension_registry::ExtensionRegistry;
pub use model::SemanticModel;
pub use queries::{
    AncestorOfKind, AssociatedTypeBindingsForStruct, CallableParamTypesForCall, ChildByName,
    ConformancesForSymbol, DeclaredName, DeclaredNamesInScope, ExecutableBodyFor, ExtensionMethods,
    ExtensionsFor, FunctionsInSymbol, GenericsData, GenericsDataFor, HasBody, ImportsInScope,
    InferenceResultFor, InheritedProtocolMember, IsInsideAny, IsVisibleFrom, LocalName,
    PropertyRequirement, ProtocolAssociatedTypesWithDefaults, ProtocolInitializersWithDefiner,
    ProtocolMethodsWithDefiner, ProtocolRequiredInitializers, ProtocolRequiredMethods,
    ProtocolRequiredProperties, ResolveModulePath, ResolveName, ResolveTypePath, ResolveValuePath,
    ResolvedAliasedType, ScopeFor, StructFieldInfo, StructFieldTypeInfo, StructFieldTypes,
    StructFields, StructMethods, SymbolFor, VisibleChildren, VisibleChildrenByName,
};
pub use query::Query;
pub use registry::SymbolRegistry;
pub use resolution::{SymbolResolution, TypePathResolution, ValuePathResolution};
pub use scope::{Import, ImportItem, Scope};
pub use type_oracle::ContextualOracle;
