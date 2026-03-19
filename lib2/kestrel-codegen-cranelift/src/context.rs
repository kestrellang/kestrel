//! Code generation context — manages the Cranelift module and compilation state.
//!
//! Orchestrates the three-pass compilation:
//! 1. Define all statics (data section entries)
//! 2. Declare all functions (forward declarations)
//! 3. Define all functions (compile bodies)

use crate::common::{self, needs_sret};
use crate::error::CodegenError;
use crate::function;
use crate::monomorphize::{FunctionInstantiation, MonomorphizationSet};
use crate::types;
use cranelift_codegen::ir::{self, AbiParam, Signature};
use cranelift_codegen::isa::{CallConv, TargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{DataDescription, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use kestrel_codegen2::{
    mangle_function_with_self, substitute_type, LayoutCache, Mangler, TargetConfig,
};
use kestrel_debug::ktrace;
use kestrel_hecs::Entity;
use kestrel_mir::{
    CallingConvention, FunctionDef, FunctionKind, ImmediateKind, MirModule, MirTy, StaticDef,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::CodegenOptions;

/// Central code generation state.
pub struct CodegenContext<'a> {
    pub module: &'a MirModule,
    pub target: &'a TargetConfig,
    pub options: &'a CodegenOptions,
    pub cl_module: ObjectModule,
    pub isa: Arc<dyn TargetIsa>,
    pub layouts: LayoutCache<'a>,
    /// Mangled symbol name → Cranelift function ID
    pub func_ids_by_name: HashMap<String, FuncId>,
    /// String literal deduplication
    pub string_data: HashMap<String, cranelift_module::DataId>,
    pub mono_set: MonomorphizationSet,
    /// Entity → FunctionId for fast lookup
    pub entity_to_func: HashMap<Entity, kestrel_mir::FunctionId>,
    /// Reusable function builder context
    pub func_builder_ctx: FunctionBuilderContext,
}

impl<'a> CodegenContext<'a> {
    pub fn new(
        module: &'a MirModule,
        target: &'a TargetConfig,
        options: &'a CodegenOptions,
        mono_set: MonomorphizationSet,
    ) -> Result<Self, CodegenError> {
        // Configure Cranelift
        let mut flag_builder = settings::builder();
        flag_builder.set("is_pic", "false").unwrap();

        match options.opt_level {
            0 => flag_builder.set("opt_level", "none").unwrap(),
            1 => flag_builder.set("opt_level", "speed").unwrap(),
            _ => flag_builder.set("opt_level", "speed_and_size").unwrap(),
        }

        let flags = settings::Flags::new(flag_builder);
        let isa = cranelift_native::builder()
            .map_err(|e| CodegenError::ModuleCreation(format!("native ISA: {e}")))?
            .finish(flags)
            .map_err(|e| CodegenError::ModuleCreation(format!("ISA finish: {e}")))?;

        let obj_builder =
            ObjectBuilder::new(isa.clone(), "kestrel_module", cranelift_module::default_libcall_names())
                .map_err(|e| CodegenError::ModuleCreation(format!("ObjectBuilder: {e}")))?;

        let cl_module = ObjectModule::new(obj_builder);
        let layouts = LayoutCache::new(module, target);

        let entity_to_func: HashMap<Entity, kestrel_mir::FunctionId> = module
            .functions
            .iter()
            .enumerate()
            .map(|(i, f)| (f.entity, kestrel_mir::FunctionId::new(i)))
            .collect();

        Ok(Self {
            module,
            target,
            options,
            cl_module,
            isa,
            layouts,
            func_ids_by_name: HashMap::new(),
            string_data: HashMap::new(),
            mono_set,
            entity_to_func,
            func_builder_ctx: FunctionBuilderContext::new(),
        })
    }

    /// Run the three-pass compilation.
    pub fn compile_all(&mut self) -> Result<(), CodegenError> {
        self.define_all_statics()?;
        self.declare_all_functions()?;
        self.define_all_functions()?;
        Ok(())
    }

    /// Emit the finished object file bytes.
    pub fn finish(self) -> Result<Vec<u8>, CodegenError> {
        let product = self
            .cl_module
            .finish()
            .emit()
            .map_err(|e| CodegenError::ModuleFinish(format!("{e}")))?;
        Ok(product)
    }

    // --- Pass 1: Statics ---

    fn define_all_statics(&mut self) -> Result<(), CodegenError> {
        for static_def in &self.module.statics {
            self.define_static(static_def)?;
        }
        Ok(())
    }

    fn define_static(&mut self, static_def: &StaticDef) -> Result<(), CodegenError> {
        let name = &static_def.name;
        let mut mangler = Mangler::new(self.module);
        mangler.push_prefix();
        mangler.mangle_name_path(name);
        let mangled = mangler.finish();

        let linkage = Linkage::Local;

        if let Some(ref file_const) = static_def.file_constant_data {
            // File constant: embed bytes in rodata, create slice struct in data
            let data_name = format!("{mangled}_data");
            let base_path = file_const
                .base_path
                .as_ref()
                .map(|p| p.clone())
                .unwrap_or_default();
            let file_path = base_path.join(&file_const.relative_path);

            let bytes = std::fs::read(&file_path).map_err(|e| {
                CodegenError::DataSection(format!("read file constant '{}': {e}", file_path.display()))
            })?;

            // Define the raw data blob
            let data_id = self
                .cl_module
                .declare_data(&data_name, Linkage::Local, false, false)
                .map_err(|e| CodegenError::DataSection(format!("declare data: {e}")))?;
            let mut desc = DataDescription::new();
            desc.define(bytes.clone().into_boxed_slice());
            self.cl_module
                .define_data(data_id, &desc)
                .map_err(|e| CodegenError::DataSection(format!("define data: {e}")))?;

            // Create the LiteralSlice: { ptr, count }
            let ptr_size = self.target.pointer_size() as usize;
            let slice_size = ptr_size * 2;
            let mut slice_data = vec![0u8; slice_size];
            // Count is stored at offset ptr_size
            let count = bytes.len();
            slice_data[ptr_size..slice_size]
                .copy_from_slice(&(count as u64).to_le_bytes()[..ptr_size]);

            let slice_id = self
                .cl_module
                .declare_data(&mangled, linkage, true, false)
                .map_err(|e| CodegenError::DataSection(format!("declare slice: {e}")))?;
            let mut slice_desc = DataDescription::new();
            slice_desc.define(slice_data.into_boxed_slice());
            // Relocation: ptr at offset 0 → data_id
            let data_gv = self
                .cl_module
                .declare_data_in_data(data_id, &mut slice_desc);
            slice_desc.write_data_addr(0, data_gv, 0);
            self.cl_module
                .define_data(slice_id, &slice_desc)
                .map_err(|e| CodegenError::DataSection(format!("define slice: {e}")))?;
        } else {
            // Regular static: zero-initialized
            let layout = self.layouts.layout_of(&static_def.ty);
            let data_id = self
                .cl_module
                .declare_data(&mangled, linkage, static_def.is_mutable, false)
                .map_err(|e| CodegenError::DataSection(format!("declare static: {e}")))?;
            let mut desc = DataDescription::new();
            desc.define_zeroinit(layout.size as usize);
            self.cl_module
                .define_data(data_id, &desc)
                .map_err(|e| CodegenError::DataSection(format!("define static: {e}")))?;
        }

        Ok(())
    }

    // --- Pass 2: Declare Functions ---

    fn declare_all_functions(&mut self) -> Result<(), CodegenError> {
        // Declare all monomorphized function instantiations
        let insts: Vec<FunctionInstantiation> = self.mono_set.functions.iter().cloned().collect();
        for inst in &insts {
            let func_def = &self.module.functions[inst.func_id.index()];
            let mangled = self.mangle_instantiation(inst);
            let sig = self.create_signature(func_def, &inst.type_args, inst.self_type.as_ref())?;

            let linkage = if func_def.is_extern() {
                Linkage::Import
            } else {
                Linkage::Local
            };

            let func_id = self
                .cl_module
                .declare_function(&mangled, linkage, &sig)
                .map_err(|e| CodegenError::FunctionDefinition {
                    name: mangled.clone(),
                    source: e,
                })?;
            self.func_ids_by_name.insert(mangled, func_id);
        }

        // Declare runtime helpers (e.g., memcmp)
        self.declare_runtime_helpers()?;

        Ok(())
    }

    fn declare_runtime_helpers(&mut self) -> Result<(), CodegenError> {
        let ptr_ty = common::ptr_type(self.target);

        // memcmp(ptr, ptr, size) -> i32
        if !self.func_ids_by_name.contains_key("memcmp") {
            let mut sig = self.cl_module.make_signature();
            sig.params.push(AbiParam::new(ptr_ty));
            sig.params.push(AbiParam::new(ptr_ty));
            sig.params.push(AbiParam::new(ptr_ty)); // size_t
            sig.returns.push(AbiParam::new(ir::types::I32));
            sig.call_conv = self.c_call_conv();

            let func_id = self
                .cl_module
                .declare_function("memcmp", Linkage::Import, &sig)
                .map_err(|e| CodegenError::FunctionDefinition {
                    name: "memcmp".into(),
                    source: e,
                })?;
            self.func_ids_by_name.insert("memcmp".into(), func_id);
        }

        Ok(())
    }

    // --- Pass 3: Define Functions ---

    fn define_all_functions(&mut self) -> Result<(), CodegenError> {
        let insts: Vec<FunctionInstantiation> = self.mono_set.functions.iter().cloned().collect();
        for inst in insts {
            let func_def = &self.module.functions[inst.func_id.index()];

            // Skip extern functions (no body to compile)
            if func_def.is_extern() {
                continue;
            }

            // Skip functions without bodies
            let Some(_body) = &func_def.body else {
                continue;
            };

            let mangled = self.mangle_instantiation(&inst);
            ktrace!("codegen", "compiling: {}", mangled);

            let Some(&func_id) = self.func_ids_by_name.get(&mangled) else {
                return Err(CodegenError::FunctionCompilation {
                    name: mangled,
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "function not declared",
                    )),
                });
            };

            // Build substitution map
            let subst = function::build_subst(func_def, &inst.type_args);

            let sig = self.create_signature(func_def, &inst.type_args, inst.self_type.as_ref())?;

            function::compile_function(
                self,
                func_def,
                func_id,
                &sig,
                &subst,
                inst.self_type.as_ref(),
                &mangled,
            )?;
        }

        Ok(())
    }

    // --- Helpers ---

    /// Mangle a function instantiation into a linker symbol name.
    pub fn mangle_instantiation(&self, inst: &FunctionInstantiation) -> String {
        let func_def = &self.module.functions[inst.func_id.index()];
        mangle_function_with_self(
            self.module,
            func_def,
            &inst.type_args,
            inst.self_type.as_ref(),
        )
    }

    /// Create a Cranelift signature for a function instantiation.
    pub fn create_signature(
        &mut self,
        func_def: &FunctionDef,
        type_args: &[MirTy],
        self_type: Option<&MirTy>,
    ) -> Result<Signature, CodegenError> {
        let ptr_ty = common::ptr_type(self.target);
        let mut sig = self.cl_module.make_signature();

        // Build substitution
        let subst: HashMap<Entity, MirTy> = func_def
            .type_params
            .iter()
            .zip(type_args.iter())
            .map(|(tp, arg)| (tp.entity, arg.clone()))
            .collect();

        // Resolve return type
        let ret_ty = substitute_type(&func_def.ret, &subst);
        let is_main = self.is_main_function(func_def);

        // Extern functions use C calling convention
        if let Some(extern_info) = &func_def.extern_info {
            sig.call_conv = self.c_call_conv();
            for param in &func_def.params {
                let ty = substitute_type(&param.ty, &subst);
                sig.params.push(AbiParam::new(types::translate_type(&ty, self.target)));
            }
            if !matches!(ret_ty, MirTy::Unit) {
                sig.returns.push(AbiParam::new(types::translate_type(&ret_ty, self.target)));
            }
            return Ok(sig);
        }

        // sret: aggregate returns (except main) pass via hidden first pointer param
        let use_sret = !is_main && needs_sret(&ret_ty);
        if use_sret {
            sig.params
                .push(AbiParam::special(ptr_ty, ir::ArgumentPurpose::StructReturn));
        }

        // Regular parameters
        for param in &func_def.params {
            let ty = substitute_type(&param.ty, &subst);
            sig.params
                .push(AbiParam::new(types::translate_type(&ty, self.target)));
        }

        // Return type
        if is_main {
            sig.returns.push(AbiParam::new(ir::types::I64));
        } else if !use_sret && !matches!(ret_ty, MirTy::Unit | MirTy::Never) {
            sig.returns
                .push(AbiParam::new(types::translate_type(&ret_ty, self.target)));
        }

        Ok(sig)
    }

    /// Check if a function is the entry point (main).
    pub fn is_main_function(&self, func_def: &FunctionDef) -> bool {
        if let Some(entry) = self.module.entry_point {
            self.module.functions[entry.index()].entity == func_def.entity
        } else {
            // Fallback: last segment of name is "main"
            func_def.name.split('.').last() == Some("main")
        }
    }

    /// Get the C calling convention for the current target.
    pub fn c_call_conv(&self) -> CallConv {
        self.isa.default_call_conv()
    }

    /// Get or create a string literal data entry. Returns the DataId.
    pub fn get_or_create_string_data(
        &mut self,
        s: &str,
    ) -> Result<cranelift_module::DataId, CodegenError> {
        if let Some(&id) = self.string_data.get(s) {
            return Ok(id);
        }

        let name = format!(".str.{}", self.string_data.len());
        let data_id = self
            .cl_module
            .declare_data(&name, Linkage::Local, false, false)
            .map_err(|e| CodegenError::DataSection(format!("declare string: {e}")))?;

        let mut desc = DataDescription::new();
        desc.define(s.as_bytes().to_vec().into_boxed_slice());
        self.cl_module
            .define_data(data_id, &desc)
            .map_err(|e| CodegenError::DataSection(format!("define string: {e}")))?;

        self.string_data.insert(s.to_string(), data_id);
        Ok(data_id)
    }
}
