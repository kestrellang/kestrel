//! Count expectations for MIR items.

use crate::mir::context::MirTestContext;
use crate::{Expectable, TestContext, skip_codegen};

/// Expects exactly N structs in the MIR.
pub struct MirStructCount(pub usize);

impl Expectable for MirStructCount {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if skip_codegen() {
            return Ok(());
        }
        let mir_result = ctx.mir();
        let mir_ctx = MirTestContext::new(mir_result);
        let actual = mir_ctx.mir.structs.iter().count();

        if actual != self.0 {
            let diagnostics = mir_ctx.format_diagnostics(&ctx.diagnostics);
            let mir = mir_ctx.format_mir();
            return Err(format!(
                "Expected {} struct(s), but found {}\n\n{}\n--- MIR ---\n{}",
                self.0, actual, diagnostics, mir
            ));
        }

        Ok(())
    }
}

/// Expects exactly N enums in the MIR.
pub struct MirEnumCount(pub usize);

impl Expectable for MirEnumCount {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if skip_codegen() {
            return Ok(());
        }
        let mir_result = ctx.mir();
        let mir_ctx = MirTestContext::new(mir_result);
        let actual = mir_ctx.mir.enums.iter().count();

        if actual != self.0 {
            let diagnostics = mir_ctx.format_diagnostics(&ctx.diagnostics);
            let mir = mir_ctx.format_mir();
            return Err(format!(
                "Expected {} enum(s), but found {}\n\n{}\n--- MIR ---\n{}",
                self.0, actual, diagnostics, mir
            ));
        }

        Ok(())
    }
}

/// Expects exactly N functions in the MIR.
pub struct MirFunctionCount(pub usize);

impl Expectable for MirFunctionCount {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if skip_codegen() {
            return Ok(());
        }
        let mir_result = ctx.mir();
        let mir_ctx = MirTestContext::new(mir_result);
        let actual = mir_ctx.mir.functions.iter().count();

        if actual != self.0 {
            let diagnostics = mir_ctx.format_diagnostics(&ctx.diagnostics);
            let mir = mir_ctx.format_mir();
            return Err(format!(
                "Expected {} function(s), but found {}\n\n{}\n--- MIR ---\n{}",
                self.0, actual, diagnostics, mir
            ));
        }

        Ok(())
    }
}

/// Expects exactly N witnesses in the MIR.
pub struct MirWitnessCount(pub usize);

impl Expectable for MirWitnessCount {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if skip_codegen() {
            return Ok(());
        }
        let mir_result = ctx.mir();
        let mir_ctx = MirTestContext::new(mir_result);
        let actual = mir_ctx.mir.witnesses.iter().count();

        if actual != self.0 {
            let diagnostics = mir_ctx.format_diagnostics(&ctx.diagnostics);
            let mir = mir_ctx.format_mir();
            return Err(format!(
                "Expected {} witness(es), but found {}\n\n{}\n--- MIR ---\n{}",
                self.0, actual, diagnostics, mir
            ));
        }

        Ok(())
    }
}
