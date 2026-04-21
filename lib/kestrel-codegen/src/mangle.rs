//! Name mangling for Kestrel symbols — v0 scheme.
//!
//! Converts qualified names with optional type arguments into unique,
//! linker-safe symbol names following the v0 mangling specification.
//!
//! # Mangling Scheme
//!
//! - Prefix: `_K0`
//! - Ident: `{byte_len}_{utf8_bytes}`
//! - Single segment path: `ident`
//! - Nested path: `N` ident+ `E`
//! - Receiver: `r` (Ref), `m` (RefMut), `c` (consuming)
//! - Signature: `Z` param* `E`
//! - Labeled param: `L` ident type
//! - Unlabeled param: bare type
//! - Instantiation: `I` type+ `E`
//!
//! # Type Encoding
//!
//! - Integers: `i` + byte_width (i1=I8, i2=I16, i4=I32, i8=I64)
//! - Floats: `f` + byte_width (f2=F16, f4=F32, f8=F64)
//! - Bool: `b`, Str: `s`, Unit: `v`, Never: `n`
//! - Pointer: `P` type, Ref: `R` type, RefMut: `M` type
//! - Tuple: `T` type* `E`
//! - FuncThin: `F` count `_` params ret `E`
//! - FuncThick: `C` count `_` params ret `E`
//! - Named: path optional `I` type+ `E`
//! - SelfType: `S`
//! - AssociatedTypeProjection: `Q` type ident
//! - Error: `X`

use kestrel_execution_graph::{
    Function, Id, MirContext, MirTy, QualifiedName, ReceiverConvention, Ty,
};

/// Mangle a qualified name (for statics and non-function symbols).
///
/// Format: `_K0` + path + optional instantiation
pub fn mangle_name(ctx: &MirContext, name: Id<QualifiedName>, type_args: &[Id<Ty>]) -> String {
    let mut m = Mangler::new(ctx);
    m.push_str("_K0");
    m.mangle_path(name);
    m.mangle_instantiation(type_args);
    m.finish()
}

/// Mangle a function with its full signature.
pub fn mangle_function(ctx: &MirContext, func_id: Id<Function>, type_args: &[Id<Ty>]) -> String {
    mangle_function_with_self(ctx, func_id, type_args, None)
}

/// Mangle a function with its full signature including optional Self type substitution.
pub fn mangle_function_with_self(
    ctx: &MirContext,
    func_id: Id<Function>,
    type_args: &[Id<Ty>],
    self_type: Option<Id<Ty>>,
) -> String {
    let func_def = &ctx.functions[func_id];

    let mut m = Mangler::new(ctx);
    if let Some(st) = self_type {
        m.self_type = Some(st);
    }

    m.push_str("_K0");
    m.mangle_path(func_def.name);
    m.mangle_receiver(func_def.receiver_convention);
    m.mangle_signature(func_def, ctx);
    m.mangle_instantiation(type_args);

    // Self type disambiguation for protocol extension methods.
    // When a method is compiled with a concrete Self type (e.g., Array conforming
    // to Iterator), the self_type distinguishes it from the same method compiled
    // for a different conformance.
    if let Some(st) = self_type {
        m.push_str("S_");
        m.mangle_type(st);
    }

    m.finish()
}

/// Name mangler that produces unique linker symbols.
pub struct Mangler<'a> {
    ctx: &'a MirContext,
    output: String,
    self_type: Option<Id<Ty>>,
}

impl<'a> Mangler<'a> {
    /// Create a new mangler.
    pub fn new(ctx: &'a MirContext) -> Self {
        Self {
            ctx,
            output: String::with_capacity(64),
            self_type: None,
        }
    }

    fn push_str(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn push(&mut self, c: char) {
        self.output.push(c);
    }

    fn finish(self) -> String {
        self.output
    }

    /// Mangle a path: single ident or `N` ident+ `E`.
    ///
    /// `$` suffixes are stripped from the LAST segment only, since the signature
    /// section (`Z...E`) provides disambiguation for the final function/method name.
    /// Interior segments (e.g., parent function names in closure paths) keep their
    /// `$` suffix to maintain uniqueness.
    fn mangle_path(&mut self, name: Id<QualifiedName>) {
        let name_data = self.ctx.name(name);
        let segments = &name_data.segments;

        if segments.len() == 1 {
            self.mangle_ident(&strip_dollar(&segments[0]));
        } else {
            self.push('N');
            for (i, segment) in segments.iter().enumerate() {
                if i == segments.len() - 1 {
                    self.mangle_ident(&strip_dollar(segment));
                } else {
                    self.mangle_ident(segment);
                }
            }
            self.push('E');
        }
    }

    /// Mangle an identifier: `{byte_len}_{utf8_bytes}`.
    fn mangle_ident(&mut self, s: &str) {
        self.push_str(&s.len().to_string());
        self.push('_');
        self.push_str(s);
    }

    /// Mangle receiver convention marker.
    fn mangle_receiver(&mut self, recv: Option<ReceiverConvention>) {
        match recv {
            Some(ReceiverConvention::Ref) => self.push('r'),
            Some(ReceiverConvention::RefMut) => self.push('m'),
            Some(ReceiverConvention::Consuming) => self.push('c'),
            None => {},
        }
    }

    /// Mangle function signature: `Z` param* `E`.
    ///
    /// Excludes the self parameter. Self is detected by receiver_convention being set,
    /// or by the first param being named "self" (for initializers which have no
    /// receiver convention but still have a self param).
    fn mangle_signature(
        &mut self,
        func_def: &kestrel_execution_graph::FunctionDef,
        ctx: &MirContext,
    ) {
        self.push('Z');

        // Skip self param: either via receiver_convention or by name for initializers
        let skip = if func_def.receiver_convention.is_some() {
            1
        } else if func_def
            .params
            .first()
            .is_some_and(|&p| ctx.params[p].name == "self")
        {
            1
        } else {
            0
        };

        for &param_id in func_def.params.iter().skip(skip) {
            let param = &ctx.params[param_id];
            if let Some(ref label) = param.external_label {
                self.push('L');
                self.mangle_ident(label);
            }
            self.mangle_type(param.ty);
        }

        self.push('E');
    }

    /// Mangle instantiation: `I` type+ `E` if non-empty.
    fn mangle_instantiation(&mut self, type_args: &[Id<Ty>]) {
        if !type_args.is_empty() {
            self.push('I');
            for &ty in type_args {
                self.mangle_type(ty);
            }
            self.push('E');
        }
    }

    /// Mangle a type.
    fn mangle_type(&mut self, ty: Id<Ty>) {
        match self.ctx.ty(ty) {
            MirTy::I8 => self.push_str("i1"),
            MirTy::I16 => self.push_str("i2"),
            MirTy::I32 => self.push_str("i4"),
            MirTy::I64 => self.push_str("i8"),
            MirTy::F16 => self.push_str("f2"),
            MirTy::F32 => self.push_str("f4"),
            MirTy::F64 => self.push_str("f8"),
            MirTy::Bool => self.push('b'),
            MirTy::Unit => self.push('v'),
            MirTy::Never => self.push('n'),
            MirTy::Str => self.push('s'),

            MirTy::Pointer(inner) => {
                self.push('P');
                self.mangle_type(*inner);
            },
            MirTy::Ref(inner) => {
                self.push('R');
                self.mangle_type(*inner);
            },
            MirTy::RefMut(inner) => {
                self.push('M');
                self.mangle_type(*inner);
            },

            MirTy::Tuple(elems) => {
                let elems = elems.clone();
                self.push('T');
                for elem in &elems {
                    self.mangle_type(*elem);
                }
                self.push('E');
            },

            MirTy::Named { name, type_args } => {
                let name = *name;
                let type_args = type_args.clone();
                self.mangle_path(name);
                if !type_args.is_empty() {
                    self.push('I');
                    for ty in &type_args {
                        self.mangle_type(*ty);
                    }
                    self.push('E');
                }
            },

            MirTy::TypeParam(id) => {
                // TypeParam can appear in function signatures before monomorphization
                // (e.g., closures inheriting parent type params). Encode as a named ident.
                let param = self.ctx.type_param(*id);
                self.mangle_ident(&param.name);
            },

            MirTy::FuncThin { params, ret } => {
                let params = params.clone();
                let ret = *ret;
                self.push('F');
                self.push_str(&params.len().to_string());
                self.push('_');
                for p in &params {
                    self.mangle_type(*p);
                }
                self.mangle_type(ret);
                self.push('E');
            },

            MirTy::FuncThick { params, ret } => {
                let params = params.clone();
                let ret = *ret;
                self.push('C');
                self.push_str(&params.len().to_string());
                self.push('_');
                for p in &params {
                    self.mangle_type(*p);
                }
                self.mangle_type(ret);
                self.push('E');
            },

            MirTy::SelfType => {
                // If we have a concrete self_type substitution, use it
                if let Some(st) = self.self_type {
                    self.mangle_type(st);
                } else {
                    self.push('S');
                }
            },

            MirTy::AssociatedTypeProjection {
                base, associated, ..
            } => {
                let base = *base;
                let associated = associated.clone();
                self.push('Q');
                self.mangle_type(base);
                self.mangle_ident(&associated);
            },

            MirTy::Error => {
                self.push('X');
            },
        }
    }
}

/// Strip `$` suffix from a segment for path encoding.
///
/// `$` suffixes are used internally for overload disambiguation in QualifiedName
/// but should not appear in mangled output — the signature section handles disambiguation.
fn strip_dollar(segment: &str) -> String {
    segment.split('$').next().unwrap().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_execution_graph::{QualifiedNameData, ReceiverConvention};

    #[test]
    fn test_simple_name() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["std", "core", "add"]));
        let mangled = mangle_name(&ctx, name, &[]);
        assert_eq!(mangled, "_K0N3_std4_core3_addE");
    }

    #[test]
    fn test_single_segment_name() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["foo"]));
        let mangled = mangle_name(&ctx, name, &[]);
        assert_eq!(mangled, "_K03_foo");
    }

    #[test]
    fn test_name_with_int_type_arg() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["identity"]));
        let i64_ty = ctx.ty_i64();
        let mangled = mangle_name(&ctx, name, &[i64_ty]);
        assert_eq!(mangled, "_K08_identityIi8E");
    }

    #[test]
    fn test_name_with_multiple_type_args() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["Pair"]));
        let i64_ty = ctx.ty_i64();
        let bool_ty = ctx.ty_bool();
        let mangled = mangle_name(&ctx, name, &[i64_ty, bool_ty]);
        assert_eq!(mangled, "_K04_PairIi8bE");
    }

    #[test]
    fn test_nested_generic() {
        let mut ctx = MirContext::new();
        let vec_name = ctx.intern_name(QualifiedNameData::from_parts(&["Vec"]));
        let i64_ty = ctx.ty_i64();
        let vec_int_ty = ctx.ty_named(vec_name, vec![i64_ty]);

        let name = ctx.intern_name(QualifiedNameData::from_parts(&["process"]));
        let mangled = mangle_name(&ctx, name, &[vec_int_ty]);
        assert_eq!(mangled, "_K07_processI3_VecIi8EE");
    }

    #[test]
    fn test_pointer_type() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["deref"]));
        let i64_ty = ctx.ty_i64();
        let ptr_ty = ctx.ty_ptr(i64_ty);
        let mangled = mangle_name(&ctx, name, &[ptr_ty]);
        assert_eq!(mangled, "_K05_derefIPi8E");
    }

    #[test]
    fn test_tuple_type() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["swap"]));
        let i64_ty = ctx.ty_i64();
        let bool_ty = ctx.ty_bool();
        let tuple_ty = ctx.ty_tuple(vec![i64_ty, bool_ty]);
        let mangled = mangle_name(&ctx, name, &[tuple_ty]);
        assert_eq!(mangled, "_K04_swapITi8bEE");
    }

    #[test]
    fn test_function_type() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["apply"]));
        let i64_ty = ctx.ty_i64();
        let func_ty = ctx.intern_type(MirTy::FuncThin {
            params: vec![i64_ty],
            ret: i64_ty,
        });
        let mangled = mangle_name(&ctx, name, &[func_ty]);
        assert_eq!(mangled, "_K05_applyIF1_i8i8EE");
    }

    #[test]
    fn test_unit_type() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["noop"]));
        let unit_ty = ctx.ty_unit();
        let mangled = mangle_name(&ctx, name, &[unit_ty]);
        assert_eq!(mangled, "_K04_noopIvE");
    }

    #[test]
    fn test_integer_byte_widths() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["f"]));
        let i8_ty = ctx.intern_type(MirTy::I8);
        let i16_ty = ctx.intern_type(MirTy::I16);
        let i32_ty = ctx.intern_type(MirTy::I32);
        let i64_ty = ctx.ty_i64();
        let mangled = mangle_name(&ctx, name, &[i8_ty, i16_ty, i32_ty, i64_ty]);
        assert_eq!(mangled, "_K01_fIi1i2i4i8E");
    }

    #[test]
    fn test_float_byte_widths() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["f"]));
        let f16_ty = ctx.intern_type(MirTy::F16);
        let f32_ty = ctx.intern_type(MirTy::F32);
        let f64_ty = ctx.intern_type(MirTy::F64);
        let mangled = mangle_name(&ctx, name, &[f16_ty, f32_ty, f64_ty]);
        assert_eq!(mangled, "_K01_fIf2f4f8E");
    }

    #[test]
    fn test_function_with_signature() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["Main", "add"]));
        let i64_ty = ctx.ty_i64();
        let unit_ty = ctx.ty_unit();

        let func_id = ctx.add_function(name, unit_ty).id();

        // Add two unlabeled params
        ctx.function_builder(func_id).param("a", i64_ty);
        ctx.function_builder(func_id).param("b", i64_ty);

        let mangled = mangle_function(&ctx, func_id, &[]);
        assert_eq!(mangled, "_K0N4_Main3_addEZi8i8E");
    }

    #[test]
    fn test_function_with_labeled_params() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["Main", "greet"]));
        let str_ty = ctx.intern_type(MirTy::Str);
        let unit_ty = ctx.ty_unit();

        let func_id = ctx.add_function(name, unit_ty).id();
        ctx.function_builder(func_id)
            .param_with_label("name", str_ty, Some("name".to_string()));

        let mangled = mangle_function(&ctx, func_id, &[]);
        assert_eq!(mangled, "_K0N4_Main5_greetEZL4_namesE");
    }

    #[test]
    fn test_function_with_receiver() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["Main", "Point", "get:x"]));
        let i64_ty = ctx.ty_i64();
        let ref_ty = ctx.ty_ref(i64_ty); // placeholder self type

        let func_id = ctx.add_function(name, i64_ty).id();
        ctx.function_builder(func_id).param("self", ref_ty);
        ctx.function_mut(func_id).receiver_convention = Some(ReceiverConvention::Ref);

        let mangled = mangle_function(&ctx, func_id, &[]);
        assert_eq!(mangled, "_K0N4_Main5_Point5_get:xErZE");
    }

    #[test]
    fn test_function_with_refmut_receiver() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["Main", "Point", "set:x"]));
        let i64_ty = ctx.ty_i64();
        let ref_mut_ty = ctx.ty_ref_mut(i64_ty); // placeholder self type
        let unit_ty = ctx.ty_unit();

        let func_id = ctx.add_function(name, unit_ty).id();
        ctx.function_builder(func_id).param("self", ref_mut_ty);
        ctx.function_mut(func_id).receiver_convention = Some(ReceiverConvention::RefMut);
        ctx.function_builder(func_id).param("newValue", i64_ty);

        let mangled = mangle_function(&ctx, func_id, &[]);
        assert_eq!(mangled, "_K0N4_Main5_Point5_set:xEmZi8E");
    }

    #[test]
    fn test_static_mangling() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["Main", "counter"]));
        let mangled = mangle_name(&ctx, name, &[]);
        assert_eq!(mangled, "_K0N4_Main7_counterE");
    }

    #[test]
    fn test_closure_naming() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&[
            "Main", "foo", "closure", "0",
        ]));
        let unit_ty = ctx.ty_unit();

        let func_id = ctx.add_function(name, unit_ty).id();
        // Closure has an env param (not counted in signature)

        let mangled = mangle_function(&ctx, func_id, &[]);
        assert_eq!(mangled, "_K0N4_Main3_foo7_closure1_0EZE");
    }

    #[test]
    fn test_dollar_stripped_from_path() {
        let mut ctx = MirContext::new();
        // Internally overloaded name with $label suffix
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["Main", "init$x$y"]));
        let mangled = mangle_name(&ctx, name, &[]);
        assert_eq!(mangled, "_K0N4_Main4_initE");
    }

    #[test]
    fn test_thick_function_type() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["f"]));
        let i64_ty = ctx.ty_i64();
        let thick_ty = ctx.intern_type(MirTy::FuncThick {
            params: vec![i64_ty, i64_ty],
            ret: i64_ty,
        });
        let mangled = mangle_name(&ctx, name, &[thick_ty]);
        assert_eq!(mangled, "_K01_fIC2_i8i8i8EE");
    }

    #[test]
    fn test_empty_tuple() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["f"]));
        let tuple_ty = ctx.ty_tuple(vec![]);
        let mangled = mangle_name(&ctx, name, &[tuple_ty]);
        assert_eq!(mangled, "_K01_fITEE");
    }

    #[test]
    fn test_named_type_in_type_arg() {
        let mut ctx = MirContext::new();
        let array_name = ctx.intern_name(QualifiedNameData::from_parts(&["std", "Array"]));
        let i64_ty = ctx.ty_i64();
        let array_ty = ctx.ty_named(array_name, vec![i64_ty]);

        let name = ctx.intern_name(QualifiedNameData::from_parts(&["Main", "process"]));
        let mangled = mangle_name(&ctx, name, &[array_ty]);
        assert_eq!(mangled, "_K0N4_Main7_processEIN3_std5_ArrayEIi8EE");
    }

    #[test]
    fn test_deinit_with_consuming_receiver() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&[
            "Main", "Resource", "deinit",
        ]));
        let unit_ty = ctx.ty_unit();
        let i64_ty = ctx.ty_i64(); // placeholder self type

        let func_id = ctx.add_function(name, unit_ty).id();
        let ref_mut_ty = ctx.ty_ref_mut(i64_ty);
        ctx.function_builder(func_id).param("self", ref_mut_ty);
        ctx.function_mut(func_id).receiver_convention = Some(ReceiverConvention::Consuming);

        let mangled = mangle_function(&ctx, func_id, &[]);
        assert_eq!(mangled, "_K0N4_Main8_Resource6_deinitEcZE");
    }

    #[test]
    fn test_mixed_labeled_unlabeled_params() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["Main", "f"]));
        let i8_ty = ctx.intern_type(MirTy::I8);
        let unit_ty = ctx.ty_unit();

        let func_id = ctx.add_function(name, unit_ty).id();
        // (_ a: Int8, x: Int8)
        ctx.function_builder(func_id)
            .param_with_label("a", i8_ty, None);
        ctx.function_builder(func_id)
            .param_with_label("x", i8_ty, Some("x".to_string()));

        let mangled = mangle_function(&ctx, func_id, &[]);
        assert_eq!(mangled, "_K0N4_Main1_fEZi1L1_xi1E");
    }

    #[test]
    fn test_init_with_labeled_params() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["Main", "Point", "init"]));
        let i64_ty = ctx.ty_i64();
        let unit_ty = ctx.ty_unit();

        let func_id = ctx.add_function(name, unit_ty).id();
        // init has self param but no receiver convention
        let ref_mut_ty = ctx.ty_ref_mut(i64_ty);
        ctx.function_builder(func_id).param("self", ref_mut_ty);
        // receiver_convention stays None for initializers

        ctx.function_builder(func_id)
            .param_with_label("x", i64_ty, Some("x".to_string()));
        ctx.function_builder(func_id)
            .param_with_label("y", i64_ty, Some("y".to_string()));

        let mangled = mangle_function(&ctx, func_id, &[]);
        // Init: no receiver marker, self excluded from sig by name detection
        assert_eq!(mangled, "_K0N4_Main5_Point4_initEZL1_xi8L1_yi8E");
    }
}
