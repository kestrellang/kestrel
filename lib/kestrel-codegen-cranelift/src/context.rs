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
use kestrel_codegen::{
    LayoutCache, Mangler, TargetConfig, mangle_function_with_self, substitute_type_with_self,
};
use kestrel_hecs::Entity;
use kestrel_mir::{
    FunctionDef, MirModule, MirTy, StaticDef,
};
use std::collections::HashMap;
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
    /// Captured CLIF text per function, populated when `options.emit_clif` is set.
    pub clif_outputs: Vec<(String, String)>,
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
        flag_builder.set("is_pic", "true").unwrap();

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

        let obj_builder = ObjectBuilder::new(
            isa.clone(),
            "kestrel_module",
            cranelift_module::default_libcall_names(),
        )
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
            clif_outputs: Vec::new(),
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
            // File constant: embed bytes in rodata, create slice struct in data.
            // The file is read here (codegen time) — MIR lowering attaches only
            // the path via `FileConstantData`. If the file moves between lower
            // and codegen the read fails and aborts this static's emission;
            // that's the intended contract since the bytes are baked into the
            // object file and there's no other opportunity to embed them.
            let data_name = format!("{mangled}_data");
            let base_path = file_const
                .base_path.clone()
                .unwrap_or_default();
            let file_path = base_path.join(&file_const.relative_path);

            let bytes = std::fs::read(&file_path).map_err(|e| {
                CodegenError::DataSection(format!(
                    "read file constant '{}': {e}",
                    file_path.display()
                ))
            })?;

            // Define the raw data blob. Align to the element type's natural
            // alignment so ld doesn't warn about unaligned pointer loads into
            // the table (e.g. Int32 reads from LiteralSlice[Int32]).
            let elem_layout = self.layouts.layout_of(&file_const.element_ty);
            let elem_size = elem_layout.size.max(1);
            let elem_align = elem_layout.align.max(1);
            let data_id = self
                .cl_module
                .declare_data(&data_name, Linkage::Local, false, false)
                .map_err(|e| CodegenError::DataSection(format!("declare data: {e}")))?;
            let mut desc = DataDescription::new();
            desc.define(bytes.clone().into_boxed_slice());
            desc.set_align(elem_align);
            self.cl_module
                .define_data(data_id, &desc)
                .map_err(|e| CodegenError::DataSection(format!("define data: {e}")))?;

            // Create the LiteralSlice: { ptr, count }
            let ptr_size = self.target.pointer_size() as usize;
            let slice_size = ptr_size * 2;
            let mut slice_data = vec![0u8; slice_size];
            // Count is stored at offset ptr_size
            if bytes.len() % elem_size as usize != 0 {
                return Err(CodegenError::DataSection(format!(
                    "file constant '{}' has {} bytes, not a multiple of element size {}",
                    file_path.display(),
                    bytes.len(),
                    elem_size
                )));
            }
            let count = bytes.len() / elem_size as usize;
            slice_data[ptr_size..slice_size]
                .copy_from_slice(&(count as u64).to_le_bytes()[..ptr_size]);

            let slice_id = self
                .cl_module
                .declare_data(&mangled, linkage, true, false)
                .map_err(|e| CodegenError::DataSection(format!("declare slice: {e}")))?;
            let mut slice_desc = DataDescription::new();
            slice_desc.define(slice_data.into_boxed_slice());
            slice_desc.set_align(ptr_size as u64);
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
            desc.set_align(layout.align.max(1));
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

            let is_main = self.is_main_function(func_def);
            if emit_action(func_def, is_main) == EmitAction::Skip {
                continue;
            }

            let mangled = self.mangle_instantiation(inst);
            let sig = self.create_signature(func_def, &inst.type_args, inst.self_type.as_ref())?;

            let (symbol_name, linkage) = if is_main {
                // Main entry point: export as "main" — Cranelift adds the platform
                // underscore prefix (e.g. _main on macOS) automatically
                ("main".to_string(), Linkage::Export)
            } else if func_def.is_extern() {
                // Extern functions: import by their C symbol name
                let sym = func_def
                    .extern_info
                    .as_ref()
                    .map(|e| e.symbol_name.clone())
                    .unwrap_or(mangled.clone());
                (sym, Linkage::Import)
            } else {
                (mangled.clone(), Linkage::Local)
            };

            let func_id = self
                .cl_module
                .declare_function(&symbol_name, linkage, &sig)
                .map_err(|e| CodegenError::FunctionDefinition {
                    name: symbol_name.clone(),
                    source: e,
                })?;
            // Store under both the mangled name and the linker symbol name, so
            // later lookups by either key resolve to the same FuncId.
            // `symbol_name` differs from `mangled` only for main (`"main"`) and
            // extern functions (their C symbol); for everything else the two
            // are equal and the second insert is skipped.
            self.func_ids_by_name.insert(mangled.clone(), func_id);
            if symbol_name != mangled {
                self.func_ids_by_name.insert(symbol_name, func_id);
            }
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

            // Only functions in the `Full` emit path have bodies to compile.
            let is_main = self.is_main_function(func_def);
            if emit_action(func_def, is_main) != EmitAction::Full {
                continue;
            }

            let mangled = self.mangle_instantiation(&inst);
            let Some(&func_id) = self.func_ids_by_name.get(&mangled) else {
                return Err(CodegenError::FunctionCompilation {
                    name: mangled,
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "function not declared",
                    )),
                });
            };

            // Skip phantom instantiations where type_args don't cover all type_params.
            // These arise when call sites don't fully propagate struct type_args;
            // the correctly-resolved versions exist elsewhere in the mono_set.
            if inst.type_args.len() < func_def.type_params.len() {
                continue;
            }

            // Build substitution map, including associated type resolutions
            let mut subst = function::build_subst(func_def, &inst.type_args);
            function::resolve_assoc_type_substs(
                self.module,
                func_def,
                &mut subst,
                inst.self_type.as_ref(),
            );

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

        // Build substitution. Use the same augmentation path that
        // `compile_function` uses so signature ABI matches body codegen —
        // without `resolve_assoc_type_substs`, conformance-introduced free
        // TypeParams (e.g. `extend C: Proto[T_ext]` → `Output = Param(T_ext)`)
        // wouldn't be bound, and the signature would treat the param as
        // by-reference while the body / callee compiles it as a concrete
        // scalar. Mismatch → calls pass the wrong ABI shape.
        let mut subst: HashMap<Entity, MirTy> = func_def
            .type_params
            .iter()
            .zip(type_args.iter())
            .map(|(tp, arg)| (tp.entity, arg.clone()))
            .collect();
        function::resolve_assoc_type_substs(self.module, func_def, &mut subst, self_type);

        // Resolve return type
        let ret_ty = substitute_type_with_self(&func_def.ret, &subst, self_type, self.module);
        let is_main = self.is_main_function(func_def);

        // Extern functions use C calling convention. Translate params and
        // return via `translate_type_with_layout` so small Named wrappers
        // (Int32, UInt32, Bool, etc.) flatten to their scalar ABI type —
        // `translate_type` alone would hand C a pointer, which is never what
        // an extern declaration actually wants.
        if func_def.is_extern() {
            sig.call_conv = self.c_call_conv();
            for param in &func_def.params {
                let ty = substitute_type_with_self(&param.ty, &subst, self_type, self.module);
                sig.params
                    .push(AbiParam::new(types::translate_type_with_layout(
                        &ty,
                        self.target,
                        &mut self.layouts,
                    )));
            }
            if !ret_ty.is_unit() {
                sig.returns
                    .push(AbiParam::new(types::translate_type_with_layout(
                        &ret_ty,
                        self.target,
                        &mut self.layouts,
                    )));
            }
            return Ok(sig);
        }

        // sret: aggregate returns (except main) pass via hidden first pointer param
        let use_sret = !is_main && needs_sret(&ret_ty, &mut self.layouts);
        if use_sret {
            sig.params
                .push(AbiParam::special(ptr_ty, ir::ArgumentPurpose::StructReturn));
        }

        // Regular parameters. `mutating` (InOut) params are passed as pointers
        // regardless of value type so the callee can write back to caller storage.
        for param in &func_def.params {
            let ty = substitute_type_with_self(&param.ty, &subst, self_type, self.module);
            let cl_ty = if matches!(param.mode, kestrel_mir::ParamMode::InOut) {
                ptr_ty
            } else {
                types::translate_type(&ty, self.target)
            };
            sig.params.push(AbiParam::new(cl_ty));
        }

        // Return type
        if is_main {
            sig.returns.push(AbiParam::new(ir::types::I64));
        } else if !(use_sret || ret_ty.is_unit() || matches!(ret_ty, MirTy::Never)) {
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
            func_def.name.split('.').next_back() == Some("main")
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

/// What to do with a given function instantiation during the declare/define passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmitAction {
    /// Declare and define — the function has a body (or is `main`).
    Full,
    /// Declare only — extern function with no body to compile.
    DeclareOnly,
    /// Skip — bodyless non-extern function (lang builtin, abstract method).
    /// Declaring one would cause a "declared but not defined" linker error.
    Skip,
}

/// Decide the emit action for a function instantiation.
///
/// Shared by the declare and define passes so their filter logic can't drift.
fn emit_action(func_def: &FunctionDef, is_main: bool) -> EmitAction {
    if func_def.is_extern() {
        EmitAction::DeclareOnly
    } else if func_def.body.is_some() || is_main {
        EmitAction::Full
    } else {
        EmitAction::Skip
    }
}
