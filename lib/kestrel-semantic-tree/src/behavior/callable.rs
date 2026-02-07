use std::hash::{Hash, Hasher};

use kestrel_span::{Name, Span};
use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::Symbol;

use crate::{
    behavior::KestrelBehaviorKind, language::KestrelLanguage, symbol::protocol::ProtocolSymbol,
    ty::Ty,
};

/// Describes how a method receives its instance (self).
///
/// This determines what operations are allowed on `self` within the method body
/// and what constraints apply when calling the method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReceiverKind {
    /// Immutable borrow of self (default for instance methods)
    /// Syntax: `func method()`
    Borrowing,

    /// Mutable borrow of self
    /// Syntax: `mutating func method()`
    Mutating,

    /// Takes ownership of self (moves it)
    /// Syntax: `consuming func method()`
    Consuming,

    /// For initializers - self is being constructed
    /// Syntax: `init()`
    Initializing,
}

/// Access mode for function parameters.
///
/// Determines how the caller's value is passed and what the callee can do with it.
/// This is distinct from `ReceiverKind` which is only for `self` in methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ParameterAccessMode {
    /// Read-only access (default). Caller retains ownership.
    /// Syntax: `x: T` (no keyword, this is the default)
    #[default]
    Borrow,

    /// Read-write access. Caller retains ownership but must use `var` binding.
    /// Syntax: `mutating x: T`
    Mutating,

    /// Takes ownership (move or copy depending on Copyable).
    /// Syntax: `consuming x: T`
    Consuming,
}

/// Represents a function parameter with optional label for overload resolution.
///
/// Parameters support Swift-style labeled arguments:
/// - `label` is the external name used by callers (optional)
/// - `bind_name` is the internal name used in the function body
/// - `access_mode` determines how the value is passed (borrow/mutating/consuming)
///
/// Examples:
/// - `x: Int` -> access_mode=Borrow, label=None, bind_name="x"
/// - `with x: Int` -> access_mode=Borrow, label="with", bind_name="x"
/// - `mutating x: Int` -> access_mode=Mutating, label=None, bind_name="x"
/// - `consuming point p: Point` -> access_mode=Consuming, label="point", bind_name="p"
/// - `x: Int = 0` -> has_default=true
#[derive(Debug, Clone)]
pub struct CallableParameter {
    /// Access mode for this parameter (borrow/mutating/consuming)
    pub access_mode: ParameterAccessMode,
    /// Optional external label for callers
    pub label: Option<Name>,
    /// Internal binding name used in function body
    pub bind_name: Name,
    /// The parameter's type
    pub ty: Ty,
    /// Whether this parameter has a default value
    pub has_default: bool,
}

impl CallableParameter {
    /// Create a new parameter without a label (default borrow mode, no default value)
    pub fn new(bind_name: Name, ty: Ty) -> Self {
        Self {
            access_mode: ParameterAccessMode::Borrow,
            label: None,
            bind_name,
            ty,
            has_default: false,
        }
    }

    /// Create a new parameter with a label (default borrow mode, no default value)
    pub fn with_label(label: Name, bind_name: Name, ty: Ty) -> Self {
        Self {
            access_mode: ParameterAccessMode::Borrow,
            label: Some(label),
            bind_name,
            ty,
            has_default: false,
        }
    }

    /// Create a new parameter with access mode and no label (no default value)
    pub fn with_access_mode(access_mode: ParameterAccessMode, bind_name: Name, ty: Ty) -> Self {
        Self {
            access_mode,
            label: None,
            bind_name,
            ty,
            has_default: false,
        }
    }

    /// Create a new parameter with access mode and label (no default value)
    pub fn with_access_mode_and_label(
        access_mode: ParameterAccessMode,
        label: Name,
        bind_name: Name,
        ty: Ty,
    ) -> Self {
        Self {
            access_mode,
            label: Some(label),
            bind_name,
            ty,
            has_default: false,
        }
    }

    /// Set whether this parameter has a default value
    pub fn with_default(mut self, has_default: bool) -> Self {
        self.has_default = has_default;
        self
    }

    /// Get the access mode for this parameter
    pub fn access_mode(&self) -> ParameterAccessMode {
        self.access_mode
    }

    /// Get the external label if present.
    ///
    /// Returns None if the parameter has no explicit label (unlabeled parameter).
    /// Unlabeled parameters are called positionally without a label.
    pub fn external_label(&self) -> Option<&str> {
        self.label.as_ref().map(|l| l.value.as_str())
    }

    /// Get the internal binding name
    pub fn internal_name(&self) -> &str {
        &self.bind_name.value
    }

    /// Check if this parameter has an explicit label
    pub fn has_label(&self) -> bool {
        self.label.is_some()
    }

    /// Check if this parameter is mutating
    pub fn is_mutating(&self) -> bool {
        self.access_mode == ParameterAccessMode::Mutating
    }

    /// Check if this parameter is consuming
    pub fn is_consuming(&self) -> bool {
        self.access_mode == ParameterAccessMode::Consuming
    }

    /// Check if this parameter has a default value
    pub fn has_default(&self) -> bool {
        self.has_default
    }
}

/// Uniquely identifies a callable for overload resolution and duplicate detection.
///
/// Two callables with the same signature are considered duplicates and will
/// cause a compilation error.
///
/// The signature consists of:
/// - The callable's name
/// - The labels for each parameter (None = unlabeled positional parameter)
/// - The parameter types (for type-based overloading)
/// - The return type
#[derive(Debug, Clone)]
pub struct CallableSignature {
    /// Name of the callable
    pub name: String,
    /// Labels for each parameter (None = unlabeled positional parameter)
    pub labels: Vec<Option<String>>,
    /// Parameter types for type-based overloading
    pub param_types: Vec<SignatureType>,
    /// Return type
    pub return_type: SignatureType,
}

/// Simplified type representation for signature comparison.
///
/// This is used instead of full `Ty` because:
/// 1. We need Hash + Eq for HashMap-based duplicate detection
/// 2. We only care about structural equality, not spans
/// 3. Unresolved paths are compared by name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignatureType {
    /// Unit type ()
    Unit,
    /// Never type !
    Never,
    /// Boolean
    Bool,
    /// Integer (we ignore bit width for now - could refine later)
    Int,
    /// Float (we ignore bit width for now)
    Float,
    /// String
    String,
    /// Tuple of types
    Tuple(Vec<SignatureType>),
    /// Array of a single element type
    Array(Box<SignatureType>),
    /// Function type
    Function {
        params: Vec<SignatureType>,
        return_type: Box<SignatureType>,
    },
    /// Named type (unresolved path or resolved class/struct)
    Named(Vec<String>),
    /// Unknown/error type
    Unknown,
}

impl SignatureType {
    /// Convert a Ty to a SignatureType for comparison
    pub fn from_ty(ty: &Ty) -> Self {
        use crate::ty::TyKind;

        // Expand type aliases (e.g., OptionalTypeOperator[T] -> Optional[T])
        // so signature comparisons treat aliases as their underlying types.
        let ty = ty.expand_aliases();
        match ty.kind() {
            TyKind::Unit => SignatureType::Unit,
            TyKind::Never => SignatureType::Never,
            TyKind::Bool => SignatureType::Bool,
            TyKind::Int(_) => SignatureType::Int,
            TyKind::Float(_) => SignatureType::Float,
            TyKind::String => SignatureType::String,
            TyKind::Tuple(elements) => {
                SignatureType::Tuple(elements.iter().map(SignatureType::from_ty).collect())
            },
            // Note: Array[T] struct types are handled by the Struct case above
            TyKind::Pointer(_) => {
                // Pointer types are represented as a named type for signature matching
                SignatureType::Named(vec!["lang".to_string(), "ptr".to_string()])
            },
            TyKind::Function {
                params,
                return_type,
            } => SignatureType::Function {
                params: params.iter().map(SignatureType::from_ty).collect(),
                return_type: Box::new(SignatureType::from_ty(return_type)),
            },
            TyKind::Error => SignatureType::Named(vec!["<error>".to_string()]),
            TyKind::SelfType => SignatureType::Named(vec!["Self".to_string()]),
            TyKind::Infer => SignatureType::Named(vec!["_".to_string()]),
            TyKind::TypeParameter(param) => {
                SignatureType::Named(vec![param.metadata().name().value.clone()])
            },
            TyKind::Struct { symbol, .. } => {
                SignatureType::Named(vec![symbol.metadata().name().value.clone()])
            },
            TyKind::Enum { symbol, .. } => {
                SignatureType::Named(vec![symbol.metadata().name().value.clone()])
            },
            TyKind::Protocol {
                symbol,
                substitutions,
            } => {
                // Include type arguments in the signature to distinguish
                // Conv[Int8] from Conv[Int32]
                let mut parts = vec![symbol.metadata().name().value.clone()];
                // Add type arguments in order of the protocol's type parameters
                for type_param in symbol.type_parameters() {
                    if let Some(sub_ty) = substitutions.get(type_param.metadata().id()) {
                        // Recursively convert the substitution type
                        let sub_sig = SignatureType::from_ty(sub_ty);
                        parts.push(format!("{:?}", sub_sig));
                    }
                }
                SignatureType::Named(parts)
            },
            TyKind::TypeAlias { symbol, .. } => {
                // For type aliases, use the alias name
                // (could also resolve to underlying type)
                SignatureType::Named(vec![symbol.metadata().name().value.clone()])
            },
            TyKind::AssociatedType { symbol, container } => {
                // For associated types, include the protocol qualification to distinguish
                // between protocols with the same associated type name (e.g., Addable.Output
                // vs RangeConstructible.Output)
                let assoc_name = symbol.metadata().name().value.clone();

                // Try to get the protocol name from the container or the symbol's parent
                let protocol_name = match container {
                    Some(container_ty) => match container_ty.kind() {
                        TyKind::Protocol {
                            symbol: proto_sym, ..
                        } => Some(proto_sym.metadata().name().value.clone()),
                        TyKind::SelfType => {
                            // For Self type, look up the associated type's defining protocol
                            symbol
                                .metadata()
                                .parent()
                                .and_then(|p| p.downcast_arc::<ProtocolSymbol>().ok())
                                .map(|p| p.metadata().name().value.clone())
                        },
                        _ => None,
                    },
                    None => {
                        // No container - associated type used within its defining protocol
                        // Look up the parent protocol
                        symbol
                            .metadata()
                            .parent()
                            .and_then(|p| p.downcast_arc::<ProtocolSymbol>().ok())
                            .map(|p| p.metadata().name().value.clone())
                    },
                };

                match protocol_name {
                    Some(proto) => SignatureType::Named(vec![proto, assoc_name]),
                    None => SignatureType::Named(vec![assoc_name]),
                }
            },
            TyKind::UnresolvedFunction { return_type, .. } => {
                // Treat as a function type with unknown params
                SignatureType::Function {
                    params: vec![], // Unknown params
                    return_type: Box::new(SignatureType::from_ty(return_type)),
                }
            },
            TyKind::UnresolvedPath { segments } => {
                // Unresolved path - use the path segments as the name
                SignatureType::Named(segments.clone())
            },
        }
    }
}

impl PartialEq for CallableSignature {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.labels == other.labels
            && self.param_types == other.param_types
            && self.return_type == other.return_type
    }
}

impl Eq for CallableSignature {}

impl Hash for CallableSignature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.labels.hash(state);
        self.param_types.hash(state);
        self.return_type.hash(state);
    }
}

/// Lookup key for method matching without return type.
///
/// Used when we want to find a method by name/labels/params and then
/// separately validate the return type for better error messages.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodLookupKey {
    /// Name of the method
    pub name: String,
    /// Labels for each parameter
    pub labels: Vec<Option<String>>,
    /// Parameter types
    pub param_types: Vec<SignatureType>,
}

/// Key for duplicate callable detection.
///
/// In Kestrel, overloading is label-based only - two callables with the same
/// name and labels are duplicates regardless of parameter/return types.
///
/// Examples:
/// - `func foo(x: Int)` and `func foo(x: String)` → DUPLICATES (same name + labels)
/// - `func foo(x: Int)` and `func foo(y: Int)` → Valid overloads (different labels)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DuplicateKey {
    /// Name of the callable
    pub name: String,
    /// Labels for each parameter (None = unlabeled positional parameter)
    pub labels: Vec<Option<String>>,
}

impl DuplicateKey {
    /// Create a new duplicate key
    pub fn new(name: String, labels: Vec<Option<String>>) -> Self {
        Self { name, labels }
    }

    /// Format the key for display in error messages
    pub fn display(&self) -> String {
        let params: Vec<String> = self
            .labels
            .iter()
            .map(|label| match label {
                Some(l) => format!("{}:", l),
                None => "_:".to_string(),
            })
            .collect();

        format!("{}({})", self.name, params.join(", "))
    }
}

impl CallableSignature {
    /// Create a new signature
    pub fn new(
        name: String,
        labels: Vec<Option<String>>,
        param_types: Vec<SignatureType>,
        return_type: SignatureType,
    ) -> Self {
        Self {
            name,
            labels,
            param_types,
            return_type,
        }
    }

    /// Get a lookup key for matching by name/labels/params only (ignoring return type)
    pub fn lookup_key(&self) -> MethodLookupKey {
        MethodLookupKey {
            name: self.name.clone(),
            labels: self.labels.clone(),
            param_types: self.param_types.clone(),
        }
    }

    /// Get the arity (number of parameters)
    pub fn arity(&self) -> usize {
        self.param_types.len()
    }

    /// Format the signature for display in error messages
    pub fn display(&self) -> String {
        let params: Vec<String> = self
            .labels
            .iter()
            .zip(self.param_types.iter())
            .map(|(label, ty)| match label {
                Some(l) => format!("{}: {:?}", l, ty),
                None => format!("_: {:?}", ty),
            })
            .collect();

        format!("{}({})", self.name, params.join(", "))
    }
}

/// CallableBehavior represents callable semantics that can be attached to symbols.
///
/// This behavior is used for:
/// - Functions (standalone and methods)
/// - Initializers (future)
/// - Closures (future)
///
/// It provides:
/// - Parameter information with labels for overload resolution
/// - Return type for type checking
/// - Receiver kind for instance methods
/// - Signature generation for duplicate detection
#[derive(Debug, Clone)]
pub struct CallableBehavior {
    /// The callable's parameters
    parameters: Vec<CallableParameter>,
    /// The return type
    return_type: Ty,
    /// The receiver kind (None for static/free functions, Some for instance methods)
    receiver: Option<ReceiverKind>,
    /// The span covering the entire callable declaration
    span: Span,
}

impl Behavior<KestrelLanguage> for CallableBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Callable
    }
}

impl CallableBehavior {
    /// Create a new CallableBehavior for a static/free function (no receiver)
    pub fn new(parameters: Vec<CallableParameter>, return_type: Ty, span: Span) -> Self {
        Self {
            parameters,
            return_type,
            receiver: None,
            span,
        }
    }

    /// Create a new CallableBehavior with a receiver (instance method)
    pub fn with_receiver(
        parameters: Vec<CallableParameter>,
        return_type: Ty,
        receiver: ReceiverKind,
        span: Span,
    ) -> Self {
        Self {
            parameters,
            return_type,
            receiver: Some(receiver),
            span,
        }
    }

    /// Get the parameters
    pub fn parameters(&self) -> &[CallableParameter] {
        &self.parameters
    }

    /// Get the number of required parameters (those without default values).
    pub fn required_parameter_count(&self) -> usize {
        self.parameters.iter().filter(|p| !p.has_default()).count()
    }

    /// Check if the given argument count is compatible with this callable's parameters,
    /// accounting for default parameter values.
    pub fn arity_matches(&self, argument_count: usize) -> bool {
        argument_count >= self.required_parameter_count() && argument_count <= self.parameters.len()
    }

    /// Get the return type
    pub fn return_type(&self) -> &Ty {
        &self.return_type
    }

    /// Get the span
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Get the receiver kind (None for static/free functions)
    pub fn receiver(&self) -> Option<ReceiverKind> {
        self.receiver
    }

    /// Check if this is an instance method (has a receiver)
    pub fn is_instance_method(&self) -> bool {
        self.receiver.is_some()
    }

    /// Check if this is a static/free function (no receiver)
    pub fn is_static(&self) -> bool {
        self.receiver.is_none()
    }

    /// Get the arity (number of parameters)
    pub fn arity(&self) -> usize {
        self.parameters.len()
    }

    /// Generate a signature for this callable with the given name.
    ///
    /// The signature is used for:
    /// - Duplicate detection (same signature = error)
    /// - Overload resolution (different signatures = valid overloads)
    pub fn signature(&self, name: &str) -> CallableSignature {
        let labels: Vec<Option<String>> = self
            .parameters
            .iter()
            .map(|p| p.external_label().map(|s| s.to_string()))
            .collect();

        let param_types: Vec<SignatureType> = self
            .parameters
            .iter()
            .map(|p| SignatureType::from_ty(&p.ty))
            .collect();

        let return_type = SignatureType::from_ty(&self.return_type);

        CallableSignature::new(name.to_string(), labels, param_types, return_type)
    }

    /// Generate a key for duplicate detection (name + labels only).
    ///
    /// In Kestrel, overloading is label-based - two callables with the same
    /// name and labels are duplicates regardless of types.
    pub fn duplicate_key(&self, name: &str) -> DuplicateKey {
        let labels: Vec<Option<String>> = self
            .parameters
            .iter()
            .map(|p| p.external_label().map(|s| s.to_string()))
            .collect();

        DuplicateKey::new(name.to_string(), labels)
    }

    /// Get the function type representation of this callable
    pub fn function_type(&self) -> Ty {
        let param_types: Vec<Ty> = self.parameters.iter().map(|p| p.ty.clone()).collect();
        Ty::function(param_types, self.return_type.clone(), self.span.clone())
    }

    /// Get parameter labels for display/debugging
    pub fn parameter_labels(&self) -> Vec<Option<&str>> {
        self.parameters.iter().map(|p| p.external_label()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_span::Span;
    use kestrel_span::Spanned;

    fn make_name(s: &str) -> Name {
        Spanned::new(s.to_string(), Span::new(0, 0..s.len()))
    }

    #[test]
    fn test_signature_equality_same_unlabeled() {
        // Two unlabeled parameters with same types = same signature
        let sig1 = CallableSignature::new(
            "add".to_string(),
            vec![None, None],
            vec![SignatureType::Int, SignatureType::Int],
            SignatureType::Int,
        );
        let sig2 = CallableSignature::new(
            "add".to_string(),
            vec![None, None],
            vec![SignatureType::Int, SignatureType::Int],
            SignatureType::Int,
        );

        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_signature_equality_same_labeled() {
        // Two labeled parameters with same labels and types = same signature
        let sig1 = CallableSignature::new(
            "add".to_string(),
            vec![Some("x".to_string()), Some("y".to_string())],
            vec![SignatureType::Int, SignatureType::Int],
            SignatureType::Int,
        );
        let sig2 = CallableSignature::new(
            "add".to_string(),
            vec![Some("x".to_string()), Some("y".to_string())],
            vec![SignatureType::Int, SignatureType::Int],
            SignatureType::Int,
        );

        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_signature_different_labels() {
        let sig1 = CallableSignature::new(
            "greet".to_string(),
            vec![Some("with".to_string())],
            vec![SignatureType::Named(vec!["String".to_string()])],
            SignatureType::Unit,
        );
        let sig2 = CallableSignature::new(
            "greet".to_string(),
            vec![Some("using".to_string())],
            vec![SignatureType::Named(vec!["String".to_string()])],
            SignatureType::Unit,
        );

        assert_ne!(sig1, sig2); // Different labels = different signatures
    }

    #[test]
    fn test_signature_labeled_vs_unlabeled() {
        // Labeled vs unlabeled = different signatures
        let sig1 = CallableSignature::new(
            "foo".to_string(),
            vec![Some("x".to_string())],
            vec![SignatureType::Int],
            SignatureType::Unit,
        );
        let sig2 = CallableSignature::new(
            "foo".to_string(),
            vec![None],
            vec![SignatureType::Int],
            SignatureType::Unit,
        );

        assert_ne!(sig1, sig2); // Labeled vs unlabeled = different signatures
    }

    #[test]
    fn test_signature_different_types() {
        let sig1 = CallableSignature::new(
            "add".to_string(),
            vec![None, None],
            vec![SignatureType::Int, SignatureType::Int],
            SignatureType::Int,
        );
        let sig2 = CallableSignature::new(
            "add".to_string(),
            vec![None, None],
            vec![SignatureType::Float, SignatureType::Float],
            SignatureType::Float,
        );

        assert_ne!(sig1, sig2); // Different types = different signatures
    }

    #[test]
    fn test_signature_different_arity() {
        let sig1 = CallableSignature::new(
            "add".to_string(),
            vec![None],
            vec![SignatureType::Int],
            SignatureType::Int,
        );
        let sig2 = CallableSignature::new(
            "add".to_string(),
            vec![None, None],
            vec![SignatureType::Int, SignatureType::Int],
            SignatureType::Int,
        );

        assert_ne!(sig1, sig2); // Different arity = different signatures
    }

    #[test]
    fn test_callable_behavior_signature_unlabeled() {
        use crate::ty::IntBits;
        // Unlabeled parameters have None for labels
        let params = vec![
            CallableParameter::new(make_name("x"), Ty::int(IntBits::I64, Span::new(0, 0..3))),
            CallableParameter::new(make_name("y"), Ty::int(IntBits::I64, Span::new(0, 5..8))),
        ];
        let return_ty = Ty::int(IntBits::I64, Span::new(0, 13..16));
        let behavior = CallableBehavior::new(params, return_ty, Span::new(0, 0..20));

        let sig = behavior.signature("add");

        assert_eq!(sig.name, "add");
        // Unlabeled params have None for labels
        assert_eq!(sig.labels, vec![None, None]);
        assert_eq!(sig.arity(), 2);
    }

    #[test]
    fn test_callable_with_labels() {
        let params = vec![CallableParameter::with_label(
            make_name("with"),
            make_name("name"),
            Ty::string(Span::new(0, 0..6)),
        )];
        let return_ty = Ty::unit(Span::new(0, 10..12));
        let behavior = CallableBehavior::new(params, return_ty, Span::new(0, 0..15));

        let sig = behavior.signature("greet");

        assert_eq!(sig.name, "greet");
        assert_eq!(sig.labels, vec![Some("with".to_string())]); // Uses label, not bind_name
    }
}
