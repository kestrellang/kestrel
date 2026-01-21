//! Strict MIR type representation for assertions.

use kestrel_execution_graph::{Id, MirContext, Ty};

/// Strict MIR type for assertions. Must match exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirTy {
    // Primitives
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Bool,
    Unit,
    Str,
    Never,

    // Compound types
    /// Named type: `"Main.Point"`
    Named(String),
    /// Named type with generic arguments: `"Main.Option"`, `[MirTy::I64]`
    NamedGeneric(String, Vec<MirTy>),
    /// Immutable reference: `&T`
    Ref(Box<MirTy>),
    /// Mutable reference: `&var T`
    RefMut(Box<MirTy>),
    /// Pointer: `*T`
    Pointer(Box<MirTy>),
    /// Tuple: `(T, U, ...)`
    Tuple(Vec<MirTy>),

    // Function types
    /// Thin function: `func(T) -> U`
    Func {
        params: Vec<MirTy>,
        ret: Box<MirTy>,
    },
    /// Thick function (closure): `func(T) -> U` with captures
    FuncThick {
        params: Vec<MirTy>,
        ret: Box<MirTy>,
    },

    // Generics
    /// Type parameter: `T`
    TypeParam(String),
    /// Associated type projection: `T.Element` where `T: Protocol`
    AssociatedType {
        base: Box<MirTy>,
        protocol: String,
        associated: String,
    },
    /// Self type (in protocol method signatures)
    SelfType,

    /// Error type (lowering failed)
    Error,
}

impl MirTy {
    // === Convenience constructors ===

    /// Create a named type.
    pub fn named(name: &str) -> Self {
        MirTy::Named(name.to_string())
    }

    /// Create a named type with generic arguments.
    pub fn named_generic(name: &str, args: Vec<MirTy>) -> Self {
        MirTy::NamedGeneric(name.to_string(), args)
    }

    /// Alias for `named_generic` - create a generic type with type arguments.
    pub fn generic(name: &str, args: Vec<MirTy>) -> Self {
        MirTy::NamedGeneric(name.to_string(), args)
    }

    /// Create an immutable reference type.
    pub fn ref_(inner: MirTy) -> Self {
        MirTy::Ref(Box::new(inner))
    }

    /// Create a mutable reference type.
    pub fn ref_mut(inner: MirTy) -> Self {
        MirTy::RefMut(Box::new(inner))
    }

    /// Create a pointer type.
    pub fn ptr(inner: MirTy) -> Self {
        MirTy::Pointer(Box::new(inner))
    }

    /// Create a tuple type.
    pub fn tuple(elems: Vec<MirTy>) -> Self {
        MirTy::Tuple(elems)
    }

    /// Create a thin function type.
    pub fn func(params: Vec<MirTy>, ret: MirTy) -> Self {
        MirTy::Func {
            params,
            ret: Box::new(ret),
        }
    }

    /// Create a thick function type (closure with captures).
    pub fn func_thick(params: Vec<MirTy>, ret: MirTy) -> Self {
        MirTy::FuncThick {
            params,
            ret: Box::new(ret),
        }
    }

    /// Create a type parameter.
    pub fn type_param(name: &str) -> Self {
        MirTy::TypeParam(name.to_string())
    }

    /// Create an associated type projection.
    pub fn associated_type(base: MirTy, protocol: &str, associated: &str) -> Self {
        MirTy::AssociatedType {
            base: Box::new(base),
            protocol: protocol.to_string(),
            associated: associated.to_string(),
        }
    }

    // === Matching against actual MIR types ===

    /// Check if this type pattern matches an actual MIR type.
    pub(crate) fn matches(&self, actual_id: Id<Ty>, ctx: &MirContext) -> bool {
        let actual = ctx.ty(actual_id);
        self.matches_ty(actual, ctx)
    }

    fn matches_ty(&self, actual: &kestrel_execution_graph::MirTy, ctx: &MirContext) -> bool {
        use kestrel_execution_graph::MirTy as ActualTy;

        match (self, actual) {
            // Primitives
            (MirTy::I8, ActualTy::I8) => true,
            (MirTy::I16, ActualTy::I16) => true,
            (MirTy::I32, ActualTy::I32) => true,
            (MirTy::I64, ActualTy::I64) => true,
            (MirTy::F32, ActualTy::F32) => true,
            (MirTy::F64, ActualTy::F64) => true,
            (MirTy::Bool, ActualTy::Bool) => true,
            (MirTy::Unit, ActualTy::Unit) => true,
            (MirTy::Str, ActualTy::Str) => true,
            (MirTy::Never, ActualTy::Never) => true,
            (MirTy::SelfType, ActualTy::SelfType) => true,
            (MirTy::Error, ActualTy::Error) => true,

            // Named types (no generics)
            (MirTy::Named(expected_name), ActualTy::Named { name, type_args })
                if type_args.is_empty() =>
            {
                let actual_name = ctx.name(*name);
                actual_name.to_string() == *expected_name
            },

            // Named types with generics
            (
                MirTy::NamedGeneric(expected_name, expected_args),
                ActualTy::Named { name, type_args },
            ) => {
                let actual_name = ctx.name(*name);
                if actual_name.to_string() != *expected_name {
                    return false;
                }
                if expected_args.len() != type_args.len() {
                    return false;
                }
                expected_args
                    .iter()
                    .zip(type_args.iter())
                    .all(|(e, a)| e.matches(*a, ctx))
            },

            // References
            (MirTy::Ref(expected_inner), ActualTy::Ref(actual_inner)) => {
                expected_inner.matches(*actual_inner, ctx)
            },
            (MirTy::RefMut(expected_inner), ActualTy::RefMut(actual_inner)) => {
                expected_inner.matches(*actual_inner, ctx)
            },
            (MirTy::Pointer(expected_inner), ActualTy::Pointer(actual_inner)) => {
                expected_inner.matches(*actual_inner, ctx)
            },

            // Tuples
            (MirTy::Tuple(expected_elems), ActualTy::Tuple(actual_elems)) => {
                if expected_elems.len() != actual_elems.len() {
                    return false;
                }
                expected_elems
                    .iter()
                    .zip(actual_elems.iter())
                    .all(|(e, a)| e.matches(*a, ctx))
            },

            // Function types (thin)
            (
                MirTy::Func {
                    params: expected_params,
                    ret: expected_ret,
                },
                ActualTy::FuncThin {
                    params: actual_params,
                    ret: actual_ret,
                },
            ) => {
                if expected_params.len() != actual_params.len() {
                    return false;
                }
                for (e, a) in expected_params.iter().zip(actual_params.iter()) {
                    if !e.matches(*a, ctx) {
                        return false;
                    }
                }
                expected_ret.matches(*actual_ret, ctx)
            },

            // Function types (thick/escaping)
            (
                MirTy::FuncThick {
                    params: expected_params,
                    ret: expected_ret,
                },
                ActualTy::FuncThick {
                    params: actual_params,
                    ret: actual_ret,
                },
            ) => {
                if expected_params.len() != actual_params.len() {
                    return false;
                }
                for (e, a) in expected_params.iter().zip(actual_params.iter()) {
                    if !e.matches(*a, ctx) {
                        return false;
                    }
                }
                expected_ret.matches(*actual_ret, ctx)
            },

            // Type parameters
            (MirTy::TypeParam(expected_name), ActualTy::TypeParam(actual_id)) => {
                let actual_param = ctx.type_param(*actual_id);
                actual_param.name == *expected_name
            },

            // Associated type projections
            (
                MirTy::AssociatedType {
                    base: expected_base,
                    protocol: expected_protocol,
                    associated: expected_associated,
                },
                ActualTy::AssociatedTypeProjection {
                    base: actual_base,
                    protocol: actual_protocol,
                    associated: actual_associated,
                },
            ) => {
                let actual_protocol_name = ctx.name(*actual_protocol);
                expected_base.matches(*actual_base, ctx)
                    && actual_protocol_name.to_string() == *expected_protocol
                    && actual_associated == expected_associated
            },

            // No match
            _ => false,
        }
    }

    /// Format this type for display in error messages.
    pub(crate) fn display(&self) -> String {
        match self {
            MirTy::I8 => "i8".to_string(),
            MirTy::I16 => "i16".to_string(),
            MirTy::I32 => "i32".to_string(),
            MirTy::I64 => "i64".to_string(),
            MirTy::F32 => "f32".to_string(),
            MirTy::F64 => "f64".to_string(),
            MirTy::Bool => "bool".to_string(),
            MirTy::Unit => "()".to_string(),
            MirTy::Str => "str".to_string(),
            MirTy::Never => "!".to_string(),
            MirTy::Named(name) => name.clone(),
            MirTy::NamedGeneric(name, args) => {
                let args_str: Vec<_> = args.iter().map(|a| a.display()).collect();
                format!("{}[{}]", name, args_str.join(", "))
            },
            MirTy::Ref(inner) => format!("&{}", inner.display()),
            MirTy::RefMut(inner) => format!("&var {}", inner.display()),
            MirTy::Pointer(inner) => format!("*{}", inner.display()),
            MirTy::Tuple(elems) => {
                let elems_str: Vec<_> = elems.iter().map(|e| e.display()).collect();
                format!("({})", elems_str.join(", "))
            },
            MirTy::Func { params, ret } => {
                let params_str: Vec<_> = params.iter().map(|p| p.display()).collect();
                format!("func({}) -> {}", params_str.join(", "), ret.display())
            },
            MirTy::FuncThick { params, ret } => {
                let params_str: Vec<_> = params.iter().map(|p| p.display()).collect();
                format!(
                    "func({}) -> {} [thick]",
                    params_str.join(", "),
                    ret.display()
                )
            },
            MirTy::TypeParam(name) => name.clone(),
            MirTy::AssociatedType {
                base,
                protocol,
                associated,
            } => format!("{}.{} (from {})", base.display(), associated, protocol),
            MirTy::SelfType => "Self".to_string(),
            MirTy::Error => "<error>".to_string(),
        }
    }
}

/// Format an actual MIR type for display in error messages.
pub(crate) fn format_actual_ty(ty_id: Id<Ty>, ctx: &MirContext) -> String {
    ctx.ty(ty_id).display(ctx).to_string()
}
