//! Code generation context.

use crate::error::CodegenError;
use crate::types::translate_type;
use crate::CodegenOptions;
use kestrel_codegen::{mangle_name, Layout, LayoutCache, TargetConfig};
use kestrel_execution_graph::{Function, FunctionDef, Id, MirContext, QualifiedNameData, Ty};

use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::{AbiParam, Function as CraneliftFunction, Signature, UserFuncName};
use cranelift_codegen::isa::{CallConv, TargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::Context as CraneliftContext;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};

use std::collections::HashMap;
use std::sync::Arc;

/// Main code generation context.
pub struct CodegenContext<'a> {
    /// The MIR being compiled.
    pub mir: &'a MirContext,
    /// Target configuration.
    pub target: &'a TargetConfig,
    /// Code generation options.
    pub options: &'a CodegenOptions,
    /// The Cranelift object module.
    pub module: ObjectModule,
    /// Target ISA.
    pub isa: Arc<dyn TargetIsa>,
    /// Layout cache for type sizes.
    pub layouts: LayoutCache<'a>,
    /// Map from MIR function ID to Cranelift function ID.
    pub func_ids: HashMap<Id<Function>, FuncId>,
    /// Map from mangled name to Cranelift function ID.
    pub func_ids_by_name: HashMap<String, FuncId>,
    /// Function builder context (reused across functions).
    pub func_builder_ctx: FunctionBuilderContext,
}

impl<'a> CodegenContext<'a> {
    /// Create a new code generation context.
    pub fn new(
        mir: &'a MirContext,
        target: &'a TargetConfig,
        options: &'a CodegenOptions,
    ) -> Result<Self, CodegenError> {
        // Create ISA
        let isa = create_isa(target, options)?;

        // Create object module
        let builder = ObjectBuilder::new(
            isa.clone(),
            "kestrel_module",
            cranelift_module::default_libcall_names(),
        )
        .map_err(|e| CodegenError::ModuleCreation(e.to_string()))?;

        let module = ObjectModule::new(builder);
        let layouts = LayoutCache::new(mir, target);

        Ok(Self {
            mir,
            target,
            options,
            module,
            isa,
            layouts,
            func_ids: HashMap::new(),
            func_ids_by_name: HashMap::new(),
            func_builder_ctx: FunctionBuilderContext::new(),
        })
    }

    /// Compile all functions in the MIR context.
    pub fn compile_all(&mut self) -> Result<(), CodegenError> {
        // First pass: declare all functions
        self.declare_all_functions()?;

        // Second pass: define all functions
        self.define_all_functions()?;

        Ok(())
    }

    /// Declare all functions in the module.
    fn declare_all_functions(&mut self) -> Result<(), CodegenError> {
        for (func_id, func_def) in self.mir.functions.iter() {
            let is_main = self.is_main(func_def);

            // Main function is exported as "main" for the C runtime
            // Other functions use mangled names
            let (symbol_name, linkage) = if is_main {
                ("main".to_string(), Linkage::Export)
            } else {
                (mangle_name(self.mir, func_def.name, &[]), Linkage::Local)
            };

            let sig = self.create_signature(func_def);

            let cl_func_id = self
                .module
                .declare_function(&symbol_name, linkage, &sig)
                .map_err(|e| CodegenError::FunctionDefinition {
                    name: symbol_name.clone(),
                    error: e.to_string(),
                })?;

            self.func_ids.insert(func_id, cl_func_id);
            self.func_ids_by_name.insert(symbol_name, cl_func_id);
        }
        Ok(())
    }

    /// Define all functions.
    fn define_all_functions(&mut self) -> Result<(), CodegenError> {
        for (func_id, func_def) in self.mir.functions.iter() {
            self.compile_function(func_id, func_def)?;
        }
        Ok(())
    }

    /// Compile a single function.
    fn compile_function(
        &mut self,
        func_id: Id<Function>,
        func_def: &FunctionDef,
    ) -> Result<(), CodegenError> {
        let is_main = self.is_main(func_def);
        let symbol_name = if is_main {
            "main".to_string()
        } else {
            mangle_name(self.mir, func_def.name, &[])
        };
        let cl_func_id = self.func_ids[&func_id];

        let sig = self.create_signature(func_def);
        let mut cl_func =
            CraneliftFunction::with_name_signature(UserFuncName::user(0, cl_func_id.as_u32()), sig);

        // Compile the function body
        crate::function::compile_function_body(self, func_def, &mut cl_func, is_main)?;

        // Verify the function before defining it
        if let Err(verifier_errors) =
            cranelift_codegen::verifier::verify_function(&cl_func, self.isa.as_ref())
        {
            return Err(CodegenError::FunctionDefinition {
                name: symbol_name,
                error: format!(
                    "Verifier errors:\n{}\n\nFunction IR:\n{}",
                    verifier_errors,
                    cl_func.display()
                ),
            });
        }

        // Define the function in the module
        let mut ctx = CraneliftContext::for_function(cl_func);
        self.module
            .define_function(cl_func_id, &mut ctx)
            .map_err(|e| CodegenError::FunctionDefinition {
                name: symbol_name,
                error: e.to_string(),
            })?;

        Ok(())
    }

    /// Create a Cranelift signature for a function.
    fn create_signature(&self, func_def: &FunctionDef) -> Signature {
        let call_conv = self.isa.default_call_conv();
        let mut sig = Signature::new(call_conv);

        // Parameters - all passed by pointer
        for &param_id in &func_def.params {
            let param = &self.mir.params[param_id];
            let cl_type = translate_type(self.mir, param.ty, self.target);
            sig.params.push(AbiParam::new(cl_type));
        }

        // Return type
        // Special case: main() must return i64 for C runtime even if Kestrel return type is Unit
        let is_main = self.is_main(func_def);
        let ret_ty = self.mir.ty(func_def.ret);
        if is_main {
            // C runtime expects int main() - always return i64
            sig.returns.push(AbiParam::new(cl_types::I64));
        } else if !matches!(ret_ty, kestrel_execution_graph::MirTy::Unit) {
            let cl_type = translate_type(self.mir, func_def.ret, self.target);
            sig.returns.push(AbiParam::new(cl_type));
        }

        sig
    }

    /// Check if a function is the main entry point.
    ///
    /// The main function is identified by having "main" as the last segment.
    /// This works whether it's a top-level `main` or `Module.main`.
    fn is_main(&self, func_def: &FunctionDef) -> bool {
        let name = self.mir.name(func_def.name);
        name.segments.last().map(|s| s.as_str()) == Some("main")
    }

    /// Finish compilation and return the object file bytes.
    pub fn finish(self) -> Result<Vec<u8>, CodegenError> {
        let product = self.module.finish();
        let bytes = product
            .emit()
            .map_err(|e| CodegenError::ModuleFinish(e.to_string()))?;
        Ok(bytes)
    }
}

/// Create a Cranelift target ISA from the target config.
fn create_isa(
    target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<Arc<dyn TargetIsa>, CodegenError> {
    let mut flags_builder = settings::builder();

    // Set optimization level
    match options.opt_level {
        0 => {
            flags_builder.set("opt_level", "none").unwrap();
        }
        1 => {
            flags_builder.set("opt_level", "speed").unwrap();
        }
        _ => {
            flags_builder.set("opt_level", "speed_and_size").unwrap();
        }
    }

    // Enable position-independent code for shared libraries
    flags_builder.set("is_pic", "true").unwrap();

    let flags = settings::Flags::new(flags_builder);

    cranelift_native::builder()
        .map_err(|e| CodegenError::ModuleCreation(e.to_string()))?
        .finish(flags)
        .map_err(|e| CodegenError::ModuleCreation(e.to_string()))
}
