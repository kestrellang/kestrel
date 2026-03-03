//! Struct expectations for MIR testing.

use crate::mir::context::MirTestContext;
use crate::mir::types::{MirTy, format_actual_ty};
use crate::{Expectable, TestContext};
use kestrel_execution_graph::StructDef;

/// Expectations for a struct definition in the MIR.
pub struct MirStruct {
    name: String,
    expectations: Vec<StructExpectation>,
}

enum StructExpectation {
    HasField { name: String, ty: MirTy },
    FieldCount(usize),
    TypeParamCount(usize),
    NoField(String),
}

impl MirStruct {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            expectations: Vec::new(),
        }
    }

    /// Expect the struct to have a field with the given name and type.
    pub fn has_field(mut self, name: &str, ty: MirTy) -> Self {
        self.expectations.push(StructExpectation::HasField {
            name: name.to_string(),
            ty,
        });
        self
    }

    /// Expect the struct to have exactly N fields.
    pub fn has_field_count(mut self, n: usize) -> Self {
        self.expectations.push(StructExpectation::FieldCount(n));
        self
    }

    /// Expect the struct to have exactly N type parameters.
    pub fn has_type_params(mut self, n: usize) -> Self {
        self.expectations.push(StructExpectation::TypeParamCount(n));
        self
    }

    /// Expect the struct NOT to have a field with the given name.
    pub fn no_field(mut self, name: &str) -> Self {
        self.expectations
            .push(StructExpectation::NoField(name.to_string()));
        self
    }

    fn check_expectation(
        &self,
        expectation: &StructExpectation,
        def: &StructDef,
        mir_ctx: &MirTestContext,
    ) -> Result<(), String> {
        match expectation {
            StructExpectation::HasField { name, ty } => {
                let field_id = def.field_by_name(name).ok_or_else(|| {
                    let available: Vec<_> = def
                        .fields
                        .iter()
                        .map(|f| mir_ctx.mir.fields[*f].name.clone())
                        .collect();
                    format!(
                        "Struct '{}' does not have field '{}'. Available fields: {:?}",
                        self.name, name, available
                    )
                })?;

                let field_def = &mir_ctx.mir.fields[field_id];
                if !ty.matches(field_def.ty, mir_ctx.mir) {
                    return Err(format!(
                        "Struct '{}' field '{}' has type '{}', expected '{}'",
                        self.name,
                        name,
                        format_actual_ty(field_def.ty, mir_ctx.mir),
                        ty.display()
                    ));
                }

                Ok(())
            },

            StructExpectation::FieldCount(expected) => {
                let actual = def.fields.len();
                if actual != *expected {
                    return Err(format!(
                        "Struct '{}' has {} field(s), expected {}",
                        self.name, actual, expected
                    ));
                }
                Ok(())
            },

            StructExpectation::TypeParamCount(expected) => {
                let actual = def.type_params.len();
                if actual != *expected {
                    return Err(format!(
                        "Struct '{}' has {} type parameter(s), expected {}",
                        self.name, actual, expected
                    ));
                }
                Ok(())
            },

            StructExpectation::NoField(name) => {
                if def.field_by_name(name).is_some() {
                    return Err(format!(
                        "Struct '{}' should NOT have field '{}', but it does",
                        self.name, name
                    ));
                }
                Ok(())
            },
        }
    }
}

impl Expectable for MirStruct {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if crate::skip_codegen() {
            return Ok(());
        }
        let mir_result = ctx.mir();
        let mir_ctx = MirTestContext::new(mir_result);

        let Some((_, def)) = mir_ctx.find_struct(&self.name) else {
            let available: Vec<_> = mir_ctx
                .mir
                .structs
                .iter()
                .map(|(_, d)| mir_ctx.mir.name(d.name).to_string())
                .collect();
            let diagnostics = mir_ctx.format_diagnostics(&ctx.diagnostics);
            let mir = mir_ctx.format_mir();
            return Err(format!(
                "Struct '{}' not found in MIR. Available structs: {:?}\n\n{}\n--- MIR ---\n{}",
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
