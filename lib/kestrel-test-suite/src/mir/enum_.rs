//! Enum expectations for MIR testing.

use crate::mir::context::MirTestContext;
use crate::{Expectable, TestContext};
use kestrel_execution_graph::EnumDef;

/// Expectations for an enum definition in the MIR.
pub struct MirEnum {
    name: String,
    expectations: Vec<EnumExpectation>,
}

enum EnumExpectation {
    HasCase(String),
    CaseCount(usize),
    TypeParamCount(usize),
    CaseHasStruct {
        case_name: String,
        struct_name: String,
    },
}

impl MirEnum {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            expectations: Vec::new(),
        }
    }

    /// Expect the enum to have a case with the given name.
    pub fn has_case(mut self, name: &str) -> Self {
        self.expectations
            .push(EnumExpectation::HasCase(name.to_string()));
        self
    }

    /// Expect the enum to have exactly N cases.
    pub fn has_case_count(mut self, n: usize) -> Self {
        self.expectations.push(EnumExpectation::CaseCount(n));
        self
    }

    /// Expect the enum to have exactly N type parameters.
    pub fn has_type_params(mut self, n: usize) -> Self {
        self.expectations.push(EnumExpectation::TypeParamCount(n));
        self
    }

    /// Expect a case to have a specific payload struct.
    pub fn case_has_struct(mut self, case_name: &str, struct_name: &str) -> Self {
        self.expectations.push(EnumExpectation::CaseHasStruct {
            case_name: case_name.to_string(),
            struct_name: struct_name.to_string(),
        });
        self
    }

    fn check_expectation(
        &self,
        expectation: &EnumExpectation,
        def: &EnumDef,
        mir_ctx: &MirTestContext,
    ) -> Result<(), String> {
        match expectation {
            EnumExpectation::HasCase(name) => {
                if def.case_by_name(name).is_none() {
                    let available: Vec<_> = def
                        .cases
                        .iter()
                        .map(|c| mir_ctx.mir.enum_cases[*c].name.clone())
                        .collect();
                    return Err(format!(
                        "Enum '{}' does not have case '{}'. Available cases: {:?}",
                        self.name, name, available
                    ));
                }
                Ok(())
            },

            EnumExpectation::CaseCount(expected) => {
                let actual = def.cases.len();
                if actual != *expected {
                    return Err(format!(
                        "Enum '{}' has {} case(s), expected {}",
                        self.name, actual, expected
                    ));
                }
                Ok(())
            },

            EnumExpectation::TypeParamCount(expected) => {
                let actual = def.type_params.len();
                if actual != *expected {
                    return Err(format!(
                        "Enum '{}' has {} type parameter(s), expected {}",
                        self.name, actual, expected
                    ));
                }
                Ok(())
            },

            EnumExpectation::CaseHasStruct {
                case_name,
                struct_name,
            } => {
                let case_id = def.case_by_name(case_name).ok_or_else(|| {
                    format!("Enum '{}' does not have case '{}'", self.name, case_name)
                })?;

                let case_def = &mir_ctx.mir.enum_cases[case_id];
                let actual_struct_name = mir_ctx.mir.name(case_def.struct_name);
                if actual_struct_name.to_string() != *struct_name {
                    return Err(format!(
                        "Enum '{}' case '{}' has struct '{}', expected '{}'",
                        self.name, case_name, actual_struct_name, struct_name
                    ));
                }

                Ok(())
            },
        }
    }
}

impl Expectable for MirEnum {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if crate::skip_codegen() {
            return Ok(());
        }
        let mir_result = ctx.mir();
        let mir_ctx = MirTestContext::new(mir_result);

        let Some((_, def)) = mir_ctx.find_enum(&self.name) else {
            let available: Vec<_> = mir_ctx
                .mir
                .enums
                .iter()
                .map(|(_, d)| mir_ctx.mir.name(d.name).to_string())
                .collect();
            let diagnostics = mir_ctx.format_diagnostics(&ctx.diagnostics);
            let mir = mir_ctx.format_mir();
            return Err(format!(
                "Enum '{}' not found in MIR. Available enums: {:?}\n\n{}\n--- MIR ---\n{}",
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
