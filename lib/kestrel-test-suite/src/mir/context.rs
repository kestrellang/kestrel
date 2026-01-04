//! Internal MIR test context.

use kestrel_execution_graph::{
    Enum, EnumDef, Function, FunctionDef, Id, MirContext, Protocol, ProtocolDef, Struct, StructDef,
    Witness, WitnessDef,
};
use kestrel_execution_graph_lowering::LoweringResult;
use kestrel_reporting::DiagnosticContext;

/// Internal context for MIR testing.
pub(crate) struct MirTestContext<'a> {
    pub mir: &'a MirContext,
    pub lowering_result: &'a LoweringResult,
}

impl<'a> MirTestContext<'a> {
    pub fn new(lowering_result: &'a LoweringResult) -> Self {
        Self {
            mir: &lowering_result.mir,
            lowering_result,
        }
    }

    /// Find a struct by its fully qualified name.
    pub fn find_struct(&self, name: &str) -> Option<(Id<Struct>, &StructDef)> {
        for (id, def) in self.mir.structs.iter() {
            let struct_name = self.mir.name(def.name);
            if struct_name.to_string() == name {
                return Some((id, def));
            }
        }
        None
    }

    /// Find an enum by its fully qualified name.
    pub fn find_enum(&self, name: &str) -> Option<(Id<Enum>, &EnumDef)> {
        for (id, def) in self.mir.enums.iter() {
            let enum_name = self.mir.name(def.name);
            if enum_name.to_string() == name {
                return Some((id, def));
            }
        }
        None
    }

    /// Find a function by its fully qualified name.
    pub fn find_function(&self, name: &str) -> Option<(Id<Function>, &FunctionDef)> {
        for (id, def) in self.mir.functions.iter() {
            let func_name = self.mir.name(def.name);
            if func_name.to_string() == name {
                return Some((id, def));
            }
        }
        None
    }

    /// Find a witness by implementing type and protocol names.
    pub fn find_witness(
        &self,
        impl_type: &str,
        protocol: &str,
    ) -> Option<(Id<Witness>, &WitnessDef)> {
        for (id, def) in self.mir.witnesses.iter() {
            let impl_ty = self.mir.ty(def.implementing_type);
            let impl_ty_str = impl_ty.display(self.mir).to_string();
            let protocol_name = self.mir.name(def.protocol);
            if impl_ty_str == impl_type && protocol_name.to_string() == protocol {
                return Some((id, def));
            }
        }
        None
    }

    /// Find a protocol by its fully qualified name.
    pub fn find_protocol(&self, name: &str) -> Option<(Id<Protocol>, &ProtocolDef)> {
        for (id, def) in self.mir.protocols.iter() {
            let protocol_name = self.mir.name(def.name);
            if protocol_name.to_string() == name {
                return Some((id, def));
            }
        }
        None
    }

    /// Format the entire MIR for display in error messages.
    pub fn format_mir(&self) -> String {
        self.mir.display().to_string()
    }

    /// Check if there are any lowering errors.
    pub fn has_lowering_errors(&self) -> bool {
        self.lowering_result
            .diagnostics
            .iter()
            .any(|d| d.severity == kestrel_reporting::Severity::Error)
    }

    /// Format all diagnostics (semantic + lowering) for display.
    pub fn format_diagnostics(&self, semantic_diagnostics: &DiagnosticContext) -> String {
        let mut output = String::new();

        if !semantic_diagnostics.is_empty() {
            output.push_str("--- Semantic Diagnostics ---\n");
            for diag in semantic_diagnostics.diagnostics() {
                output.push_str(&format!("  {:?}: {}\n", diag.severity, diag.message));
            }
            output.push('\n');
        }

        if !self.lowering_result.diagnostics.is_empty() {
            output.push_str("--- Lowering Diagnostics ---\n");
            for diag in &self.lowering_result.diagnostics {
                output.push_str(&format!("  {:?}: {}\n", diag.severity, diag.message));
            }
            output.push('\n');
        }

        output
    }
}
