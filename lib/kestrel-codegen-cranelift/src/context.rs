use std::collections::HashMap;
use std::sync::Arc;

use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{DataDescription, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use kestrel_codegen::TargetConfig;
use kestrel_hecs::Entity;
use kestrel_mir::ImmediateKind;
use kestrel_mir::mono::{MonoFunction, MonoModule};

use crate::CodegenOptions;
use crate::error::CodegenError;
use crate::func;
use crate::ty::TypeCache;

pub struct CodegenCtx<'m> {
    pub module: &'m MonoModule,
    pub options: &'m CodegenOptions,
    pub cl_module: ObjectModule,
    pub isa: Arc<dyn TargetIsa>,
    pub tc: TypeCache,
    pub ptr_ty: ir::Type,
    pub ptr_size: u64,

    pub func_ids: Vec<Option<FuncId>>,
    pub string_data: HashMap<String, cranelift_module::DataId>,
    pub static_data: HashMap<Entity, cranelift_module::DataId>,
    pub func_builder_ctx: FunctionBuilderContext,
    pub clif_outputs: Vec<(String, String)>,
}

impl<'m> CodegenCtx<'m> {
    pub fn new(
        module: &'m MonoModule,
        target: &TargetConfig,
        options: &'m CodegenOptions,
    ) -> Result<Self, CodegenError> {
        let mut flag_builder = settings::builder();
        flag_builder.set("is_pic", "true").unwrap();
        match options.opt_level {
            0 => flag_builder.set("opt_level", "none").unwrap(),
            1 => flag_builder.set("opt_level", "speed").unwrap(),
            _ => flag_builder.set("opt_level", "speed_and_size").unwrap(),
        }
        let isa_builder = cranelift_native::builder()
            .map_err(|e| CodegenError::ModuleCreation(format!("native ISA: {e}")))?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| CodegenError::ModuleCreation(format!("ISA finish: {e}")))?;

        let ptr_ty = if target.is_64bit() {
            ir::types::I64
        } else {
            ir::types::I32
        };
        let ptr_size = target.pointer_size();

        let obj_builder = ObjectBuilder::new(
            isa.clone(),
            "kestrel_module",
            cranelift_module::default_libcall_names(),
        )
        .map_err(|e| CodegenError::ModuleCreation(format!("ObjectBuilder: {e}")))?;

        let cl_module = ObjectModule::new(obj_builder);
        let tc = TypeCache::new(module, ptr_ty, ptr_size);

        Ok(Self {
            module,
            options,
            cl_module,
            isa,
            tc,
            ptr_ty,
            ptr_size,
            func_ids: vec![None; module.functions.len()],
            string_data: HashMap::new(),
            static_data: HashMap::new(),
            func_builder_ctx: FunctionBuilderContext::new(),
            clif_outputs: Vec::new(),
        })
    }

    pub fn compile_all(&mut self) -> Result<(), CodegenError> {
        self.define_all_statics()?;
        self.declare_all_functions()?;
        self.define_all_functions()?;
        Ok(())
    }

    pub fn finish(self) -> Result<crate::CompilationResult, CodegenError> {
        let product = self
            .cl_module
            .finish()
            .emit()
            .map_err(|e| CodegenError::ModuleFinish(format!("{e}")))?;

        Ok(crate::CompilationResult {
            object_bytes: product,
            clif_text: self.clif_outputs,
        })
    }

    pub fn is_main_function(&self, func: &MonoFunction) -> bool {
        // The entry point is the `@main`-marked function (propagated from
        // FunctionDef.is_main through monomorphization). Independent of name.
        func.is_main
    }

    // -- Statics --

    fn define_all_statics(&mut self) -> Result<(), CodegenError> {
        for s in self.module.statics.values() {
            self.define_static(s)?;
        }
        Ok(())
    }

    fn define_static(
        &mut self,
        s: &kestrel_mir::item::static_def::StaticDef,
    ) -> Result<(), CodegenError> {
        let repr = self.tc.repr(s.ty, &self.module.ty_arena, self.module);
        let size = repr.size().max(1) as usize;

        if let Some(fcd) = &s.file_constant_data {
            return self.define_file_constant(s, fcd, size);
        }

        // Writable even for immutable `let`s: every static lowered here is
        // populated at startup by `__kestrel_init_statics` (its `__init$` thunk
        // writes through `global_ref`), so the slot must be in writable memory
        // or the init store faults. File constants are pre-baked read-only and
        // returned above.
        let data_id = self
            .cl_module
            .declare_data(&s.name, Linkage::Local, true, false)
            .map_err(|e| CodegenError::DataSection(format!("{e}")))?;

        let mut desc = DataDescription::new();

        if let Some(init) = &s.initializer {
            match &init.kind {
                ImmediateKind::IntLiteral { bits, value } => {
                    let bytes = match bits {
                        kestrel_mir::IntBits::I8 => vec![*value as u8],
                        kestrel_mir::IntBits::I16 => (*value as i16).to_le_bytes().to_vec(),
                        kestrel_mir::IntBits::I32 => (*value as i32).to_le_bytes().to_vec(),
                        kestrel_mir::IntBits::I64 => (*value as i64).to_le_bytes().to_vec(),
                    };
                    desc.define(bytes.into_boxed_slice());
                },
                ImmediateKind::FloatLiteral { bits, value } => {
                    let bytes = match bits {
                        kestrel_mir::FloatBits::F32 | kestrel_mir::FloatBits::F16 => {
                            (*value as f32).to_le_bytes().to_vec()
                        },
                        kestrel_mir::FloatBits::F64 => value.to_le_bytes().to_vec(),
                    };
                    desc.define(bytes.into_boxed_slice());
                },
                ImmediateKind::BoolLiteral(b) => {
                    desc.define(vec![*b as u8].into_boxed_slice());
                },
                _ => {
                    desc.define_zeroinit(size);
                },
            }
        } else {
            desc.define_zeroinit(size);
        }

        self.cl_module
            .define_data(data_id, &desc)
            .map_err(|e| CodegenError::DataSection(format!("{e}")))?;

        self.static_data.insert(s.entity, data_id);
        Ok(())
    }

    fn define_file_constant(
        &mut self,
        s: &kestrel_mir::item::static_def::StaticDef,
        fcd: &kestrel_mir::item::FileConstantData,
        _size: usize,
    ) -> Result<(), CodegenError> {
        let path = if let Some(base) = &fcd.base_path {
            base.join(&fcd.relative_path)
        } else {
            std::path::PathBuf::from(&fcd.relative_path)
        };

        let file_bytes = std::fs::read(&path).map_err(|e| {
            CodegenError::DataSection(format!(
                "failed to read file constant '{}': {e}",
                path.display()
            ))
        })?;

        let data_name = format!("{}.data", s.name);
        let raw_data_id = self
            .cl_module
            .declare_data(&data_name, Linkage::Local, false, false)
            .map_err(|e| CodegenError::DataSection(format!("{e}")))?;

        let mut raw_desc = DataDescription::new();
        raw_desc.define(file_bytes.clone().into_boxed_slice());
        self.cl_module
            .define_data(raw_data_id, &raw_desc)
            .map_err(|e| CodegenError::DataSection(format!("{e}")))?;

        let elem_repr = self
            .tc
            .repr(fcd.element_ty, &self.module.ty_arena, self.module);
        let elem_size = elem_repr.size().max(1);
        let count = file_bytes.len() as u64 / elem_size;

        let slice_data_id = self
            .cl_module
            .declare_data(&s.name, Linkage::Local, false, false)
            .map_err(|e| CodegenError::DataSection(format!("{e}")))?;

        let mut slice_desc = DataDescription::new();
        let ptr_size = self.ptr_size as usize;
        let mut slice_bytes = vec![0u8; ptr_size * 2];
        let count_bytes = (count as i64).to_le_bytes();
        slice_bytes[ptr_size..ptr_size + 8.min(ptr_size)]
            .copy_from_slice(&count_bytes[..8.min(ptr_size)]);
        slice_desc.define(slice_bytes.into_boxed_slice());

        let raw_gv = self
            .cl_module
            .declare_data_in_data(raw_data_id, &mut slice_desc);
        slice_desc.write_data_addr(0, raw_gv, 0);

        self.cl_module
            .define_data(slice_data_id, &slice_desc)
            .map_err(|e| CodegenError::DataSection(format!("{e}")))?;

        self.static_data.insert(s.entity, slice_data_id);
        Ok(())
    }

    // -- Functions --

    fn declare_all_functions(&mut self) -> Result<(), CodegenError> {
        // Extern imports with the same symbol must share a FuncId.
        // Local functions always get unique IDs — even if mangled names
        // collide, each mono function is a distinct compilation unit.
        let mut extern_declared: HashMap<String, FuncId> = HashMap::new();
        for (i, func) in self.module.functions.iter().enumerate() {
            if let Some(ext) = &func.extern_info
                && let Some(&existing_id) = extern_declared.get(&ext.symbol_name) {
                    self.func_ids[i] = Some(existing_id);
                    continue;
                }
            let func_id = self.declare_function(func, i)?;
            if let Some(ext) = &func.extern_info {
                extern_declared.insert(ext.symbol_name.clone(), func_id);
            }
            self.func_ids[i] = Some(func_id);
        }
        Ok(())
    }

    fn declare_function(
        &mut self,
        func: &MonoFunction,
        idx: usize,
    ) -> Result<FuncId, CodegenError> {
        let is_main = self.is_main_function(func);
        let call_conv = self.isa.default_call_conv();

        let (sig, linkage, name) = if let Some(ext) = &func.extern_info {
            let sig = crate::abi::build_extern_signature(
                func,
                &mut self.tc,
                &self.module.ty_arena,
                self.module,
                call_conv,
            );
            (sig, Linkage::Import, ext.symbol_name.clone())
        } else if is_main {
            let sig = crate::abi::build_signature(
                func,
                true,
                &mut self.tc,
                &self.module.ty_arena,
                self.module,
                call_conv,
            );
            (sig, Linkage::Export, "main".to_string())
        } else {
            let sig = crate::abi::build_signature(
                func,
                false,
                &mut self.tc,
                &self.module.ty_arena,
                self.module,
                call_conv,
            );
            // Append mono index to guarantee unique names within the
            // Cranelift module. The linker sees mangled_name; the suffix
            // only prevents Cranelift's declare_function dedup.
            let unique_name = format!("{}.mono{}", func.name, idx);
            (sig, Linkage::Local, unique_name)
        };

        let func_id = self
            .cl_module
            .declare_function(&name, linkage, &sig)
            .map_err(|e| CodegenError::FunctionDefinition {
                name: name.clone(),
                source: e,
            })?;

        Ok(func_id)
    }

    fn define_all_functions(&mut self) -> Result<(), CodegenError> {
        let mut errors: Vec<(String, String)> = Vec::new();
        for i in 0..self.module.functions.len() {
            let func = &self.module.functions[i];
            if func.body.is_some() {
                let func_id = self.func_ids[i].expect("function must be declared");
                let func_name = func.name.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    func::compile_function(self, i, func_id)
                }));
                match result {
                    Ok(Ok(())) => {},
                    Ok(Err(e)) => {
                        errors.push((func_name, format!("{e}")));
                        self.define_trap_stub(func_id);
                    },
                    Err(panic) => {
                        let msg = if let Some(s) = panic.downcast_ref::<String>() {
                            s.clone()
                        } else if let Some(s) = panic.downcast_ref::<&str>() {
                            s.to_string()
                        } else {
                            "unknown panic".to_string()
                        };
                        errors.push((func_name, format!("panic: {msg}")));
                        self.define_trap_stub(func_id);
                    },
                }
            }
        }
        if !errors.is_empty() {
            let body_count = self
                .module
                .functions
                .iter()
                .filter(|f| f.body.is_some())
                .count();
            eprintln!(
                "warning: {} of {} functions failed to compile (skipped):",
                errors.len(),
                body_count
            );
            let mut by_cat: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for (_, err) in &errors {
                let cat = if err.contains("non-dominating") {
                    "dominance"
                } else if err.contains("invalid pointer width") {
                    "ptr-width"
                } else if err.contains("type set") {
                    "type-set"
                } else if err.contains("ICE:") {
                    "ICE"
                } else if err.contains("has type") {
                    "type-mismatch"
                } else if err.contains("result 0 has type") {
                    "return-type"
                } else {
                    "other"
                };
                *by_cat.entry(cat.to_string()).or_default() += 1;
            }
            for (cat, count) in by_cat.iter() {
                eprintln!("  {count:>5} {cat}");
            }
            if std::env::var("KESTREL_VERBOSE_CODEGEN").is_ok() {
                for (name, err) in &errors {
                    eprintln!("    {name}: {err}");
                }
            }
        }
        Ok(())
    }

    // -- String data --

    /// Define a minimal trap function for a failed compilation so the
    /// object module doesn't panic on an undeclared-but-local symbol.
    fn define_trap_stub(&mut self, func_id: FuncId) {
        let sig = self
            .cl_module
            .declarations()
            .get_function_decl(func_id)
            .signature
            .clone();
        let mut cl_func = ir::Function::with_name_signature(ir::UserFuncName::user(0, 0), sig);
        let mut fbc = std::mem::take(&mut self.func_builder_ctx);
        let mut builder = cranelift_frontend::FunctionBuilder::new(&mut cl_func, &mut fbc);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);
        builder
            .ins()
            .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
        builder.finalize();
        self.func_builder_ctx = fbc;
        let mut comp_ctx = cranelift_codegen::Context::for_function(cl_func);
        if comp_ctx
            .compile(self.isa.as_ref(), &mut Default::default())
            .is_ok()
        {
            let _ = self.cl_module.define_function(func_id, &mut comp_ctx);
        }
    }

    pub fn get_or_create_string_data(
        &mut self,
        _cl_func: &mut ir::Function,
        s: &str,
    ) -> Result<cranelift_module::DataId, CodegenError> {
        if let Some(&id) = self.string_data.get(s) {
            return Ok(id);
        }

        let name = format!(".str.{}", self.string_data.len());
        let data_id = self
            .cl_module
            .declare_data(&name, Linkage::Local, false, false)
            .map_err(|e| CodegenError::DataSection(format!("{e}")))?;

        let mut desc = DataDescription::new();
        desc.define(s.as_bytes().to_vec().into_boxed_slice());
        self.cl_module
            .define_data(data_id, &desc)
            .map_err(|e| CodegenError::DataSection(format!("{e}")))?;

        self.string_data.insert(s.to_string(), data_id);
        Ok(data_id)
    }
}
