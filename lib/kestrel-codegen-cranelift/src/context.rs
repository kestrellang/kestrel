//! Code generation context.

use crate::CodegenOptions;
use crate::error::CodegenError;
use crate::monomorphize::{FunctionInstantiation, MonomorphizationSet, build_substitution};
use crate::types::translate_type_ext;
use kestrel_codegen::{LayoutCache, TargetConfig, mangle_function_with_self, mangle_name};
use kestrel_execution_graph::{Function, FunctionDef, Id, MirContext, QualifiedName, Ty};

use cranelift_codegen::Context as CraneliftContext;
use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::{AbiParam, Function as CraneliftFunction, Signature, UserFuncName};
use cranelift_codegen::isa::{CallConv, TargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
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
    /// Map from string literal content to data section ID.
    pub string_data: HashMap<String, DataId>,
    /// The set of all instantiations discovered during collection.
    pub mono_set: MonomorphizationSet,
}

impl<'a> CodegenContext<'a> {
    /// Create a new code generation context.
    pub fn new(
        mir: &'a MirContext,
        target: &'a TargetConfig,
        options: &'a CodegenOptions,
        mono_set: MonomorphizationSet,
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
            string_data: HashMap::new(),
            mono_set,
        })
    }

    /// Compile all functions in the MIR context.
    pub fn compile_all(&mut self) -> Result<(), CodegenError> {
        // First pass: declare and define all static variables
        self.define_all_statics()?;

        // Second pass: declare all functions (including runtime helpers)
        self.declare_all_functions()?;
        self.declare_runtime_helpers()?;

        // Third pass: define all functions
        self.define_all_functions()?;

        Ok(())
    }

    /// Define all static variables in the module.
    ///
    /// This creates global data entries for each static variable in the MIR.
    fn define_all_statics(&mut self) -> Result<(), CodegenError> {
        use crate::monomorphize::Substitution;
        use crate::types::translate_type_with_subst;

        // Collect statics to avoid borrow issues
        let statics: Vec<_> = self
            .mir
            .statics
            .iter()
            .map(|(id, def)| (id, def.name, def.ty, def.file_constant_data.clone()))
            .collect();

        for (_static_id, name_id, ty, file_constant_data) in statics {
            let mangled_name = mangle_name(self.mir, name_id, &[]);

            // Check if this is a file constant
            if let Some(fc_data) = file_constant_data {
                self.define_file_constant_static(&mangled_name, &fc_data)?;
                continue;
            }

            // Normal static: compute size and create zero-initialized data
            let empty_subst = Substitution::new();
            let cl_type = translate_type_with_subst(self.mir, ty, self.target, &empty_subst);
            let size = cl_type.bytes() as usize;

            // Create zeroed data for the static
            let mut desc = DataDescription::new();
            desc.define_zeroinit(size);

            // Declare and define the data
            let data_id = self
                .module
                .declare_data(&mangled_name, Linkage::Export, true, false)
                .map_err(|e| {
                    CodegenError::DataSection(format!(
                        "failed to declare static '{}': {}",
                        mangled_name, e
                    ))
                })?;

            self.module.define_data(data_id, &desc).map_err(|e| {
                CodegenError::DataSection(format!(
                    "failed to define static '{}': {}",
                    mangled_name, e
                ))
            })?;
        }

        Ok(())
    }

    /// Define a file constant static (for @fileconstant).
    ///
    /// This embeds the file bytes in .rodata and creates a LiteralSlice struct
    /// pointing to the embedded data.
    fn define_file_constant_static(
        &mut self,
        name: &str,
        fc_data: &kestrel_execution_graph::FileConstantData,
    ) -> Result<(), CodegenError> {
        let ptr_size = if self.target.is_64bit() { 8 } else { 4 };

        // Resolve the file path - either relative to the source file's directory
        // or relative to the current working directory
        let resolved_path = if let Some(base) = &fc_data.base_path {
            base.join(&fc_data.relative_path)
        } else {
            std::path::PathBuf::from(&fc_data.relative_path)
        };

        // Read the file from the resolved path
        let file_bytes = std::fs::read(&resolved_path).map_err(|e| {
            CodegenError::DataSection(format!(
                "failed to read file constant '{}': {}",
                resolved_path.display(),
                e
            ))
        })?;

        // Compute element size and alignment from element_ty using layout cache
        // (translate_type returns pointer type for structs, which gives wrong size)
        let element_layout = self.layouts.layout_of(fc_data.element_ty);
        let element_size = element_layout.size;
        let element_align = element_layout.align;

        // Validate file size is aligned to element size
        if element_size > 0 && file_bytes.len() % element_size != 0 {
            return Err(CodegenError::DataSection(format!(
                "file '{}' size ({}) is not aligned to element size ({})",
                resolved_path.display(),
                file_bytes.len(),
                element_size
            )));
        }

        // Compute element count
        let count = if element_size > 0 {
            file_bytes.len() / element_size
        } else {
            0
        };

        // 1. Embed the raw file bytes in .rodata
        let data_name = format!("{}_data", name);
        let mut data_desc = DataDescription::new();
        data_desc.define(file_bytes.into_boxed_slice());
        // Set alignment to at least 8 bytes to avoid unaligned pointer warnings
        data_desc.set_align(element_align.max(8) as u64);

        let data_id = self
            .module
            .declare_data(&data_name, Linkage::Local, false, false)
            .map_err(|e| CodegenError::DataSection(format!("failed to declare data: {}", e)))?;

        self.module
            .define_data(data_id, &data_desc)
            .map_err(|e| CodegenError::DataSection(format!("failed to define data: {}", e)))?;

        // 2. Create the LiteralSlice struct in .data
        // LiteralSlice[T] = { ptr: lang.ptr[T], len: lang.i64 }
        // On 64-bit: ptr at offset 0 (8 bytes), len at offset 8 (8 bytes) = 16 bytes total
        let slice_size = ptr_size * 2;
        let mut slice_bytes = vec![0u8; slice_size];

        // Write count at offset ptr_size (len field)
        let count_bytes = (count as i64).to_le_bytes();
        slice_bytes[ptr_size..ptr_size + 8].copy_from_slice(&count_bytes[..8.min(ptr_size)]);

        // Create the slice data
        let mut slice_desc = DataDescription::new();
        slice_desc.define(slice_bytes.into_boxed_slice());
        // Set alignment to pointer size for the slice struct
        slice_desc.set_align(ptr_size as u64);

        // Add a relocation for the pointer field (points to the embedded data)
        let data_ref = self.module.declare_data_in_data(data_id, &mut slice_desc);
        slice_desc.write_data_addr(0, data_ref, 0);

        // Declare and define the LiteralSlice static
        let slice_id = self
            .module
            .declare_data(name, Linkage::Export, true, false)
            .map_err(|e| {
                CodegenError::DataSection(format!("failed to declare slice static: {}", e))
            })?;

        self.module
            .define_data(slice_id, &slice_desc)
            .map_err(|e| {
                CodegenError::DataSection(format!("failed to define slice static: {}", e))
            })?;

        Ok(())
    }

    /// Resolve the symbol name for a function instantiation.
    pub(crate) fn symbol_name_for_instantiation(&self, inst: &FunctionInstantiation) -> String {
        let func_def = &self.mir.functions[inst.func_id];
        self.symbol_name_for_function(inst.func_id, func_def, &inst.type_args, inst.self_type)
    }

    /// Resolve the symbol name for a function definition and concrete type args.
    pub(crate) fn symbol_name_for_function(
        &self,
        func_id: Id<Function>,
        func_def: &FunctionDef,
        type_args: &[Id<Ty>],
        self_type: Option<Id<Ty>>,
    ) -> String {
        if self.is_main(func_def) {
            "main".to_string()
        } else if let Some(extern_info) = &func_def.extern_info {
            extern_info.symbol_name.clone()
        } else {
            mangle_function_with_self(self.mir, func_id, type_args, self_type)
        }
    }

    /// Resolve a symbol name by qualified name, falling back to mangling when unknown.
    pub(crate) fn resolve_symbol_name(
        &self,
        name: Id<QualifiedName>,
        type_args: &[Id<Ty>],
        self_type: Option<Id<Ty>>,
    ) -> String {
        if let Some((func_id, func_def)) =
            self.mir.functions.iter().find(|(_, def)| def.name == name)
        {
            self.symbol_name_for_function(func_id, func_def, type_args, self_type)
        } else {
            mangle_name(self.mir, name, type_args)
        }
    }

    /// Resolve linkage for a function definition.
    fn linkage_for_function(&self, func_def: &FunctionDef) -> Linkage {
        if self.is_main(func_def) {
            Linkage::Export
        } else if func_def.extern_info.is_some() {
            Linkage::Import
        } else {
            Linkage::Local
        }
    }

    /// Declare runtime helper functions (e.g., memcmp for string comparison).
    fn declare_runtime_helpers(&mut self) -> Result<(), CodegenError> {
        // Declare memcmp for string comparison
        let ptr_type = if self.target.is_64bit() {
            cranelift_codegen::ir::types::I64
        } else {
            cranelift_codegen::ir::types::I32
        };

        let mut memcmp_sig =
            cranelift_codegen::ir::Signature::new(cranelift_codegen::isa::CallConv::SystemV);
        memcmp_sig
            .params
            .push(cranelift_codegen::ir::AbiParam::new(ptr_type));
        memcmp_sig
            .params
            .push(cranelift_codegen::ir::AbiParam::new(ptr_type));
        memcmp_sig
            .params
            .push(cranelift_codegen::ir::AbiParam::new(ptr_type));
        memcmp_sig
            .returns
            .push(cranelift_codegen::ir::AbiParam::new(
                cranelift_codegen::ir::types::I32,
            ));

        let memcmp_id = self
            .module
            .declare_function("memcmp", Linkage::Import, &memcmp_sig)
            .map_err(|e| CodegenError::FunctionDefinition {
                name: "memcmp".to_string(),
                error: e.to_string(),
            })?;

        self.func_ids_by_name
            .insert("memcmp".to_string(), memcmp_id);

        Ok(())
    }

    /// Declare all functions in the module.
    ///
    /// This declares both non-generic functions and all discovered
    /// instantiations of generic functions.
    fn declare_all_functions(&mut self) -> Result<(), CodegenError> {
        // Collect all instantiations to declare
        let instantiations: Vec<_> = self.mono_set.functions.iter().cloned().collect();

        for inst in instantiations {
            let func_def = &self.mir.functions[inst.func_id];
            let symbol_name = self.symbol_name_for_instantiation(&inst);
            let linkage = self.linkage_for_function(func_def);

            // Skip if already declared (can happen with multiple paths to same instantiation)
            if self.func_ids_by_name.contains_key(&symbol_name) {
                continue;
            }

            let sig = self.create_signature_with_subst(func_def, &inst.type_args, inst.self_type);

            let cl_func_id = self
                .module
                .declare_function(&symbol_name, linkage, &sig)
                .map_err(|e| CodegenError::FunctionDefinition {
                    name: symbol_name.clone(),
                    error: e.to_string(),
                })?;

            self.func_ids_by_name.insert(symbol_name, cl_func_id);
        }
        Ok(())
    }

    /// Define all functions.
    ///
    /// This compiles each instantiation with its corresponding substitution.
    fn define_all_functions(&mut self) -> Result<(), CodegenError> {
        // Collect instantiations to define
        let instantiations: Vec<_> = self.mono_set.functions.iter().cloned().collect();

        for inst in instantiations {
            let func_def = &self.mir.functions[inst.func_id];
            // Skip extern functions - they have no body to compile
            if func_def.is_extern() {
                continue;
            }
            self.compile_function_instantiation(&inst)?;
        }
        Ok(())
    }

    /// Compile a single function instantiation.
    fn compile_function_instantiation(
        &mut self,
        inst: &FunctionInstantiation,
    ) -> Result<(), CodegenError> {
        let func_def = &self.mir.functions[inst.func_id];
        let is_main = self.is_main(func_def);
        let symbol_name = self.symbol_name_for_instantiation(inst);

        eprintln!("\n=== DEBUG: Compiling function ===");
        eprintln!("Function: {}", self.mir.name(func_def.name));
        eprintln!("Symbol: {}", symbol_name);
        eprintln!("Type args: {:?}", inst.type_args);
        eprintln!("Self type: {:?}", inst.self_type);
        if !inst.type_args.is_empty() {
            eprintln!("Type arg details:");
            for (i, &ty) in inst.type_args.iter().enumerate() {
                eprintln!("  [{}] {:?} -> {:?}", i, ty, self.mir.ty(ty));
            }
        }
        if let Some(st) = inst.self_type {
            eprintln!("Self type details: {:?} -> {:?}", st, self.mir.ty(st));
        }

        if !func_def.type_params.is_empty() && func_def.type_params.len() != inst.type_args.len() {
            return Err(CodegenError::Unsupported(format!(
                "missing type arguments for function instantiation '{}': expected {}, got {}",
                self.mir.name(func_def.name),
                func_def.type_params.len(),
                inst.type_args.len()
            )));
        }

        let cl_func_id = *self.func_ids_by_name.get(&symbol_name).ok_or_else(|| {
            CodegenError::FunctionDefinition {
                name: symbol_name.clone(),
                error: "function not declared".to_string(),
            }
        })?;

        // Build the substitution for this instantiation
        let mut subst = build_substitution(self.mir, &func_def.type_params, &inst.type_args);

        // Set self_type if this instantiation has one (protocol extension methods)
        if let Some(st) = inst.self_type {
            subst.set_self_type(st);
        }

        let sig = self.create_signature_with_subst(func_def, &inst.type_args, inst.self_type);
        let mut cl_func =
            CraneliftFunction::with_name_signature(UserFuncName::user(0, cl_func_id.as_u32()), sig);

        // Compile the function body with substitution
        crate::function::compile_function_body(self, func_def, &subst, &mut cl_func, is_main)?;

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

        // Debug: print generated IR
        if std::env::var("KESTREL_DEBUG_IR").is_ok() {
            eprintln!(
                "=== Generated IR for {} ===\n{}\n",
                symbol_name,
                cl_func.display()
            );
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

    /// Create a Cranelift signature for a function with type argument substitution.
    fn create_signature_with_subst(
        &self,
        func_def: &FunctionDef,
        type_args: &[Id<Ty>],
        self_type: Option<Id<Ty>>,
    ) -> Signature {
        // Use C calling convention for extern functions, default otherwise
        let call_conv = if func_def.is_extern() {
            self.c_call_conv()
        } else {
            self.isa.default_call_conv()
        };
        let mut sig = Signature::new(call_conv);

        // Build substitution
        let mut subst = build_substitution(self.mir, &func_def.type_params, type_args);
        if let Some(st) = self_type {
            subst.set_self_type(st);
        }

        // Return type (used for sret decisions)
        let is_main = self.is_main(func_def);
        let concrete_ret = subst
            .apply_ty_readonly(self.mir, func_def.ret)
            .expect("type substitution failed for return type");
        let ret_ty = self.mir.ty(concrete_ret);
        let is_aggregate_ret = matches!(
            ret_ty,
            kestrel_execution_graph::MirTy::Tuple(_)
                | kestrel_execution_graph::MirTy::Named { .. }
                | kestrel_execution_graph::MirTy::Str
                | kestrel_execution_graph::MirTy::FuncThick { .. }
        );
        let needs_sret = !func_def.is_extern()
            && !is_main
            && !matches!(ret_ty, kestrel_execution_graph::MirTy::Unit)
            && is_aggregate_ret;

        if needs_sret {
            let ptr_type = if self.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };
            sig.params.push(AbiParam::new(ptr_type));
        }

        // Parameters - apply substitution to get concrete types
        for &param_id in &func_def.params {
            let param = &self.mir.params[param_id];
            // Collection phase should have interned all types, so this should always succeed.
            let concrete_ty = subst
                .apply_ty_readonly(self.mir, param.ty)
                .expect("type substitution failed for param type");
            let cl_type =
                translate_type_ext(self.mir, concrete_ty, self.target, func_def.is_extern());
            sig.params.push(AbiParam::new(cl_type));
        }

        // Return type
        // Special case: main() must return i64 for C runtime even if Kestrel return type is Unit
        if is_main {
            // C runtime expects int main() - always return i64
            sig.returns.push(AbiParam::new(cl_types::I64));
        } else if !matches!(ret_ty, kestrel_execution_graph::MirTy::Unit) && !needs_sret {
            let cl_type =
                translate_type_ext(self.mir, concrete_ret, self.target, func_def.is_extern());
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

    /// Get the C calling convention for the target platform.
    ///
    /// Uses SystemV for Unix-like systems (Linux, macOS, BSD) and
    /// WindowsFastcall for Windows.
    fn c_call_conv(&self) -> CallConv {
        use target_lexicon::OperatingSystem;
        match self.target.triple.operating_system {
            OperatingSystem::Windows => CallConv::WindowsFastcall,
            _ => CallConv::SystemV,
        }
    }

    /// Add a string literal to the data section.
    ///
    /// Returns the DataId for the string, creating a new entry if needed.
    /// Deduplicates identical strings.
    pub fn add_string_data(&mut self, s: &str) -> Result<DataId, CodegenError> {
        // Check if we already have this string
        if let Some(&id) = self.string_data.get(s) {
            return Ok(id);
        }

        // Create new data
        let mut desc = DataDescription::new();
        desc.define(s.as_bytes().to_vec().into_boxed_slice());

        let name = format!("str_{}", self.string_data.len());
        let data_id = self
            .module
            .declare_data(&name, Linkage::Local, false, false)
            .map_err(|e| CodegenError::DataSection(e.to_string()))?;

        self.module
            .define_data(data_id, &desc)
            .map_err(|e| CodegenError::DataSection(e.to_string()))?;

        self.string_data.insert(s.to_string(), data_id);
        Ok(data_id)
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
    _target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<Arc<dyn TargetIsa>, CodegenError> {
    let mut flags_builder = settings::builder();

    // Set optimization level
    match options.opt_level {
        0 => {
            flags_builder.set("opt_level", "none").unwrap();
        },
        1 => {
            flags_builder.set("opt_level", "speed").unwrap();
        },
        _ => {
            flags_builder.set("opt_level", "speed_and_size").unwrap();
        },
    }

    // Enable position-independent code for shared libraries
    flags_builder.set("is_pic", "true").unwrap();

    let flags = settings::Flags::new(flags_builder);

    cranelift_native::builder()
        .map_err(|e| CodegenError::ModuleCreation(e.to_string()))?
        .finish(flags)
        .map_err(|e| CodegenError::ModuleCreation(e.to_string()))
}
