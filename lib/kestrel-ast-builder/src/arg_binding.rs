//! Argument-to-parameter binding — the single source of truth for mapping a
//! call's arguments onto a callable's parameters.
//!
//! Kestrel matches arguments to parameters in **declaration order**: provided
//! arguments may not be reordered, but any parameter that has a default value
//! may be **skipped** (at the front, middle, or end). A labeled argument binds
//! to the next parameter carrying that label; a positional (unlabeled) argument
//! binds to the next positional parameter. Parameters skipped along the way must
//! be defaultable.
//!
//! This module is consumed by overload selection and label/type checking
//! (`kestrel-type-infer`) and by call lowering (`kestrel-mir-lower`), so all
//! stages agree on exactly which argument fills which parameter slot. It is a
//! pure function — NOT a query: its input includes the call site's argument
//! labels (ephemeral, high-cardinality), so memoization would never hit.

/// A parameter as seen by argument binding: its (optional) label and whether it
/// carries a default value. Adapted from `AstParam` (via `default_entity`) or
/// type-infer's `ParamInfo` (via `has_default`).
#[derive(Clone, Copy, Debug)]
pub struct BindParam<'a> {
    pub label: Option<&'a str>,
    pub has_default: bool,
}

impl<'a> BindParam<'a> {
    pub fn new(label: Option<&'a str>, has_default: bool) -> Self {
        Self { label, has_default }
    }
}

/// Where a single parameter slot's value comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    /// Filled by the explicit argument at this index in the call's argument list.
    Arg(usize),
    /// Filled by the parameter's own default value.
    Default,
}

/// Why argument binding failed. Each variant carries enough context for callers
/// to raise their own diagnostic (`InferError::LabelMismatch` / `ArgCountMismatch`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindError {
    /// Argument `arg_index` (label `arg_label`) could not bind to the parameter
    /// it lined up with, whose label was `expected`. Covers a wrong label and a
    /// positional/labeled mismatch, after skipping defaultable parameters.
    LabelMismatch {
        arg_index: usize,
        arg_label: Option<String>,
        expected: Option<String>,
    },
    /// A required (non-defaulted) parameter `param_index` received no argument.
    MissingArgument {
        param_index: usize,
        label: Option<String>,
    },
    /// More arguments were supplied than there are parameters.
    TooManyArguments { expected: usize, got: usize },
}

/// Bind `arg_labels` (call arguments, in source order) onto `params`
/// (declaration order). Returns one [`Binding`] per parameter: `Arg(i)` if the
/// explicit argument at index `i` fills it, or `Default` if its default does.
///
/// Arguments keep their source order; defaulted parameters may be skipped
/// anywhere. The returned vector always has `params.len()` entries.
pub fn bind_arguments(
    params: &[BindParam<'_>],
    arg_labels: &[Option<&str>],
) -> Result<Vec<Binding>, BindError> {
    if arg_labels.len() > params.len() {
        return Err(BindError::TooManyArguments {
            expected: params.len(),
            got: arg_labels.len(),
        });
    }

    let mut plan = vec![Binding::Default; params.len()];
    let mut pi = 0; // next parameter to consider

    for (ai, &arg_label) in arg_labels.iter().enumerate() {
        loop {
            let Some(param) = params.get(pi) else {
                // Ran out of parameters for this argument. If earlier params
                // were skipped because their labels didn't match, surface the
                // mismatch against the first such param; otherwise it's an
                // overflow (already guarded above, so report a label mismatch
                // against "no parameter").
                return Err(BindError::LabelMismatch {
                    arg_index: ai,
                    arg_label: arg_label.map(str::to_string),
                    expected: None,
                });
            };
            if arg_label == param.label {
                plan[pi] = Binding::Arg(ai);
                pi += 1;
                break;
            }
            // Labels differ — skip this parameter only if it is defaultable.
            if !param.has_default {
                return Err(BindError::LabelMismatch {
                    arg_index: ai,
                    arg_label: arg_label.map(str::to_string),
                    expected: param.label.map(str::to_string),
                });
            }
            pi += 1;
        }
    }

    // Any parameter left to its default must actually be defaultable.
    for (i, param) in params.iter().enumerate() {
        if plan[i] == Binding::Default && !param.has_default {
            return Err(BindError::MissingArgument {
                param_index: i,
                label: param.label.map(str::to_string),
            });
        }
    }

    Ok(plan)
}

/// Convenience: does this argument list bind to these parameters at all? Used by
/// overload selection, which only needs a yes/no.
pub fn binds(params: &[BindParam<'_>], arg_labels: &[Option<&str>]) -> bool {
    bind_arguments(params, arg_labels).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(label: Option<&str>, has_default: bool) -> BindParam<'_> {
        BindParam::new(label, has_default)
    }

    // adding(years: =, months: =, days: =) — all defaulted, all labeled.
    fn adding_params() -> Vec<BindParam<'static>> {
        vec![
            p(Some("years"), true),
            p(Some("months"), true),
            p(Some("days"), true),
        ]
    }

    #[test]
    fn skip_leading_default() {
        // adding(months: 1, days: 10) — skip leading `years`
        let plan = bind_arguments(&adding_params(), &[Some("months"), Some("days")]).unwrap();
        assert_eq!(plan, vec![Binding::Default, Binding::Arg(0), Binding::Arg(1)]);
    }

    #[test]
    fn skip_middle_default() {
        // adding(years: 1, days: 10) — skip middle `months`
        let plan = bind_arguments(&adding_params(), &[Some("years"), Some("days")]).unwrap();
        assert_eq!(plan, vec![Binding::Arg(0), Binding::Default, Binding::Arg(1)]);
    }

    #[test]
    fn skip_trailing_default() {
        // adding(years: 1, months: 2) — omit trailing `days`
        let plan = bind_arguments(&adding_params(), &[Some("years"), Some("months")]).unwrap();
        assert_eq!(plan, vec![Binding::Arg(0), Binding::Arg(1), Binding::Default]);
    }

    #[test]
    fn all_explicit() {
        let plan =
            bind_arguments(&adding_params(), &[Some("years"), Some("months"), Some("days")]).unwrap();
        assert_eq!(plan, vec![Binding::Arg(0), Binding::Arg(1), Binding::Arg(2)]);
    }

    #[test]
    fn none_explicit() {
        let plan = bind_arguments(&adding_params(), &[]).unwrap();
        assert_eq!(plan, vec![Binding::Default; 3]);
    }

    #[test]
    fn reordering_is_rejected() {
        // adding(days: 10, years: 1) — out of declaration order
        let err = bind_arguments(&adding_params(), &[Some("days"), Some("years")]).unwrap_err();
        // `days` binds to slot 2; then `years` has no later param to bind to.
        assert!(matches!(err, BindError::LabelMismatch { arg_index: 1, .. }));
    }

    #[test]
    fn wrong_label_is_rejected() {
        let err = bind_arguments(&adding_params(), &[Some("weeks")]).unwrap_err();
        assert_eq!(
            err,
            BindError::LabelMismatch {
                arg_index: 0,
                arg_label: Some("weeks".into()),
                expected: None,
            }
        );
    }

    #[test]
    fn missing_required_after_optional() {
        // params: a(default), b(required). Provide only a -> b unfilled.
        let params = vec![p(Some("a"), true), p(Some("b"), false)];
        let err = bind_arguments(&params, &[Some("a")]).unwrap_err();
        assert_eq!(
            err,
            BindError::MissingArgument {
                param_index: 1,
                label: Some("b".into()),
            }
        );
    }

    #[test]
    fn skip_required_is_rejected() {
        // params: a(required), b(default). Try to bind b only -> can't skip a.
        let params = vec![p(Some("a"), false), p(Some("b"), true)];
        let err = bind_arguments(&params, &[Some("b")]).unwrap_err();
        assert_eq!(
            err,
            BindError::LabelMismatch {
                arg_index: 0,
                arg_label: Some("b".into()),
                expected: Some("a".into()),
            }
        );
    }

    #[test]
    fn positional_params_bind_in_order() {
        // f(x, y) with positional (unlabeled) params and positional args.
        let params = vec![p(None, false), p(None, false)];
        let plan = bind_arguments(&params, &[None, None]).unwrap();
        assert_eq!(plan, vec![Binding::Arg(0), Binding::Arg(1)]);
    }

    #[test]
    fn positional_arg_skips_leading_labeled_default() {
        // params: tag(labeled, default), value(positional). Call f(42).
        let params = vec![p(Some("tag"), true), p(None, false)];
        let plan = bind_arguments(&params, &[None]).unwrap();
        assert_eq!(plan, vec![Binding::Default, Binding::Arg(0)]);
    }

    #[test]
    fn too_many_arguments() {
        let err = bind_arguments(&adding_params(), &[None, None, None, None]).unwrap_err();
        assert_eq!(err, BindError::TooManyArguments { expected: 3, got: 4 });
    }
}
