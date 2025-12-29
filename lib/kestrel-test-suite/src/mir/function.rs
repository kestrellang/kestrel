//! Function expectations for MIR testing.

use crate::mir::block::MirBlock;
use crate::mir::context::MirTestContext;
use crate::mir::types::{format_actual_ty, MirTy};
use crate::{Expectable, TestContext};
use kestrel_execution_graph::{Callee, FunctionDef, Rvalue, StatementData, StatementKind};

/// Expectations for a function definition in the MIR.
pub struct MirFunction {
    name: String,
    expectations: Vec<FunctionExpectation>,
}

enum FunctionExpectation {
    Returns(MirTy),
    HasParam { name: String, ty: MirTy },
    ParamCount(usize),
    HasLocal { name: String, ty: MirTy },
    LocalCount(usize),
    TypeParamCount(usize),
    BlockCount(usize),
    AtLeastBlocks(usize),
    HasWhereClause,
    Block(usize, MirBlock),
    AnyBlock(MirBlock),
    Calls(String),
    DoesNotCall(String),
    CallsEscaping,
    CallsWitness { protocol: String, method: String },
    IsNonCapturing,
    CaptureCount(usize),
}

impl MirFunction {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            expectations: Vec::new(),
        }
    }

    /// Expect the function to return a specific type.
    pub fn returns(mut self, ty: MirTy) -> Self {
        self.expectations.push(FunctionExpectation::Returns(ty));
        self
    }

    /// Expect the function to have a parameter with the given name and type.
    pub fn has_param(mut self, name: &str, ty: MirTy) -> Self {
        self.expectations.push(FunctionExpectation::HasParam {
            name: name.to_string(),
            ty,
        });
        self
    }

    /// Expect exactly N parameters.
    pub fn has_param_count(mut self, n: usize) -> Self {
        self.expectations.push(FunctionExpectation::ParamCount(n));
        self
    }

    /// Expect the function to have a local with the given name and type.
    pub fn has_local(mut self, name: &str, ty: MirTy) -> Self {
        self.expectations.push(FunctionExpectation::HasLocal {
            name: name.to_string(),
            ty,
        });
        self
    }

    /// Expect exactly N locals (excluding parameters).
    pub fn has_local_count(mut self, n: usize) -> Self {
        self.expectations.push(FunctionExpectation::LocalCount(n));
        self
    }

    /// Expect exactly N type parameters.
    pub fn has_type_params(mut self, n: usize) -> Self {
        self.expectations
            .push(FunctionExpectation::TypeParamCount(n));
        self
    }

    /// Expect the function to have a where clause.
    pub fn has_where_clause(mut self) -> Self {
        self.expectations
            .push(FunctionExpectation::HasWhereClause);
        self
    }

    /// Expect exactly N basic blocks.
    pub fn has_block_count(mut self, n: usize) -> Self {
        self.expectations.push(FunctionExpectation::BlockCount(n));
        self
    }

    /// Expect at least N basic blocks.
    pub fn has_at_least_blocks(mut self, n: usize) -> Self {
        self.expectations
            .push(FunctionExpectation::AtLeastBlocks(n));
        self
    }

    /// Expect specific things about block N (0-indexed).
    pub fn block(mut self, index: usize, f: impl FnOnce(MirBlock) -> MirBlock) -> Self {
        let block = f(MirBlock::new());
        self.expectations
            .push(FunctionExpectation::Block(index, block));
        self
    }

    /// Expect something to be true in ANY block.
    pub fn any_block(mut self, f: impl FnOnce(MirBlock) -> MirBlock) -> Self {
        let block = f(MirBlock::new());
        self.expectations.push(FunctionExpectation::AnyBlock(block));
        self
    }

    /// Expect the function to call another function (by fully qualified name).
    pub fn calls(mut self, callee: &str) -> Self {
        self.expectations
            .push(FunctionExpectation::Calls(callee.to_string()));
        self
    }

    /// Expect the function NOT to call another function.
    pub fn does_not_call(mut self, callee: &str) -> Self {
        self.expectations
            .push(FunctionExpectation::DoesNotCall(callee.to_string()));
        self
    }

    /// Expect the function to make at least one escaping (thick) call.
    pub fn calls_escaping(mut self) -> Self {
        self.expectations
            .push(FunctionExpectation::CallsEscaping);
        self
    }

    /// Expect the function to call a witness method.
    pub fn calls_witness(mut self, protocol: &str, method: &str) -> Self {
        self.expectations.push(FunctionExpectation::CallsWitness {
            protocol: protocol.to_string(),
            method: method.to_string(),
        });
        self
    }

    /// Expect this to be a non-capturing closure (no env struct parameter).
    pub fn is_non_capturing(mut self) -> Self {
        self.expectations
            .push(FunctionExpectation::IsNonCapturing);
        self
    }

    /// Expect this closure to have N captures.
    pub fn has_captures(mut self, n: usize) -> Self {
        self.expectations
            .push(FunctionExpectation::CaptureCount(n));
        self
    }

    fn check_expectation(
        &self,
        expectation: &FunctionExpectation,
        def: &FunctionDef,
        mir_ctx: &MirTestContext,
    ) -> Result<(), String> {
        match expectation {
            FunctionExpectation::Returns(expected_ty) => {
                if !expected_ty.matches(def.ret, mir_ctx.mir) {
                    return Err(format!(
                        "Function '{}' returns '{}', expected '{}'",
                        self.name,
                        format_actual_ty(def.ret, mir_ctx.mir),
                        expected_ty.display()
                    ));
                }
            }

            FunctionExpectation::HasParam { name, ty } => {
                let param_id = def.params_by_name.get(name).ok_or_else(|| {
                    let available: Vec<_> = def.params_by_name.keys().cloned().collect();
                    format!(
                        "Function '{}' does not have parameter '{}'. Available: {:?}",
                        self.name, name, available
                    )
                })?;
                let param = &mir_ctx.mir.params[*param_id];
                let local = mir_ctx.mir.local(param.local);
                if !ty.matches(local.ty, mir_ctx.mir) {
                    return Err(format!(
                        "Function '{}' parameter '{}' has type '{}', expected '{}'",
                        self.name,
                        name,
                        format_actual_ty(local.ty, mir_ctx.mir),
                        ty.display()
                    ));
                }
            }

            FunctionExpectation::ParamCount(expected) => {
                let actual = def.params.len();
                if actual != *expected {
                    return Err(format!(
                        "Function '{}' has {} parameter(s), expected {}",
                        self.name, actual, expected
                    ));
                }
            }

            FunctionExpectation::HasLocal { name, ty } => {
                let local_id = def.local_by_name(name).ok_or_else(|| {
                    let available: Vec<_> = def.locals_by_name.keys().cloned().collect();
                    format!(
                        "Function '{}' does not have local '{}'. Available: {:?}",
                        self.name, name, available
                    )
                })?;
                let local = mir_ctx.mir.local(local_id);
                if !ty.matches(local.ty, mir_ctx.mir) {
                    return Err(format!(
                        "Function '{}' local '{}' has type '{}', expected '{}'",
                        self.name,
                        name,
                        format_actual_ty(local.ty, mir_ctx.mir),
                        ty.display()
                    ));
                }
            }

            FunctionExpectation::LocalCount(expected) => {
                // Count locals that are NOT parameters
                let param_locals: std::collections::HashSet<_> = def
                    .params
                    .iter()
                    .map(|p| mir_ctx.mir.params[*p].local)
                    .collect();
                let actual = def
                    .locals
                    .iter()
                    .filter(|l| !param_locals.contains(*l))
                    .count();
                if actual != *expected {
                    return Err(format!(
                        "Function '{}' has {} local(s) (excluding params), expected {}",
                        self.name, actual, expected
                    ));
                }
            }

            FunctionExpectation::TypeParamCount(expected) => {
                let actual = def.type_params.len();
                if actual != *expected {
                    return Err(format!(
                        "Function '{}' has {} type parameter(s), expected {}",
                        self.name, actual, expected
                    ));
                }
            }

            FunctionExpectation::BlockCount(expected) => {
                let actual = def.blocks.len();
                if actual != *expected {
                    return Err(format!(
                        "Function '{}' has {} block(s), expected {}",
                        self.name, actual, expected
                    ));
                }
            }

            FunctionExpectation::AtLeastBlocks(expected) => {
                let actual = def.blocks.len();
                if actual < *expected {
                    return Err(format!(
                        "Function '{}' has {} block(s), expected at least {}",
                        self.name, actual, expected
                    ));
                }
            }

            FunctionExpectation::HasWhereClause => {
                if def.where_clause.is_none() {
                    return Err(format!(
                        "Function '{}' does not have a where clause",
                        self.name
                    ));
                }
            }

            FunctionExpectation::Block(index, block_exp) => {
                if *index >= def.blocks.len() {
                    return Err(format!(
                        "Function '{}' does not have block {} (only {} blocks)",
                        self.name,
                        index,
                        def.blocks.len()
                    ));
                }
                let block = mir_ctx.mir.block(def.blocks[*index]);
                block_exp.check(*index, block, &def.blocks, mir_ctx.mir)?;
            }

            FunctionExpectation::AnyBlock(block_exp) => {
                let mut any_passed = false;
                let mut errors = Vec::new();

                for (idx, &block_id) in def.blocks.iter().enumerate() {
                    let block = mir_ctx.mir.block(block_id);
                    match block_exp.check(idx, block, &def.blocks, mir_ctx.mir) {
                        Ok(()) => {
                            any_passed = true;
                            break;
                        }
                        Err(e) => errors.push(format!("bb{}: {}", idx, e)),
                    }
                }

                if !any_passed {
                    return Err(format!(
                        "Function '{}': no block matched the expectation. Errors:\n  {}",
                        self.name,
                        errors.join("\n  ")
                    ));
                }
            }

            FunctionExpectation::Calls(callee) => {
                if !self.function_calls(def, callee, mir_ctx) {
                    return Err(format!(
                        "Function '{}' does not call '{}'",
                        self.name, callee
                    ));
                }
            }

            FunctionExpectation::DoesNotCall(callee) => {
                if self.function_calls(def, callee, mir_ctx) {
                    return Err(format!(
                        "Function '{}' should NOT call '{}', but it does",
                        self.name, callee
                    ));
                }
            }

            FunctionExpectation::CallsEscaping => {
                if !self.function_has_escaping_call(def, mir_ctx) {
                    return Err(format!(
                        "Function '{}' does not make any escaping calls",
                        self.name
                    ));
                }
            }

            FunctionExpectation::CallsWitness { protocol, method } => {
                if !self.function_calls_witness(def, protocol, method, mir_ctx) {
                    return Err(format!(
                        "Function '{}' does not call witness method {}.{}",
                        self.name, protocol, method
                    ));
                }
            }

            FunctionExpectation::IsNonCapturing => {
                // A non-capturing closure should not have an env parameter
                // Check if this looks like a closure and has no env-related parameters
                let has_env = def.params.iter().any(|p| {
                    let param = &mir_ctx.mir.params[*p];
                    param.name.contains("env")
                });
                if has_env {
                    return Err(format!(
                        "Function '{}' appears to capture (has env parameter)",
                        self.name
                    ));
                }
            }

            FunctionExpectation::CaptureCount(expected) => {
                // Look for env-prefixed parameters to determine capture count
                // Closures with captures have an env parameter
                let env_params: Vec<_> = def
                    .params
                    .iter()
                    .filter(|p| {
                        let param = &mir_ctx.mir.params[**p];
                        param.name.contains("env") || param.name.starts_with("$")
                    })
                    .collect();

                if env_params.is_empty() && *expected > 0 {
                    return Err(format!(
                        "Function '{}' expected {} capture(s), but has no env parameter",
                        self.name, expected
                    ));
                }

                // For a more precise check, we'd need to look at the env struct definition
                // This is a simplified version that just checks if captures are expected
                if *expected == 0 && !env_params.is_empty() {
                    return Err(format!(
                        "Function '{}' expected 0 captures, but has env parameter",
                        self.name
                    ));
                }
            }
        }

        Ok(())
    }

    fn function_calls(&self, def: &FunctionDef, callee: &str, mir_ctx: &MirTestContext) -> bool {
        for &block_id in &def.blocks {
            let block = mir_ctx.mir.block(block_id);
            for &stmt_id in &block.statements {
                let stmt = mir_ctx.mir.statement(stmt_id);
                if self.statement_calls(stmt, callee, mir_ctx) {
                    return true;
                }
            }
        }
        false
    }

    fn statement_calls(
        &self,
        stmt: &StatementData,
        callee: &str,
        mir_ctx: &MirTestContext,
    ) -> bool {
        match &stmt.kind {
            StatementKind::Assign { rvalue, .. } => {
                if let Rvalue::Call {
                    callee: actual_callee,
                    ..
                } = rvalue
                {
                    self.callee_is(actual_callee, callee, mir_ctx)
                } else {
                    false
                }
            }
            StatementKind::Call {
                callee: actual_callee,
                ..
            } => self.callee_is(actual_callee, callee, mir_ctx),
        }
    }

    fn callee_is(&self, callee: &Callee, expected: &str, mir_ctx: &MirTestContext) -> bool {
        match callee {
            Callee::Direct { name, .. } => mir_ctx.mir.name(*name).to_string() == expected,
            _ => false,
        }
    }

    fn function_has_escaping_call(&self, def: &FunctionDef, mir_ctx: &MirTestContext) -> bool {
        for &block_id in &def.blocks {
            let block = mir_ctx.mir.block(block_id);
            for &stmt_id in &block.statements {
                let stmt = mir_ctx.mir.statement(stmt_id);
                match &stmt.kind {
                    StatementKind::Assign { rvalue, .. } => {
                        if let Rvalue::Call { callee, .. } = rvalue {
                            if matches!(callee, Callee::Thick(_)) {
                                return true;
                            }
                        }
                    }
                    StatementKind::Call { callee, .. } => {
                        if matches!(callee, Callee::Thick(_)) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn function_calls_witness(
        &self,
        def: &FunctionDef,
        protocol: &str,
        method: &str,
        mir_ctx: &MirTestContext,
    ) -> bool {
        for &block_id in &def.blocks {
            let block = mir_ctx.mir.block(block_id);
            for &stmt_id in &block.statements {
                let stmt = mir_ctx.mir.statement(stmt_id);
                match &stmt.kind {
                    StatementKind::Assign { rvalue, .. } => {
                        if let Rvalue::Call { callee, .. } = rvalue {
                            if self.is_witness_call(callee, protocol, method, mir_ctx) {
                                return true;
                            }
                        }
                    }
                    StatementKind::Call { callee, .. } => {
                        if self.is_witness_call(callee, protocol, method, mir_ctx) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn is_witness_call(
        &self,
        callee: &Callee,
        protocol: &str,
        method: &str,
        mir_ctx: &MirTestContext,
    ) -> bool {
        if let Callee::Witness {
            protocol: actual_protocol,
            method: actual_method,
            ..
        } = callee
        {
            let actual_protocol_name = mir_ctx.mir.name(*actual_protocol).to_string();
            actual_protocol_name == protocol && actual_method == method
        } else {
            false
        }
    }
}

impl Expectable for MirFunction {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        let mir_result = ctx.mir();
        let mir_ctx = MirTestContext::new(mir_result);

        let Some((_, def)) = mir_ctx.find_function(&self.name) else {
            let available: Vec<_> = mir_ctx
                .mir
                .functions
                .iter()
                .map(|(_, d)| mir_ctx.mir.name(d.name).to_string())
                .collect();
            let diagnostics = mir_ctx.format_diagnostics(&ctx.diagnostics);
            let mir = mir_ctx.format_mir();
            return Err(format!(
                "Function '{}' not found in MIR. Available functions: {:?}\n\n{}\n--- MIR ---\n{}",
                self.name, available, diagnostics, mir
            ));
        };

        for expectation in &self.expectations {
            if let Err(e) = self.check_expectation(expectation, def, &mir_ctx) {
                let diagnostics = mir_ctx.format_diagnostics(&ctx.diagnostics);
                let mir = mir_ctx.format_mir();
                return Err(format!("{}\n\n{}\n--- MIR ---\n{}", e, diagnostics, mir));
            }
        }

        Ok(())
    }
}
