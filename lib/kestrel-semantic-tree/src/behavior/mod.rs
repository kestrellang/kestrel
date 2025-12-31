pub mod attributes;
pub mod callable;
pub mod conformances;
pub mod conforms_to;
pub mod copy_semantics;
pub mod executable;
pub mod extension_target;
pub mod function_data;
pub mod generics;
pub mod implements;
pub mod member_access;
pub mod typed;
pub mod valued;
pub mod visibility;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KestrelBehaviorKind {
    AssociatedTypeBounds,
    Attributes,
    Callable,
    Conformances,
    ConformsTo,
    CopySemantics,
    Executable,
    ExtensionTarget,
    FlattenedProtocol,
    FunctionData,
    Generics,
    Implements,
    ImportData,
    MemberAccess,
    ResolvedExecutable,
    Typed,
    TypeAliasTyped,
    Valued,
    Visibility,
}
