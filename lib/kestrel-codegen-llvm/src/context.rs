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
use kestrel_mir::item::function::InlineMode;
use kestrel_mir::mono::{MonoFunction, MonoModule};
use kestrel_mir::{FloatBits, ImmediateKind, IntBits};

use crate::CodegenOptions;
use crate::abi;
use crate::error::CodegenError;
use crate::func;
use crate::ty::{ScalarTy, TypeCache};

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
    /// Emit the entry point as a C `int main(int, char**)` wrapper around the
    /// Kestrel `main`. WebAssembly checks call signatures, so Emscripten's
    /// `main(argc, argv)` call traps on the argument-less Kestrel entry.
    pub c_main_wrapper: bool,
    /// Emit LLVM bitcode instead of an object. Emscripten's own (newer) LLVM
    /// then generates the wasm, so the object conventions (e.g. wasm64
    /// table indices) match its libraries rather than this compiler's LLVM.
    pub emit_bitcode: bool,

    pub func_ids: Vec<Option<FunctionValue<'ctx>>>,
    /// The target's C struct-passing rules (`kestrel_codegen::c_abi`).
    pub c_abi: kestrel_codegen::c_abi::CAbi,
    /// Per mono function crossing the C boundary, its C-ABI symbol: the
    /// imported C function of an `@extern`, the wrapper of an `@export`.
    /// C function values (`MonoCFunctionRef`) take these.
    pub c_symbols: HashMap<usize, FunctionValue<'ctx>>,
    /// Call bridges for calls through C function values, per `FuncC` type.
    pub c_call_bridges: HashMap<kestrel_mir::TyId, FunctionValue<'ctx>>,
    pub string_data: HashMap<String, GlobalValue<'ctx>>,
    pub static_data: HashMap<Entity, GlobalValue<'ctx>>,
    /// Existential function tables, indexed like `MonoModule::vtables`.
    pub vtable_data: Vec<GlobalValue<'ctx>>,
    pub ir_outputs: Vec<(String, String)>,
}

impl<'ctx> CodegenCtx<'ctx> {
    pub fn new(
        cx: &'ctx Context,
        module: &'ctx MonoModule,
        target: &TargetConfig,
        options: &'ctx CodegenOptions,
    ) -> Result<Self, CodegenError> {
        if target.is_wasm() {
            Target::initialize_webassembly(&InitializationConfig::default());
        } else {
            Target::initialize_native(&InitializationConfig::default())
                .map_err(|e| CodegenError::ModuleCreation(format!("native target init: {e}")))?;
        }

        let triple = TargetTriple::create(&target.triple.to_string());
        let target_obj = Target::from_triple(&triple)
            .map_err(|e| CodegenError::ModuleCreation(format!("target from triple: {e}")))?;

        // Host CPU/features matches the Cranelift backend's native ISA; a
        // cross target (e.g. WebAssembly) gets the generic CPU.
        let (cpu, features) = if target.is_host() {
            (
                TargetMachine::get_host_cpu_name().to_string(),
                TargetMachine::get_host_cpu_features().to_string(),
            )
        } else {
            ("generic".to_string(), String::new())
        };
        // WebAssembly has no position-independent code outside dynamic
        // linking; Emscripten links static objects.
        let reloc = if target.is_wasm() { RelocMode::Static } else { RelocMode::PIC };

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
                reloc,
                CodeModel::Default,
            )
            .ok_or_else(|| CodegenError::ModuleCreation("create_target_machine failed".into()))?;

        let llmod = cx.create_module("kestrel_module");
        llmod.set_triple(&triple);
        llmod.set_data_layout(&machine.get_target_data().get_data_layout());

        // Pointer-width scalars are the LLVM `ptr`. `ScalarTy::Ptr::bytes()` is
        // hardcoded to 8, so a non-64-bit target would mis-size pointer fields.
        let ptr_size = target.pointer_size();
        debug_assert_eq!(
            ptr_size, 8,
            "ScalarTy::Ptr currently assumes 8-byte pointers"
        );
        let tc = TypeCache::new(module, ScalarTy::Ptr, ptr_size);

        Ok(Self {
            cx,
            module,
            options,
            llmod,
            machine,
            tc,
            ptr_size,
            c_main_wrapper: target.is_wasm(),
            emit_bitcode: target.is_wasm(),
            func_ids: vec![None; module.functions.len()],
            c_abi: target.c_abi(),
            c_symbols: HashMap::new(),
            c_call_bridges: HashMap::new(),
            string_data: HashMap::new(),
            static_data: HashMap::new(),
            vtable_data: Vec::new(),
            ir_outputs: Vec::new(),
        })
    }

    pub fn compile_all(&mut self) -> Result<(), CodegenError> {
        self.define_all_statics()?;
        self.declare_all_functions()?;
        self.define_all_vtables();
        self.define_all_functions()?;
        if self.c_main_wrapper {
            self.define_c_main()?;
        }
        Ok(())
    }

    /// `int main(int argc, char **argv)` calling the Kestrel entry point and
    /// truncating its `Int64` exit code (see `c_main_wrapper`).
    fn define_c_main(&mut self) -> Result<(), CodegenError> {
        let Some(entry) = self
            .module
            .functions
            .iter()
            .position(|f| self.is_main_function(f))
            .and_then(|i| self.func_ids[i])
        else {
            return Ok(()); // a library build has no entry point
        };
        if entry.count_params() != 0 {
            return Err(CodegenError::ModuleCreation(
                "C main wrapper: the entry point takes parameters".into(),
            ));
        }
        let i32t = self.cx.i32_type();
        let ptr = self.cx.ptr_type(inkwell::AddressSpace::default());
        let main = self.llmod.add_function(
            "main",
            i32t.fn_type(&[i32t.into(), ptr.into()], false),
            Some(Linkage::External),
        );
        let builder = self.cx.create_builder();
        builder.position_at_end(self.cx.append_basic_block(main, "entry"));
        let call = builder
            .build_call(entry, &[], "code")
            .map_err(|e| CodegenError::ModuleCreation(e.to_string()))?;
        let code = match call.try_as_basic_value().basic() {
            Some(v) if v.is_int_value() => builder
                .build_int_truncate_or_bit_cast(v.into_int_value(), i32t, "exit")
                .map_err(|e| CodegenError::ModuleCreation(e.to_string()))?,
            _ => i32t.const_zero(),
        };
        builder
            .build_return(Some(&code))
            .map_err(|e| CodegenError::ModuleCreation(e.to_string()))?;
        Ok(())
    }

    pub fn finish(self) -> Result<crate::CompilationResult, CodegenError> {
        // Best-effort whole-module verify (per-function verify already ran during
        // definition; this catches inter-function/declaration issues).
        if let Err(e) = self.llmod.verify()
            && kestrel_debug::is_enabled("llvm-verify")
        {
            kestrel_debug::ktrace!(
                "llvm-verify",
                " LLVM module verification failed:\n{}",
                e.to_string()
            );
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

        // Final (post-optimization) module IR, for `kestrel dump llvm`.
        let module_ir = self
            .options
            .emit_ir
            .then(|| self.llmod.print_to_string().to_string());

        let buffer = if self.emit_bitcode {
            self.llmod.write_bitcode_to_memory()
        } else {
            self.machine
                .write_to_memory_buffer(&self.llmod, FileType::Object)
                .map_err(|e| CodegenError::ModuleFinish(e.to_string()))?
        };

        // inkwell's `as_slice` includes LLVM's trailing NUL terminator. ELF and
        // Mach-O readers ignore the extra byte; the wasm reader rejects it.
        let bytes = buffer.as_slice();
        let object = bytes.strip_suffix(&[0]).unwrap_or(bytes);

        Ok(crate::CompilationResult {
            object_bytes: object.to_vec(),
            ir_text: self.ir_outputs,
            module_ir,
        })
    }

    pub fn is_main_function(&self, func: &MonoFunction) -> bool {
        // The entry point is the function flagged `is_main` — the MIR-synthesized
        // `@main` wrapper (`__kestrel_main`) carries the flag through
        // monomorphization (FunctionDef.is_main). A name-based check is wrong
        // here: the Exitable refactor demotes the user `@main` (which keeps its
        // `*.main` name) to an ordinary function, so matching on the name would
        // export the demoted function instead of the wrapper. Independent of
        // name; mirrors the Cranelift backend.
        func.is_main
    }

    // -- Statics --

    /// Each `MonoVTable` as a constant `[N x ptr]` (`kestrel_mir::item::vtable`
    /// has the layout; integer words are `inttoptr` constants). All are
    /// declared before any is initialized: tables point at one another.
    fn define_all_vtables(&mut self) {
        let ptr_ty = self.cx.ptr_type(inkwell::AddressSpace::default());
        for table in &self.module.vtables {
            let ty = ptr_ty.array_type(table.entries.len().max(1) as u32);
            let global = self.llmod.add_global(ty, None, &table.name);
            global.set_constant(true);
            global.set_linkage(Linkage::Internal);
            self.vtable_data.push(global);
        }
        for (i, table) in self.module.vtables.iter().enumerate() {
            let int = |v: u64| {
                crate::mem::ptr_int_type(self.cx, self.ptr_size)
                    .const_int(v, false)
                    .const_to_pointer(ptr_ty)
            };
            let mut words: Vec<inkwell::values::PointerValue<'ctx>> = table
                .entries
                .iter()
                .map(|entry| match entry {
                    kestrel_mir::mono::MonoVTableEntry::Func(f) => self.func_ids[f.index()]
                        .map(|fv| fv.as_global_value().as_pointer_value())
                        .unwrap_or_else(|| ptr_ty.const_null()),
                    kestrel_mir::mono::MonoVTableEntry::Size(ty) => {
                        int(self.tc.repr(*ty, &self.module.ty_arena, self.module).size())
                    },
                    kestrel_mir::mono::MonoVTableEntry::Align(ty) => int(self
                        .tc
                        .repr(*ty, &self.module.ty_arena, self.module)
                        .align()),
                    kestrel_mir::mono::MonoVTableEntry::Table(t) => {
                        self.vtable_data[t.index()].as_pointer_value()
                    },
                    kestrel_mir::mono::MonoVTableEntry::Null => ptr_ty.const_null(),
                })
                .collect();
            if words.is_empty() {
                words.push(ptr_ty.const_null());
            }
            self.vtable_data[i].set_initializer(&ptr_ty.const_array(&words));
        }
    }

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
        let global = self.llmod.add_global(init.get_type(), None, &s.name);
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
            CodegenError::DataSection(format!(
                "failed to read file constant '{}': {e}",
                path.display()
            ))
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
        let elem_repr = self
            .tc
            .repr(fcd.element_ty, &self.module.ty_arena, self.module);
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
        // Imports sharing a symbol share one C declaration; each mono extern
        // still gets its own Kestrel-ABI thunk around it.
        let mut c_imports: HashMap<String, FunctionValue<'ctx>> = HashMap::new();
        for i in 0..self.module.functions.len() {
            let fn_value = self.declare_function(i)?;
            self.func_ids[i] = Some(fn_value);
            let module: &'ctx MonoModule = self.module;
            let func = &module.functions[i];
            let Some(ext) = &func.extern_info else {
                continue;
            };
            let decl = kestrel_codegen::c_abi::CDecl::of_function(func);
            let cty = crate::c_bridge::c_fn_type(self, &decl);
            if ext.export {
                let wrapper =
                    self.llmod
                        .add_function(&ext.symbol_name, cty.fn_type, Some(Linkage::External));
                cty.apply_to_fn(wrapper);
                crate::c_bridge::define_export_wrapper(self, &decl, wrapper, fn_value, &cty);
                self.c_symbols.insert(i, wrapper);
                continue;
            }
            let c_fn = *c_imports.entry(ext.symbol_name.clone()).or_insert_with(|| {
                let f =
                    self.llmod
                        .add_function(&ext.symbol_name, cty.fn_type, Some(Linkage::External));
                cty.apply_to_fn(f);
                f
            });
            let target = crate::c_bridge::CTarget::Symbol(c_fn);
            crate::c_bridge::define_import_thunk(self, &decl, fn_value, target, &cty);
            self.add_inline_attribute(fn_value, InlineMode::Always);
            self.c_symbols.insert(i, c_fn);
        }
        Ok(())
    }

    fn declare_function(&mut self, idx: usize) -> Result<FunctionValue<'ctx>, CodegenError> {
        let module: &'ctx MonoModule = self.module;
        let func = &module.functions[idx];
        let is_main = self.is_main_function(func);
        let cx = self.cx;

        // Every function, an `@extern` thunk included, has the Kestrel ABI.
        let sig = abi::build_signature(func, &mut self.tc, &module.ty_arena, module, cx);
        let (linkage, name) = if is_main && self.c_main_wrapper {
            (Linkage::Internal, "kestrel.main".to_string())
        } else if is_main {
            (Linkage::External, "main".to_string())
        } else {
            // Mono suffix guarantees a unique LLVM symbol within the module.
            (Linkage::Internal, format!("{}.mono{}", func.name, idx))
        };

        let fn_value = self.llmod.add_function(&name, sig, Some(linkage));
        if let Some(mode) = func.inline {
            self.add_inline_attribute(fn_value, mode);
        }
        Ok(fn_value)
    }

    /// `@inline` → `inlinehint` / `alwaysinline` / `noinline`.
    pub(crate) fn add_inline_attribute(&self, fn_value: FunctionValue<'ctx>, mode: InlineMode) {
        let name = match mode {
            InlineMode::Hint => "inlinehint",
            InlineMode::Always => "alwaysinline",
            InlineMode::Never => "noinline",
        };
        let kind = inkwell::attributes::Attribute::get_named_enum_kind_id(name);
        let attr = self.cx.create_enum_attribute(kind, 0);
        fn_value.add_attribute(inkwell::attributes::AttributeLoc::Function, attr);
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

            // This is one of the compiler's panic-catching hosts: a panicking
            // function becomes a trap stub and codegen carries on in the same
            // process. Nothing below this point pushes to a stack that a later
            // function reads, so there is nothing to unwind-guard today — if
            // you add such state, bracket it with `kestrel_hecs::guard::OnDrop`,
            // not a manual pop (F22).
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                func::compile_function(self, i, fn_value)
            }));
            match result {
                Ok(Ok(())) => {
                    let print_verify = kestrel_debug::is_enabled("llvm-verify");
                    if !fn_value.verify(print_verify) {
                        if print_verify {
                            // The LLVM verifier message above is terse; also dump
                            // the whole broken function so the offending instruction
                            // has context (the function is about to be trap-stubbed).
                            use inkwell::values::AnyValue;
                            kestrel_debug::ktrace!(
                                "llvm-verify",
                                "=== broken fn {func_name} ===\n{}",
                                fn_value.print_to_string().to_string()
                            );
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

        // A function that fails codegen/verification is a miscompile: emitting a
        // trap stub and continuing produces a clean-looking build (exit 0, binary
        // written) that SIGILLs at runtime when the function is called. Abort the
        // build instead — no object is emitted, no binary is linked — and report
        // every failure so the user can act on all of them at once (#151/#149).
        if !errors.is_empty() {
            return Err(CodegenError::CompilationFailed(errors));
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
        if let Some(intr) = Intrinsic::find("llvm.trap")
            && let Some(f) = intr.get_declaration(&self.llmod, &[])
        {
            let _ = builder.build_call(f, &[], "");
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
