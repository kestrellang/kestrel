//! MIR compilation success/failure expectations.

use crate::mir::context::MirTestContext;
use crate::{Expectable, TestContext, skip_codegen};

/// Expects MIR lowering to succeed with no errors.
pub struct MirCompiles;

impl Expectable for MirCompiles {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if skip_codegen() {
            return Ok(());
        }
        let mir_result = ctx.mir();
        let mir_ctx = MirTestContext::new(mir_result);

        if mir_ctx.has_lowering_errors() {
            let diagnostics = mir_ctx.format_diagnostics(&ctx.diagnostics);
            let mir = mir_ctx.format_mir();
            return Err(format!(
                "Expected MIR lowering to succeed, but it had errors\n\n{}\n--- MIR ---\n{}",
                diagnostics, mir
            ));
        }

        Ok(())
    }
}

/// Expects MIR lowering to fail with at least one error.
pub struct MirFails;

impl Expectable for MirFails {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if skip_codegen() {
            return Ok(());
        }
        let mir_result = ctx.mir();
        let mir_ctx = MirTestContext::new(mir_result);

        if !mir_ctx.has_lowering_errors() {
            let mir = mir_ctx.format_mir();
            return Err(format!(
                "Expected MIR lowering to fail, but it succeeded\n\n--- MIR ---\n{}",
                mir
            ));
        }

        Ok(())
    }
}
