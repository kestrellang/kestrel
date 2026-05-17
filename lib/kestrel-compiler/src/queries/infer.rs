use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_type_infer::InferBody;
use kestrel_type_infer::error::InferError;
use kestrel_type_infer::result::TypedBody;

use crate::diagnostic::{ResolvedInferError, ThrowDiagnostic};

/// Run type inference and accumulate errors as diagnostics.
///
/// Wraps `InferBody` — calls it, then converts any `InferError`s in the
/// result into codespan diagnostics via `ToDiagnostic` and accumulates
/// them. This way inference errors show up in
/// `world.accumulated::<Diagnostic>()` alongside lex/parse diagnostics.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct InferWithDiagnostics {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for InferWithDiagnostics {
    type Output = Option<TypedBody>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
        let typed = ctx.query(InferBody {
            entity: self.entity,
            root: self.root,
        })?;

        // Convert inference errors to rich diagnostics and accumulate.
        // Skip FromHir errors — they duplicate diagnostics already emitted
        // during HIR lowering (e.g., unresolved names, empty type arg lists).
        for (i, err) in typed.errors.iter().enumerate() {
            if matches!(err, InferError::FromHir { .. }) {
                continue;
            }
            let detail = typed.error_details.get(i).map(|s| s.as_str()).unwrap_or("");
            ctx.throw(ResolvedInferError { error: err, detail });
        }

        Some(typed)
    }
}
