//! Witness expectations for MIR testing.

use crate::mir::context::MirTestContext;
use crate::mir::types::MirTy;
use crate::{Expectable, TestContext};
use kestrel_execution_graph::WitnessDef;

/// Expectations for a witness table in the MIR.
pub struct MirWitness {
    implementing_type: String,
    protocol: String,
    expectations: Vec<WitnessExpectation>,
}

enum WitnessExpectation {
    HasMethod { name: String, impl_func: String },
    HasMethodByName { name: String },
    MethodCount(usize),
    HasAssociatedType { name: String, ty: MirTy },
    HasAssociatedTypeByName { name: String },
}

impl MirWitness {
    pub fn new(implementing_type: &str, protocol: &str) -> Self {
        Self {
            implementing_type: implementing_type.to_string(),
            protocol: protocol.to_string(),
            expectations: Vec::new(),
        }
    }

    /// Expect the witness to have a method (by protocol method name only).
    pub fn has_method(mut self, protocol_method: &str) -> Self {
        self.expectations.push(WitnessExpectation::HasMethodByName {
            name: protocol_method.to_string(),
        });
        self
    }

    /// Expect the witness to map a protocol method to a specific implementation function.
    pub fn has_method_mapping(mut self, protocol_method: &str, impl_func: &str) -> Self {
        self.expectations.push(WitnessExpectation::HasMethod {
            name: protocol_method.to_string(),
            impl_func: impl_func.to_string(),
        });
        self
    }

    /// Expect exactly N method mappings.
    pub fn has_method_count(mut self, n: usize) -> Self {
        self.expectations.push(WitnessExpectation::MethodCount(n));
        self
    }

    /// Expect an associated type binding (just by name).
    pub fn has_associated_type(mut self, name: &str) -> Self {
        self.expectations
            .push(WitnessExpectation::HasAssociatedTypeByName {
                name: name.to_string(),
            });
        self
    }

    /// Expect an associated type binding with a specific type.
    pub fn has_associated_type_with_ty(mut self, name: &str, ty: MirTy) -> Self {
        self.expectations
            .push(WitnessExpectation::HasAssociatedType {
                name: name.to_string(),
                ty,
            });
        self
    }

    fn check_expectation(
        &self,
        expectation: &WitnessExpectation,
        def: &WitnessDef,
        mir_ctx: &MirTestContext,
    ) -> Result<(), String> {
        match expectation {
            WitnessExpectation::HasMethod { name, impl_func } => {
                let (impl_name, _type_args) = def.method_bindings.get(name).ok_or_else(|| {
                    let available: Vec<_> = def.method_bindings.keys().cloned().collect();
                    format!(
                        "Witness for {} : {} does not have method '{}'. Available: {:?}",
                        self.implementing_type, self.protocol, name, available
                    )
                })?;
                let actual_impl = mir_ctx.mir.name(*impl_name).to_string();
                if actual_impl != *impl_func {
                    return Err(format!(
                        "Witness method '{}' maps to '{}', expected '{}'",
                        name, actual_impl, impl_func
                    ));
                }
                Ok(())
            },

            WitnessExpectation::HasMethodByName { name } => {
                if !def.method_bindings.contains_key(name) {
                    let available: Vec<_> = def.method_bindings.keys().cloned().collect();
                    return Err(format!(
                        "Witness for {} : {} does not have method '{}'. Available: {:?}",
                        self.implementing_type, self.protocol, name, available
                    ));
                }
                Ok(())
            },

            WitnessExpectation::MethodCount(expected) => {
                let actual = def.method_bindings.len();
                if actual != *expected {
                    return Err(format!(
                        "Witness for {} : {} has {} method(s), expected {}",
                        self.implementing_type, self.protocol, actual, expected
                    ));
                }
                Ok(())
            },

            WitnessExpectation::HasAssociatedType { name, ty } => {
                let binding = def.type_bindings.get(name).ok_or_else(|| {
                    let available: Vec<_> = def.type_bindings.keys().cloned().collect();
                    format!(
                        "Witness for {} : {} does not have associated type '{}'. Available: {:?}",
                        self.implementing_type, self.protocol, name, available
                    )
                })?;
                if !ty.matches(*binding, mir_ctx.mir) {
                    return Err(format!(
                        "Witness associated type '{}' is '{}', expected '{}'",
                        name,
                        mir_ctx.mir.ty(*binding).display(mir_ctx.mir),
                        ty.display()
                    ));
                }
                Ok(())
            },

            WitnessExpectation::HasAssociatedTypeByName { name } => {
                if !def.type_bindings.contains_key(name) {
                    let available: Vec<_> = def.type_bindings.keys().cloned().collect();
                    return Err(format!(
                        "Witness for {} : {} does not have associated type '{}'. Available: {:?}",
                        self.implementing_type, self.protocol, name, available
                    ));
                }
                Ok(())
            },
        }
    }
}

impl Expectable for MirWitness {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        if crate::skip_codegen() {
            return Ok(());
        }
        let mir_result = ctx.mir();
        let mir_ctx = MirTestContext::new(mir_result);

        let Some((_, def)) = mir_ctx.find_witness(&self.implementing_type, &self.protocol) else {
            let available: Vec<_> = mir_ctx
                .mir
                .witnesses
                .iter()
                .map(|(_, d)| {
                    let impl_ty = mir_ctx.mir.ty(d.implementing_type);
                    let protocol = mir_ctx.mir.name(d.protocol);
                    format!("{} : {}", impl_ty.display(mir_ctx.mir), protocol)
                })
                .collect();
            let diagnostics = mir_ctx.format_diagnostics(&ctx.diagnostics);
            let mir = mir_ctx.format_mir();
            return Err(format!(
                "Witness for {} : {} not found in MIR. Available witnesses: {:?}\n\n{}\n--- MIR ---\n{}",
                self.implementing_type, self.protocol, available, diagnostics, mir
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
