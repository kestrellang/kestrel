//! Protocol expectations for MIR testing.

use crate::mir::context::MirTestContext;
use crate::{Expectable, TestContext};
use kestrel_execution_graph::ProtocolDef;

/// Expectations for a protocol definition in the MIR.
pub struct MirProtocol {
    name: String,
    expectations: Vec<ProtocolExpectation>,
}

enum ProtocolExpectation {
    HasMethod(String),
    MethodCount(usize),
    HasAssociatedType(String),
    TypeParamCount(usize),
}

impl MirProtocol {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            expectations: Vec::new(),
        }
    }

    /// Expect the protocol to have a method with the given name.
    pub fn has_method(mut self, name: &str) -> Self {
        self.expectations
            .push(ProtocolExpectation::HasMethod(name.to_string()));
        self
    }

    /// Expect exactly N methods.
    pub fn has_method_count(mut self, n: usize) -> Self {
        self.expectations.push(ProtocolExpectation::MethodCount(n));
        self
    }

    /// Expect the protocol to have an associated type with the given name.
    pub fn has_associated_type(mut self, name: &str) -> Self {
        self.expectations
            .push(ProtocolExpectation::HasAssociatedType(name.to_string()));
        self
    }

    /// Expect exactly N type parameters.
    pub fn has_type_params(mut self, n: usize) -> Self {
        self.expectations
            .push(ProtocolExpectation::TypeParamCount(n));
        self
    }

    fn check_expectation(
        &self,
        expectation: &ProtocolExpectation,
        def: &ProtocolDef,
        mir_ctx: &MirTestContext,
    ) -> Result<(), String> {
        match expectation {
            ProtocolExpectation::HasMethod(name) => {
                let found = def.methods.iter().any(|&m| {
                    let method = &mir_ctx.mir.protocol_methods[m];
                    method.name == *name
                });
                if !found {
                    let available: Vec<_> = def
                        .methods
                        .iter()
                        .map(|&m| mir_ctx.mir.protocol_methods[m].name.clone())
                        .collect();
                    return Err(format!(
                        "Protocol '{}' does not have method '{}'. Available: {:?}",
                        self.name, name, available
                    ));
                }
                Ok(())
            }

            ProtocolExpectation::MethodCount(expected) => {
                let actual = def.methods.len();
                if actual != *expected {
                    return Err(format!(
                        "Protocol '{}' has {} method(s), expected {}",
                        self.name, actual, expected
                    ));
                }
                Ok(())
            }

            ProtocolExpectation::HasAssociatedType(name) => {
                let found = def.associated_types.iter().any(|&at| {
                    let assoc = &mir_ctx.mir.associated_types[at];
                    assoc.name == *name
                });
                if !found {
                    let available: Vec<_> = def
                        .associated_types
                        .iter()
                        .map(|&at| mir_ctx.mir.associated_types[at].name.clone())
                        .collect();
                    return Err(format!(
                        "Protocol '{}' does not have associated type '{}'. Available: {:?}",
                        self.name, name, available
                    ));
                }
                Ok(())
            }

            ProtocolExpectation::TypeParamCount(expected) => {
                let actual = def.type_params.len();
                if actual != *expected {
                    return Err(format!(
                        "Protocol '{}' has {} type parameter(s), expected {}",
                        self.name, actual, expected
                    ));
                }
                Ok(())
            }
        }
    }
}

impl Expectable for MirProtocol {
    fn check(&self, ctx: &TestContext) -> Result<(), String> {
        let mir_result = ctx.mir();
        let mir_ctx = MirTestContext::new(mir_result);

        let Some((_, def)) = mir_ctx.find_protocol(&self.name) else {
            let available: Vec<_> = mir_ctx
                .mir
                .protocols
                .iter()
                .map(|(_, d)| mir_ctx.mir.name(d.name).to_string())
                .collect();
            let diagnostics = mir_ctx.format_diagnostics(&ctx.diagnostics);
            let mir = mir_ctx.format_mir();
            return Err(format!(
                "Protocol '{}' not found in MIR. Available protocols: {:?}\n\n{}\n--- MIR ---\n{}",
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
