pub mod callable;
pub mod conformances;
pub mod conforms_to;
pub mod executable;
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
    Callable,
    Conformances,
    ConformsTo,
    Executable,
    FlattenedProtocol,
    FunctionData,
    Generics,
    Implements,
    ImportData,
    MemberAccess,
    Typed,
    TypeAliasTyped,
    Valued,
    Visibility,
}
