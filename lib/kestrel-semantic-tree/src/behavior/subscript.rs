//! Subscript behavior for subscript declarations.
//!
//! This behavior stores parameter information for overload resolution.
//! It is attached to `SubscriptSymbol` during the bind phase.

use semantic_tree::behavior::Behavior;

use crate::behavior::callable::{CallableParameter, ReceiverKind};
use crate::behavior::KestrelBehaviorKind;
use crate::language::KestrelLanguage;
use crate::ty::Ty;

/// Behavior for subscript declarations - enables overload resolution.
///
/// Subscripts can be overloaded by parameter labels and types. This behavior
/// stores the subscript's signature information for matching against call sites.
///
/// # Fields
///
/// - `parameters` - The subscript's parameters (used for overload resolution)
/// - `return_type` - The type returned by the subscript getter
/// - `receiver` - How the subscript receives `self` (borrowing for getter, mutating for setter)
///
/// # Examples
///
/// ```kestrel
/// // Two overloaded subscripts with different labels
/// subscript(index: Int) -> T { ... }
/// subscript(safe index: Int) -> Optional[T] { ... }
/// ```
///
/// The first subscript has no label, the second has label "safe".
/// When resolving `array(0)` vs `array(safe: 0)`, the labels determine which
/// subscript to call.
#[derive(Debug, Clone)]
pub struct SubscriptBehavior {
    /// The subscript's parameters
    parameters: Vec<CallableParameter>,
    /// The return type of the subscript
    return_type: Ty,
    /// The receiver kind (borrowing for instance subscripts, None for static)
    receiver: Option<ReceiverKind>,
}

impl SubscriptBehavior {
    /// Create a new SubscriptBehavior for a static subscript (no receiver)
    pub fn new(parameters: Vec<CallableParameter>, return_type: Ty) -> Self {
        Self {
            parameters,
            return_type,
            receiver: None,
        }
    }

    /// Create a new SubscriptBehavior with a receiver (instance subscript)
    pub fn with_receiver(
        parameters: Vec<CallableParameter>,
        return_type: Ty,
        receiver: ReceiverKind,
    ) -> Self {
        Self {
            parameters,
            return_type,
            receiver: Some(receiver),
        }
    }

    /// Get the parameters
    pub fn parameters(&self) -> &[CallableParameter] {
        &self.parameters
    }

    /// Get the return type
    pub fn return_type(&self) -> &Ty {
        &self.return_type
    }

    /// Get the receiver kind (None for static subscripts)
    pub fn receiver(&self) -> Option<ReceiverKind> {
        self.receiver
    }

    /// Check if this is an instance subscript (has a receiver)
    pub fn is_instance(&self) -> bool {
        self.receiver.is_some()
    }

    /// Check if this is a static subscript (no receiver)
    pub fn is_static(&self) -> bool {
        self.receiver.is_none()
    }

    /// Get the arity (number of parameters)
    pub fn arity(&self) -> usize {
        self.parameters.len()
    }

    /// Check if this subscript matches the given argument labels.
    ///
    /// This is used for overload resolution. Two subscripts with different
    /// labels are considered different overloads.
    ///
    /// # Arguments
    /// * `labels` - The labels from the call site (None for unlabeled arguments)
    ///
    /// # Returns
    /// * `true` if the labels match this subscript's parameter labels
    /// * `false` otherwise
    pub fn matches_labels(&self, labels: &[Option<&str>]) -> bool {
        if labels.len() != self.parameters.len() {
            return false;
        }

        for (arg_label, param) in labels.iter().zip(&self.parameters) {
            let param_label = param.external_label();
            if *arg_label != param_label {
                return false;
            }
        }

        true
    }
}

impl Behavior<KestrelLanguage> for SubscriptBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Subscript
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_span::{Name, Span, Spanned};

    fn make_name(s: &str) -> Name {
        Spanned::new(s.to_string(), Span::from(0..s.len()))
    }

    #[test]
    fn test_subscript_behavior_static() {
        use crate::ty::IntBits;
        let params = vec![CallableParameter::new(
            make_name("index"),
            Ty::int(IntBits::I64, Span::from(0..3)),
        )];
        let return_ty = Ty::int(IntBits::I64, Span::from(10..13));
        let behavior = SubscriptBehavior::new(params, return_ty);

        assert!(behavior.is_static());
        assert!(!behavior.is_instance());
        assert_eq!(behavior.arity(), 1);
        assert!(behavior.receiver().is_none());
    }

    #[test]
    fn test_subscript_behavior_instance() {
        use crate::ty::IntBits;
        let params = vec![CallableParameter::new(
            make_name("index"),
            Ty::int(IntBits::I64, Span::from(0..3)),
        )];
        let return_ty = Ty::int(IntBits::I64, Span::from(10..13));
        let behavior =
            SubscriptBehavior::with_receiver(params, return_ty, ReceiverKind::Borrowing);

        assert!(!behavior.is_static());
        assert!(behavior.is_instance());
        assert_eq!(behavior.receiver(), Some(ReceiverKind::Borrowing));
    }

    #[test]
    fn test_matches_labels_unlabeled() {
        use crate::ty::IntBits;
        let params = vec![CallableParameter::new(
            make_name("index"),
            Ty::int(IntBits::I64, Span::from(0..3)),
        )];
        let return_ty = Ty::int(IntBits::I64, Span::from(10..13));
        let behavior = SubscriptBehavior::new(params, return_ty);

        // Unlabeled parameter matches unlabeled argument
        assert!(behavior.matches_labels(&[None]));
        // Labeled argument does not match
        assert!(!behavior.matches_labels(&[Some("safe")]));
    }

    #[test]
    fn test_matches_labels_labeled() {
        use crate::ty::IntBits;
        let params = vec![CallableParameter::with_label(
            make_name("safe"),
            make_name("index"),
            Ty::int(IntBits::I64, Span::from(0..3)),
        )];
        let return_ty = Ty::int(IntBits::I64, Span::from(10..13));
        let behavior = SubscriptBehavior::new(params, return_ty);

        // Matching label
        assert!(behavior.matches_labels(&[Some("safe")]));
        // Non-matching label
        assert!(!behavior.matches_labels(&[Some("unchecked")]));
        // Unlabeled argument does not match
        assert!(!behavior.matches_labels(&[None]));
    }

    #[test]
    fn test_matches_labels_wrong_arity() {
        use crate::ty::IntBits;
        let params = vec![
            CallableParameter::new(make_name("row"), Ty::int(IntBits::I64, Span::from(0..3))),
            CallableParameter::new(make_name("col"), Ty::int(IntBits::I64, Span::from(5..8))),
        ];
        let return_ty = Ty::int(IntBits::I64, Span::from(10..13));
        let behavior = SubscriptBehavior::new(params, return_ty);

        // Wrong number of arguments
        assert!(!behavior.matches_labels(&[None]));
        assert!(!behavior.matches_labels(&[None, None, None]));
        // Correct arity
        assert!(behavior.matches_labels(&[None, None]));
    }
}
