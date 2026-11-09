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
//!
//! **Trailing closures.** An unlabeled trailing closure (`f(x) { … }`) binds
//! like any unlabeled argument first. When that fails, it may instead fill a
//! LABELED closure-typed parameter (`xs.filter { … }` for
//! `filter(where:)`): every way of placing the call's trailing closures on
//! unlabeled or closure-typed parameters is tried, and the call binds only
//! if exactly one works. More than one is [`BindError::AmbiguousTrailingClosure`].

use kestrel_hir::body::HirCallArg;

use crate::AstParam;

/// A parameter as seen by argument binding: its (optional) label and whether it
/// carries a default value. Adapted from `AstParam` (via `default_entity`) or
/// type-infer's `ParamInfo` (via `has_default`).
#[derive(Clone, Copy, Debug)]
pub struct BindParam<'a> {
    pub label: Option<&'a str>,
    pub has_default: bool,
    /// Declared with a function type, so an unlabeled trailing closure may
    /// fill it even though it is labeled.
    pub takes_closure: bool,
}

impl<'a> BindParam<'a> {
    pub fn new(label: Option<&'a str>, has_default: bool) -> Self {
        Self {
            label,
            has_default,
            takes_closure: false,
        }
    }

    /// The binding view of a declared parameter.
    pub fn of(p: &'a AstParam) -> Self {
        Self {
            label: p.label.as_deref(),
            has_default: p.default_entity.is_some(),
            takes_closure: p.takes_closure(),
        }
    }

    /// Can an unlabeled trailing closure fill this parameter?
    fn fits_trailing_closure(&self) -> bool {
        self.label.is_none() || self.takes_closure
    }
}

/// A call argument as seen by argument binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BindArg<'a> {
    pub label: Option<&'a str>,
    /// An unlabeled trailing closure (see the module docs).
    pub trailing_closure: bool,
}

impl<'a> BindArg<'a> {
    /// An ordinary (non-trailing) argument.
    pub fn plain(label: Option<&'a str>) -> Self {
        Self {
            label,
            trailing_closure: false,
        }
    }
}

impl<'a> From<&'a HirCallArg> for BindArg<'a> {
    fn from(a: &'a HirCallArg) -> Self {
        Self {
            label: a.label.as_deref(),
            trailing_closure: a.trailing_closure,
        }
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
    /// The trailing closure at `arg_index` could fill more than one
    /// parameter (their labels in `candidates`); the caller must spell one.
    AmbiguousTrailingClosure {
        arg_index: usize,
        candidates: Vec<Option<String>>,
    },
}

/// Bind `args` (call arguments, in source order) onto `params`
/// (declaration order). Returns one [`Binding`] per parameter: `Arg(i)` if the
/// explicit argument at index `i` fills it, or `Default` if its default does.
///
/// Arguments keep their source order; defaulted parameters may be skipped
/// anywhere. The returned vector always has `params.len()` entries.
pub fn bind_arguments(
    params: &[BindParam<'_>],
    args: &[BindArg<'_>],
) -> Result<Vec<Binding>, BindError> {
    let err = match bind_by_label(params, args) {
        Ok(plan) => return Ok(plan),
        Err(e) => e,
    };
    if !args.iter().any(|a| a.trailing_closure) {
        return Err(err);
    }
    // Trailing-closure fallback: every placement that binds.
    let mut plans = Vec::new();
    place_trailing(
        params,
        args,
        0,
        0,
        vec![Binding::Default; params.len()],
        &mut plans,
    );
    match plans.len() {
        0 => Err(err),
        1 => Ok(plans.pop().unwrap()),
        _ => Err(ambiguity(params, args, &plans)),
    }
}

/// The plain label walk: each argument binds to the next parameter with its
/// label, skipping defaultable ones.
fn bind_by_label(
    params: &[BindParam<'_>],
    args: &[BindArg<'_>],
) -> Result<Vec<Binding>, BindError> {
    if args.len() > params.len() {
        return Err(BindError::TooManyArguments {
            expected: params.len(),
            got: args.len(),
        });
    }

    let mut plan = vec![Binding::Default; params.len()];
    let mut pi = 0; // next parameter to consider

    for (ai, arg) in args.iter().enumerate() {
        let arg_label = arg.label;
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

/// Collect into `out` every complete binding of `args[ai..]` onto
/// `params[pi..]` where a trailing closure may take any parameter it fits
/// and every other argument binds by label. Order is kept and only
/// defaultable parameters are skipped. Stops at two: that is already
/// ambiguous.
fn place_trailing(
    params: &[BindParam<'_>],
    args: &[BindArg<'_>],
    ai: usize,
    pi: usize,
    plan: Vec<Binding>,
    out: &mut Vec<Vec<Binding>>,
) {
    if out.len() > 1 {
        return;
    }
    let Some(arg) = args.get(ai) else {
        if params[pi..].iter().all(|p| p.has_default) {
            out.push(plan);
        }
        return;
    };
    for (q, param) in params.iter().enumerate().skip(pi) {
        let fits = if arg.trailing_closure {
            param.fits_trailing_closure()
        } else {
            arg.label == param.label
        };
        if fits {
            let mut next = plan.clone();
            next[q] = Binding::Arg(ai);
            place_trailing(params, args, ai + 1, q + 1, next, out);
        }
        if !param.has_default {
            return;
        }
    }
}

/// The ambiguity error for two plans: the first trailing closure they place
/// differently, with the labels of every parameter it took.
fn ambiguity(params: &[BindParam<'_>], args: &[BindArg<'_>], plans: &[Vec<Binding>]) -> BindError {
    let slot_of = |plan: &[Binding], ai: usize| plan.iter().position(|&b| b == Binding::Arg(ai));
    let arg_index = (0..args.len())
        .find(|&ai| {
            plans
                .iter()
                .any(|p| slot_of(p, ai) != slot_of(&plans[0], ai))
        })
        .unwrap_or(0);
    let candidates = plans
        .iter()
        .filter_map(|p| slot_of(p, arg_index))
        .map(|q| params[q].label.map(str::to_string))
        .collect();
    BindError::AmbiguousTrailingClosure {
        arg_index,
        candidates,
    }
}

/// Is `label` the internal name of a one-name parameter? One-name params take
/// no label, so a call like `joined(separator: ",")` against
/// `func joined(separator: String)` gets a targeted diagnostic.
/// `params` yields each parameter's `(label, internal name)`.
pub fn names_unlabeled_param<'a>(
    params: impl IntoIterator<Item = (Option<&'a str>, &'a str)>,
    label: Option<&str>,
) -> bool {
    let Some(label) = label else { return false };
    params
        .into_iter()
        .any(|(l, name)| l.is_none() && name == label)
}

/// Convenience: does this argument list bind to these parameters at all? Used by
/// overload selection, which only needs a yes/no.
pub fn binds(params: &[BindParam<'_>], args: &[BindArg<'_>]) -> bool {
    bind_arguments(params, args).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bind_arguments(
        params: &[BindParam<'_>],
        labels: &[Option<&str>],
    ) -> Result<Vec<Binding>, BindError> {
        let args: Vec<BindArg> = labels.iter().map(|&l| BindArg::plain(l)).collect();
        super::bind_arguments(params, &args)
    }

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
        assert_eq!(
            plan,
            vec![Binding::Default, Binding::Arg(0), Binding::Arg(1)]
        );
    }

    #[test]
    fn skip_middle_default() {
        // adding(years: 1, days: 10) — skip middle `months`
        let plan = bind_arguments(&adding_params(), &[Some("years"), Some("days")]).unwrap();
        assert_eq!(
            plan,
            vec![Binding::Arg(0), Binding::Default, Binding::Arg(1)]
        );
    }

    #[test]
    fn skip_trailing_default() {
        // adding(years: 1, months: 2) — omit trailing `days`
        let plan = bind_arguments(&adding_params(), &[Some("years"), Some("months")]).unwrap();
        assert_eq!(
            plan,
            vec![Binding::Arg(0), Binding::Arg(1), Binding::Default]
        );
    }

    #[test]
    fn all_explicit() {
        let plan = bind_arguments(
            &adding_params(),
            &[Some("years"), Some("months"), Some("days")],
        )
        .unwrap();
        assert_eq!(
            plan,
            vec![Binding::Arg(0), Binding::Arg(1), Binding::Arg(2)]
        );
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
        assert_eq!(
            err,
            BindError::TooManyArguments {
                expected: 3,
                got: 4
            }
        );
    }

    fn closure_param(label: Option<&str>, has_default: bool) -> BindParam<'_> {
        BindParam {
            label,
            has_default,
            takes_closure: true,
        }
    }

    const TRAILING: BindArg<'static> = BindArg {
        label: None,
        trailing_closure: true,
    };

    #[test]
    fn trailing_closure_fills_labeled_closure_param() {
        // filter(where:) { … }
        let params = [closure_param(Some("where"), false)];
        let plan = super::bind_arguments(&params, &[TRAILING]).unwrap();
        assert_eq!(plan, vec![Binding::Arg(0)]);
    }

    #[test]
    fn trailing_closure_skips_to_the_only_closure_param() {
        // fold(from: 0) { … }
        let params = [p(Some("from"), false), closure_param(Some("by"), false)];
        let args = [BindArg::plain(Some("from")), TRAILING];
        let plan = super::bind_arguments(&params, &args).unwrap();
        assert_eq!(plan, vec![Binding::Arg(0), Binding::Arg(1)]);
    }

    #[test]
    fn trailing_closure_never_fills_a_labeled_value_param() {
        let params = [p(Some("times"), false)];
        assert!(super::bind_arguments(&params, &[TRAILING]).is_err());
    }

    #[test]
    fn trailing_closure_ambiguous() {
        // pick(onA: = {}, onB: = {}) { … }
        let params = [
            closure_param(Some("onA"), true),
            closure_param(Some("onB"), true),
        ];
        let err = super::bind_arguments(&params, &[TRAILING]).unwrap_err();
        assert_eq!(
            err,
            BindError::AmbiguousTrailingClosure {
                arg_index: 0,
                candidates: vec![Some("onA".into()), Some("onB".into())],
            }
        );
    }

    #[test]
    fn unlabeled_param_keeps_first_claim() {
        // An unlabeled param binds by label before any fallback runs.
        let params = [
            closure_param(None, false),
            closure_param(Some("then"), true),
        ];
        let plan = super::bind_arguments(&params, &[TRAILING]).unwrap();
        assert_eq!(plan, vec![Binding::Arg(0), Binding::Default]);
    }
}
