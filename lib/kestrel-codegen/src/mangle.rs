//! Name mangling for Kestrel symbols.
//!
//! Converts qualified names with optional type arguments into unique,
//! linker-safe symbol names.
//!
//! # Mangling Scheme
//!
//! The scheme is inspired by the Itanium C++ ABI:
//! - Prefix: `_K` (Kestrel)
//! - Segments: length-prefixed (e.g., `3foo` for "foo")
//! - Generic args: `I...E` (types between I and E)
//! - Primitives: single letters (i=Int, f=Float, b=Bool, s=String, u=Unit, n=Never)
//!
//! # Examples
//!
//! ```text
//! std.core.add           -> _K3std4core3add
//! Vec[Int].push          -> _K3VecIiE4push
//! identity[String]       -> _K8identityIsE
//! Pair[Int, Bool]        -> _K4PairIibE
//! ```

use kestrel_execution_graph::{Id, MirContext, MirTy, QualifiedName, Ty};

/// Mangle a qualified name with optional type arguments into a linker symbol.
pub fn mangle_name(ctx: &MirContext, name: Id<QualifiedName>, type_args: &[Id<Ty>]) -> String {
    let mut mangler = Mangler::new(ctx);
    mangler.mangle_qualified_name(name, type_args)
}

/// Name mangler that produces unique linker symbols.
pub struct Mangler<'a> {
    ctx: &'a MirContext,
    output: String,
}

impl<'a> Mangler<'a> {
    /// Create a new mangler.
    pub fn new(ctx: &'a MirContext) -> Self {
        Self {
            ctx,
            output: String::with_capacity(64),
        }
    }

    /// Mangle a qualified name with type arguments.
    pub fn mangle_qualified_name(
        &mut self,
        name: Id<QualifiedName>,
        type_args: &[Id<Ty>],
    ) -> String {
        self.output.clear();
        self.output.push_str("_K");

        let name_data = self.ctx.name(name);
        for segment in &name_data.segments {
            self.mangle_segment(segment);
        }

        if !type_args.is_empty() {
            self.output.push('I');
            for &ty in type_args {
                self.mangle_type(ty);
            }
            self.output.push('E');
        }

        self.output.clone()
    }

    /// Mangle a single name segment.
    fn mangle_segment(&mut self, segment: &str) {
        // Length-prefix the segment
        self.output.push_str(&segment.len().to_string());
        self.output.push_str(segment);
    }

    /// Mangle a type.
    fn mangle_type(&mut self, ty: Id<Ty>) {
        match self.ctx.ty(ty) {
            MirTy::I8 => self.output.push_str("i8"),
            MirTy::I16 => self.output.push_str("i16"),
            MirTy::I32 => self.output.push_str("i32"),
            MirTy::I64 => self.output.push('i'),
            MirTy::F16 => self.output.push_str("f16"),
            MirTy::F32 => self.output.push_str("f32"),
            MirTy::F64 => self.output.push('f'),
            MirTy::Bool => self.output.push('b'),
            MirTy::Unit => self.output.push('u'),
            MirTy::Never => self.output.push('n'),
            MirTy::Str => self.output.push('s'),

            MirTy::Pointer(inner) => {
                self.output.push('P');
                self.mangle_type(*inner);
            }
            MirTy::Ref(inner) => {
                self.output.push('R');
                self.mangle_type(*inner);
            }
            MirTy::RefMut(inner) => {
                self.output.push('M');
                self.mangle_type(*inner);
            }

            MirTy::Tuple(elems) => {
                self.output.push('T');
                self.output.push_str(&elems.len().to_string());
                for &elem in elems {
                    self.mangle_type(elem);
                }
            }

            MirTy::Array(elem) => {
                self.output.push('A');
                self.mangle_type(*elem);
            }

            MirTy::Named { name, type_args } => {
                let name_data = self.ctx.name(*name);
                for segment in &name_data.segments {
                    self.mangle_segment(segment);
                }
                if !type_args.is_empty() {
                    self.output.push('I');
                    for &ty in type_args {
                        self.mangle_type(ty);
                    }
                    self.output.push('E');
                }
            }

            MirTy::TypeParam(id) => {
                // Use type param name
                let param = self.ctx.type_param(*id);
                self.mangle_segment(&param.name);
            }

            MirTy::FuncThin { params, ret } => {
                self.output.push('F');
                self.output.push_str(&params.len().to_string());
                for &p in params {
                    self.mangle_type(p);
                }
                self.mangle_type(*ret);
            }

            MirTy::FuncThick { params, ret } => {
                self.output.push('C'); // Closure/thick
                self.output.push_str(&params.len().to_string());
                for &p in params {
                    self.mangle_type(p);
                }
                self.mangle_type(*ret);
            }

            MirTy::SelfType => {
                self.output.push('S');
            }

            MirTy::AssociatedTypeProjection {
                base,
                protocol,
                associated,
            } => {
                self.output.push('Q'); // Qualified/projection
                self.mangle_type(*base);
                let proto_name = self.ctx.name(*protocol);
                for segment in &proto_name.segments {
                    self.mangle_segment(segment);
                }
                self.mangle_segment(associated);
            }

            MirTy::Error => {
                self.output.push('X'); // Error type
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_execution_graph::QualifiedNameData;

    #[test]
    fn test_simple_name() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["std", "core", "add"]));
        let mangled = mangle_name(&ctx, name, &[]);
        assert_eq!(mangled, "_K3std4core3add");
    }

    #[test]
    fn test_name_with_int_type_arg() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["identity"]));
        let i64_ty = ctx.ty_i64();
        let mangled = mangle_name(&ctx, name, &[i64_ty]);
        assert_eq!(mangled, "_K8identityIiE");
    }

    #[test]
    fn test_name_with_multiple_type_args() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["Pair"]));
        let i64_ty = ctx.ty_i64();
        let bool_ty = ctx.ty_bool();
        let mangled = mangle_name(&ctx, name, &[i64_ty, bool_ty]);
        assert_eq!(mangled, "_K4PairIibE");
    }

    #[test]
    fn test_nested_generic() {
        let mut ctx = MirContext::new();
        let vec_name = ctx.intern_name(QualifiedNameData::from_parts(&["Vec"]));
        let i64_ty = ctx.ty_i64();
        let vec_int_ty = ctx.ty_named(vec_name, vec![i64_ty]);

        let name = ctx.intern_name(QualifiedNameData::from_parts(&["process"]));
        let mangled = mangle_name(&ctx, name, &[vec_int_ty]);
        assert_eq!(mangled, "_K7processI3VecIiEE");
    }

    #[test]
    fn test_pointer_type() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["deref"]));
        let i64_ty = ctx.ty_i64();
        let ptr_ty = ctx.ty_ptr(i64_ty);
        let mangled = mangle_name(&ctx, name, &[ptr_ty]);
        assert_eq!(mangled, "_K5derefIPiE");
    }

    #[test]
    fn test_tuple_type() {
        let mut ctx = MirContext::new();
        let name = ctx.intern_name(QualifiedNameData::from_parts(&["swap"]));
        let i64_ty = ctx.ty_i64();
        let bool_ty = ctx.ty_bool();
        let tuple_ty = ctx.ty_tuple(vec![i64_ty, bool_ty]);
        let mangled = mangle_name(&ctx, name, &[tuple_ty]);
        assert_eq!(mangled, "_K4swapIT2ibE");
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
        assert_eq!(mangled, "_K5applyIF1iiE");
    }
}
