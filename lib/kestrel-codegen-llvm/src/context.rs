//! Codegen context and orchestration. Faithful port of the Cranelift backend's
//! `context.rs`: three-phase pipeline (statics -> declare funcs -> define
//! funcs), per-function fault tolerance (failed bodies become `llvm.trap`
//! stubs), and object emission via the LLVM `TargetMachine`.

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;

use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::intrinsics::Intrinsic;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::values::{FunctionValue, GlobalValue};

use kestrel_codegen::TargetConfig;
use kestrel_hecs::Entity;
use kestrel_mir::mono::{MonoFunction, MonoModule};
use kestrel_mir::{FloatBits, ImmediateKind, IntBits};

use crate::CodegenOptions;
use crate::abi;
use crate::error::CodegenError;
use crate::func;
use crate::ty::TypeCache;

pub struct CodegenCtx<'ctx> {
    /// The LLVM context (lifetime root). `&'ctx` because the MIR module is
    /// coerced to the same (shorter) lifetime — see `lib::compile`.
    pub cx: &'ctx Context,
    /// The monomorphized MIR module (the codegen input).
    pub module: &'ctx MonoModule,
    pub options: &'ctx CodegenOptions,
    pub llmod: Module<'ctx>,
    pub machine: TargetMachine,
    pub tc: TypeCache,
    pub ptr_size: u64,

    pub func_ids: Vec<Option<FunctionValue<'ctx>>>,
    pub string_data: HashMap<String, GlobalValue<'ctx>>,
    pub static_data: HashMap<Entity, GlobalValue<'ctx>>,
    pub ir_outputs: Vec<(String, String)>,
}

impl<'ctx> CodegenCtx<'ctx> {
    pub fn new(
        cx: &'ctx Context,
        module: &'ctx MonoModule,
        target: &TargetConfig,
        options: &'ctx CodegenOptions,
    ) -> Result<Self, CodegenError> {
        Target::initialize_native(&InitializationConfig::default())
            .map_err(|e| CodegenError::ModuleCreation(format!("native target init: {e}")))?;

        let triple = TargetTriple::create(&target.triple.to_string());
        let target_obj = Target::from_triple(&triple)
            .map_err(|e| CodegenError::ModuleCreation(format!("target from triple: {e}")))?;

        // Host CPU/features matches the Cranelift backend's native ISA. (Kestrel
        // codegen is host-targeted; cross-compiling would want generic here.)
        let cpu = TargetMachine::get_host_cpu_name().to_string();
        let features = TargetMachine::get_host_cpu_features().to_string();

        let opt = match options.opt_level {
            0 => OptimizationLevel::None,
            1 => OptimizationLevel::Default,
            _ => OptimizationLevel::Aggressive,
        };

        let machine = target_obj
            .create_target_machine(
                &triple,
                &cpu,
                &features,
                opt,
                RelocMode::PIC,
                CodeModel::Default,
            )
            .ok_or_else(|| CodegenError::ModuleCreation("create_target_machine failed".into()))?;

        let llmod = cx.create_module("kestrel_module");
        llmod.set_triple(&triple);
        llmod.set_data_layout(&machine.get_target_data().get_data_layout());

        let ptr_size = target.pointer_size();
        let tc = TypeCache::new(module, ptr_size);

        Ok(Self {
            cx,
            module,
            options,
            llmod,
            machine,
            tc,
            ptr_size,
            func_ids: vec![None; module.functions.len()],
            string_data: HashMap::new(),
            static_data: HashMap::new(),
            ir_outputs: Vec::new(),
        })
    }

    pub fn compile_all(&mut self) -> Result<(), CodegenError> {
        self.define_all_statics()?;
        self.declare_all_functions()?;
        self.define_all_functions()?;
        Ok(())
    }

    pub fn finish(self) -> Result<crate::CompilationResult, CodegenError> {
        // Best-effort whole-module verify (per-function verify already ran during
        // definition; this catches inter-function/declaration issues).
        if let Err(e) = self.llmod.verify() {
            if std::env::var("KESTREL_VERBOSE_CODEGEN").is_ok() {
                eprintln!("warning: LLVM module verification failed:\n{}", e.to_string());
            }
        }

        // Run the standard LLVM middle-end optimization pipeline at the requested
        // level (the TargetMachine OptimizationLevel only governs codegen/isel —
        // the IR transforms below are what inline + mem2reg + instcombine + gvn,
        // closing the un-inlined-primitive gap and folding the Option-A int<->ptr
        // pairs). opt_level 0 skips it entirely. Best-effort: a pass-pipeline
        // failure logs a warning and emits the unoptimized module rather than
        // failing the whole build.
        if self.options.opt_level > 0 {
            let pipeline = match self.options.opt_level {
                1 => "default<O1>",
                2 => "default<O2>",
                _ => "default<O3>",
            };
            if let Err(e) =
                self.llmod
                    .run_passes(pipeline, &self.machine, PassBuilderOptions::create())
            {
                eprintln!(
                    "warning: LLVM optimization pipeline '{pipeline}' failed: {}",
                    e.to_string()
                );
            }
        }

        // Debug: dump the final (post-optimization) module IR for inspection.
        if let Ok(path) = std::env::var("KESTREL_DUMP_LLVM_IR") {
            let _ = std::fs::write(&path, self.llmod.print_to_string().to_string());
        }

        let buffer = self
            .machine
            .write_to_memory_buffer(&self.llmod, FileType::Object)
            .map_err(|e| CodegenError::ModuleFinish(e.to_string()))?;

        Ok(crate::CompilationResult {
            object_bytes: buffer.as_slice().to_vec(),
            ir_text: self.ir_outputs,
        })
    }

    pub fn is_main_function(&self, func: &MonoFunction) -> bool {
        let name = self.module.resolve_name(func.source);
        name == "main" || name.ends_with(".main")
    }

    // -- Statics --

    fn define_all_statics(&mut self) -> Result<(), CodegenError> {
        let keys: Vec<_> = self.module.statics.keys().copied().collect();
        for k in keys {
            // `statics` is keyed; re-fetch to avoid holding a borrow across the call.
            let s = self.module.statics.get(&k).unwrap();
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
            return self.define_file_constant(s, fcd);
        }

        let bytes = build_init_bytes(size, s.initializer.as_ref().map(|i| &i.kind));
        let init = self.cx.const_string(&bytes, false);
        let global = self
            .llmod
            .add_global(init.get_type(), None, &s.name);
        global.set_initializer(&init);
        // Writable: every static is populated at startup by __kestrel_init_statics.
        global.set_constant(false);
        global.set_linkage(Linkage::Internal);

        self.static_data.insert(s.entity, global);
        Ok(())
    }

    fn define_file_constant(
        &mut self,
        s: &kestrel_mir::item::static_def::StaticDef,
        fcd: &kestrel_mir::item::FileConstantData,
    ) -> Result<(), CodegenError> {
        let path = if let Some(base) = &fcd.base_path {
            base.join(&fcd.relative_path)
        } else {
            std::path::PathBuf::from(&fcd.relative_path)
        };

        let file_bytes = std::fs::read(&path).map_err(|e| {
            CodegenError::DataSection(format!("failed to read file constant '{}': {e}", path.display()))
        })?;

        // Raw read-only blob.
        let raw_init = self.cx.const_string(&file_bytes, false);
        let raw_global =
            self.llmod
                .add_global(raw_init.get_type(), None, &format!("{}.data", s.name));
        raw_global.set_initializer(&raw_init);
        raw_global.set_constant(true);
        raw_global.set_linkage(Linkage::Internal);

        // Slice header { ptr, len } pointing at the blob.
        let elem_repr = self.tc.repr(fcd.element_ty, &self.module.ty_arena, self.module);
        let elem_size = elem_repr.size().max(1);
        let count = file_bytes.len() as u64 / elem_size;

        let ptr_field = raw_global.as_pointer_value();
        let len_field = crate::mem::ptr_int_type(self.cx, self.ptr_size).const_int(count, false);
        let slice_val = self
            .cx
            .const_struct(&[ptr_field.into(), len_field.into()], false);
        let slice_global = self.llmod.add_global(slice_val.get_type(), None, &s.name);
        slice_global.set_initializer(&slice_val);
        slice_global.set_constant(true);
        slice_global.set_linkage(Linkage::Internal);

        self.static_data.insert(s.entity, slice_global);
        Ok(())
    }

    // -- Functions --

    fn declare_all_functions(&mut self) -> Result<(), CodegenError> {
        // Extern imports sharing a symbol must share one FunctionValue. Local
        // functions always get a unique (mono-suffixed) name.
        let mut extern_declared: HashMap<String, FunctionValue<'ctx>> = HashMap::new();
        for i in 0..self.module.functions.len() {
            let ext_symbol = self.module.functions[i]
                .extern_info
                .as_ref()
                .map(|e| e.symbol_name.clone());
            if let Some(sym) = &ext_symbol {
                if let Some(&existing) = extern_declared.get(sym) {
                    self.func_ids[i] = Some(existing);
                    continue;
                }
            }
            let fn_value = self.declare_function(i)?;
            if let Some(sym) = ext_symbol {
                extern_declared.insert(sym, fn_value);
            }
            self.func_ids[i] = Some(fn_value);
        }
        Ok(())
    }

    fn declare_function(&mut self, idx: usize) -> Result<FunctionValue<'ctx>, CodegenError> {
        let module: &'ctx MonoModule = self.module;
        let func = &module.functions[idx];
        let is_main = self.is_main_function(func);
        let cx = self.cx;

        let (fn_type, linkage, name) = if let Some(ext) = &func.extern_info {
            let sig = abi::build_extern_signature(func, &mut self.tc, &module.ty_arena, module, cx);
            (sig, Linkage::External, ext.symbol_name.clone())
        } else if is_main {
            let sig = abi::build_signature(func, true, &mut self.tc, &module.ty_arena, module, cx);
            (sig, Linkage::External, "main".to_string())
        } else {
            let sig = abi::build_signature(func, false, &mut self.tc, &module.ty_arena, module, cx);
            // Mono suffix guarantees a unique LLVM symbol within the module.
            (sig, Linkage::Internal, format!("{}.mono{}", func.name, idx))
        };

        Ok(self.llmod.add_function(&name, fn_type, Some(linkage)))
    }

    fn define_all_functions(&mut self) -> Result<(), CodegenError> {
        let mut errors: Vec<(String, String)> = Vec::new();
        let n = self.module.functions.len();
        for i in 0..n {
            if self.module.functions[i].body.is_none() {
                continue;
            }
            let fn_value = self.func_ids[i].expect("function must be declared");
            let func_name = self.module.functions[i].name.clone();

            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                func::compile_function(self, i, fn_value)
            }));
            match result {
                Ok(Ok(())) => {
                    let print_verify = std::env::var("KESTREL_VERBOSE_CODEGEN").is_ok();
                    if !fn_value.verify(print_verify) {
                        if print_verify {
                            // The LLVM verifier message above is terse; also dump
                            // the whole broken function so the offending instruction
                            // has context (the function is about to be trap-stubbed).
                            use inkwell::values::AnyValue;
                            eprintln!("=== broken fn {func_name} ===\n{}", fn_value.print_to_string().to_string());
                        }
                        errors.push((func_name, "LLVM function verification failed".into()));
                        self.reset_to_trap_stub(fn_value);
                    }
                },
                Ok(Err(e)) => {
                    errors.push((func_name, format!("{e}")));
                    self.reset_to_trap_stub(fn_value);
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
                    self.reset_to_trap_stub(fn_value);
                },
            }
        }

        if !errors.is_empty() {
            let body_count = self.module.functions.iter().filter(|f| f.body.is_some()).count();
            eprintln!(
                "warning: {} of {} functions failed to compile (skipped):",
                errors.len(),
                body_count
            );
            let mut by_cat: HashMap<String, usize> = HashMap::new();
            for (_, err) in &errors {
                let cat = if err.contains("verification") {
                    "verify"
                } else if err.contains("unsupported") {
                    "unsupported"
                } else if err.contains("panic") {
                    "panic"
                } else if err.contains("ICE") {
                    "ICE"
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

    /// Replace a function's body with a single `llvm.trap` + `unreachable`, so a
    /// failed compilation still links (mirrors Cranelift's trap stub).
    fn reset_to_trap_stub(&mut self, fn_value: FunctionValue<'ctx>) {
        for bb in fn_value.get_basic_blocks() {
            unsafe {
                let _ = bb.delete();
            }
        }
        let entry = self.cx.append_basic_block(fn_value, "entry");
        let builder = self.cx.create_builder();
        builder.position_at_end(entry);
        if let Some(intr) = Intrinsic::find("llvm.trap") {
            if let Some(f) = intr.get_declaration(&self.llmod, &[]) {
                let _ = builder.build_call(f, &[], "");
            }
        }
        let _ = builder.build_unreachable();
    }

    // -- String data --

    pub fn get_or_create_string_data(
        &mut self,
        s: &str,
    ) -> Result<GlobalValue<'ctx>, CodegenError> {
        if let Some(&g) = self.string_data.get(s) {
            return Ok(g);
        }
        let name = format!(".str.{}", self.string_data.len());
        let init = self.cx.const_string(s.as_bytes(), false);
        let global = self.llmod.add_global(init.get_type(), None, &name);
        global.set_initializer(&init);
        global.set_constant(true);
        global.set_linkage(Linkage::Internal);
        self.string_data.insert(s.to_string(), global);
        Ok(global)
    }
}

/// Build exactly `size` little-endian bytes for a static initializer (zero-pad
/// or truncate). Mirrors the Cranelift backend's literal baking.
fn build_init_bytes(size: usize, init: Option<&ImmediateKind>) -> Vec<u8> {
    let mut bytes = match init {
        Some(ImmediateKind::IntLiteral { bits, value }) => match bits {
            IntBits::I8 => vec![*value as u8],
            IntBits::I16 => (*value as i16).to_le_bytes().to_vec(),
            IntBits::I32 => (*value as i32).to_le_bytes().to_vec(),
            IntBits::I64 => (*value as i64).to_le_bytes().to_vec(),
        },
        Some(ImmediateKind::FloatLiteral { bits, value }) => match bits {
            FloatBits::F16 | FloatBits::F32 => (*value as f32).to_le_bytes().to_vec(),
            FloatBits::F64 => value.to_le_bytes().to_vec(),
        },
        Some(ImmediateKind::BoolLiteral(b)) => vec![*b as u8],
        _ => vec![0u8; size],
    };
    bytes.resize(size, 0);
    bytes
}
