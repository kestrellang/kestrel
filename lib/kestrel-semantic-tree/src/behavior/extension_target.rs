use std::sync::Arc;

use semantic_tree::behavior::Behavior;

use crate::{
    behavior::KestrelBehaviorKind,
    language::KestrelLanguage,
    symbol::type_parameter::TypeParameterSymbol,
    ty::{Ty, WhereClause},
};

/// ExtensionTargetBehavior stores the resolved target type and constraints for an extension.
///
/// This behavior is added during the BIND phase after resolving the target type expression.
///
/// # Type Arguments
///
/// Extensions use type expressions (not type parameter lists) to reference the target.
/// For `extend Box[T, Int]`:
/// - `target_type` is `Box[T, Int]` (the full instantiated type)
/// - `type_arguments` are `[T, Int]` (the arguments used)
/// - `referenced_type_parameters` contains just `T` (references to struct's params)
///
/// # Where Clause
///
/// The where clause combines:
/// - Inherited constraints from the target struct
/// - Additional constraints declared on the extension
#[derive(Debug, Clone)]
pub struct ExtensionTargetBehavior {
    /// The fully resolved target type (e.g., Box[T, Int])
    target_type: Ty,

    /// The type arguments used in the extension target (e.g., [T, Int])
    type_arguments: Vec<Ty>,

    /// Type parameter symbols referenced by this extension
    /// These are references to the target struct's type parameters
    referenced_type_parameters: Vec<Arc<TypeParameterSymbol>>,

    /// Combined where clause (inherited + extension's own)
    where_clause: WhereClause,
}

impl Behavior<KestrelLanguage> for ExtensionTargetBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::ExtensionTarget
    }
}

impl ExtensionTargetBehavior {
    /// Create a new ExtensionTargetBehavior
    pub fn new(
        target_type: Ty,
        type_arguments: Vec<Ty>,
        referenced_type_parameters: Vec<Arc<TypeParameterSymbol>>,
        where_clause: WhereClause,
    ) -> Self {
        ExtensionTargetBehavior {
            target_type,
            type_arguments,
            referenced_type_parameters,
            where_clause,
        }
    }

    /// Get the fully resolved target type
    pub fn target_type(&self) -> &Ty {
        &self.target_type
    }

    /// Get the type arguments used in the extension target
    pub fn type_arguments(&self) -> &[Ty] {
        &self.type_arguments
    }

    /// Get the type parameter symbols referenced by this extension
    pub fn referenced_type_parameters(&self) -> &[Arc<TypeParameterSymbol>] {
        &self.referenced_type_parameters
    }

    /// Get the combined where clause
    pub fn where_clause(&self) -> &WhereClause {
        &self.where_clause
    }

    /// Calculate specificity (number of concrete type arguments)
    ///
    /// Higher specificity means the extension is more specialized.
    /// Used to resolve conflicts between overlapping extensions.
    pub fn specificity(&self) -> usize {
        self.type_arguments
            .iter()
            .filter(|ty| !ty.is_type_parameter())
            .count()
    }

    /// Check if this extension is fully generic (all type params)
    pub fn is_fully_generic(&self) -> bool {
        self.type_arguments.iter().all(|ty| ty.is_type_parameter())
    }

    /// Check if this extension is fully specialized (no type params)
    pub fn is_fully_specialized(&self) -> bool {
        self.type_arguments.iter().all(|ty| !ty.is_type_parameter())
    }
}
