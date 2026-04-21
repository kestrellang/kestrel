//! Name mangling for Kestrel symbols — v0 scheme.
//!
//! Converts entity names with optional type arguments into unique,
//! linker-safe symbol names.
//!
//! # Grammar
//!
//! ```text
//! mangled     = "_K0" path receiver? signature? instantiation? self-disambig?
//! path        = ident | "N" ident+ "E"
//! ident       = length "_" utf8-bytes
//! receiver    = "r" | "m" | "c"              (ref / refmut / consuming)
//! signature   = "Z" param* "E"
//! param       = ("L" ident)? type             (optional label + type)
//! instantiation = "I" type+ "E"
//! self-disambig = "S_" type
//!
//! type = "i1" | "i2" | "i4" | "i8"          (I8..I64)
//!      | "f2" | "f4" | "f8"                  (F16..F64)
//!      | "b" | "v" | "n" | "s" | "X"         (Bool/Unit/Never/Str/Error)
//!      | "P" type                             (Pointer)
//!      | "R" type                             (Ref)
//!      | "M" type                             (RefMut)
//!      | "T" type* "E"                        (Tuple)
//!      | path ("I" type+ "E")?                (Named + optional type args)
//!      | "F" count "_" type* type "E"         (FuncThin)
//!      | "C" count "_" type* type "E"         (FuncThick)
//!      | "S"                                  (SelfType, unresolved)
//!      | "Q" type "p" path ident              (AssociatedProjection: base.protocol.name)
//! ```
//!
//! # Improvements over lib1
//!
//! - Entity names resolved from `MirModule.entity_names` (split on `.`)
//! - `AssociatedProjection` encoding includes protocol name (prevents collisions)
//! - All `Mangler` methods are public for custom mangling strategies
//! - By-value `&MirTy` references (no `Id<Ty>` arena lookups)

use kestrel_mir::{FunctionDef, FunctionKind, MirModule, MirTy, ReceiverConvention};

/// Mangle a named entity with optional type arguments.
pub fn mangle_name(module: &MirModule, name: &str, type_args: &[MirTy]) -> String {
    let mut m = Mangler::new(module);
    m.push_prefix();
    m.mangle_name_path(name);
    m.mangle_instantiation(type_args);
    m.finish()
}

/// Mangle a function with its full signature.
pub fn mangle_function(module: &MirModule, func: &FunctionDef, type_args: &[MirTy]) -> String {
    mangle_function_with_self(module, func, type_args, None)
}

/// Mangle a function with its full signature and optional Self type.
///
/// The `self_type` disambiguates protocol extension methods compiled
/// for different conforming types.
pub fn mangle_function_with_self(
    module: &MirModule,
    func: &FunctionDef,
    type_args: &[MirTy],
    self_type: Option<&MirTy>,
) -> String {
    let mut m = Mangler::new(module);
    m.self_type = self_type;

    m.push_prefix();
    m.mangle_name_path(&func.name);
    m.mangle_receiver(&func.kind);
    m.mangle_signature(func);
    m.mangle_instantiation(type_args);

    // Self type disambiguation suffix for protocol extension methods
    if let Some(st) = self_type {
        m.push_str("S_");
        m.mangle_type(st);
    }

    m.finish()
}

/// Name mangler producing unique linker symbols.
///
/// All methods are public to allow custom mangling strategies by backends.
pub struct Mangler<'a> {
    module: &'a MirModule,
    output: String,
    /// Concrete Self type for protocol extension method mangling.
    /// When set, `SelfType` in type encoding is replaced with this type.
    pub self_type: Option<&'a MirTy>,
}

impl<'a> Mangler<'a> {
    pub fn new(module: &'a MirModule) -> Self {
        Self {
            module,
            output: String::with_capacity(64),
            self_type: None,
        }
    }

    pub fn push_prefix(&mut self) {
        self.output.push_str("_K0");
    }

    pub fn push_str(&mut self, s: &str) {
        self.output.push_str(s);
    }

    pub fn push(&mut self, c: char) {
        self.output.push(c);
    }

    /// Consume the mangler and return the mangled name.
    pub fn finish(self) -> String {
        self.output
    }

    /// Mangle a qualified name string by splitting on `.`.
    ///
    /// Single-segment names are encoded directly. Multi-segment names
    /// are wrapped in `N...E`. The last segment has `$` suffixes stripped
    /// (overload disambiguation is handled by the signature instead).
    pub fn mangle_name_path(&mut self, name: &str) {
        let segments: Vec<&str> = name.split('.').collect();

        if segments.len() == 1 {
            self.mangle_ident(strip_dollar(segments[0]));
        } else {
            self.push('N');
            for (i, segment) in segments.iter().enumerate() {
                if i == segments.len() - 1 {
                    self.mangle_ident(strip_dollar(segment));
                } else {
                    self.mangle_ident(segment);
                }
            }
            self.push('E');
        }
    }

    /// Mangle an identifier as `{byte_len}_{utf8_bytes}`.
    pub fn mangle_ident(&mut self, s: &str) {
        self.push_str(&s.len().to_string());
        self.push('_');
        self.push_str(s);
    }

    /// Mangle receiver convention from function kind.
    pub fn mangle_receiver(&mut self, kind: &FunctionKind) {
        if let FunctionKind::Method { receiver, .. } = kind {
            match receiver {
                ReceiverConvention::Ref => self.push('r'),
                ReceiverConvention::RefMut => self.push('m'),
                ReceiverConvention::Consuming => self.push('c'),
            }
        }
    }

    /// Mangle function signature: `Z` params `E`.
    ///
    /// Skips the self parameter for methods and deinits. For initializers,
    /// detects self by name.
    pub fn mangle_signature(&mut self, func: &FunctionDef) {
        self.push('Z');

        let skip = match &func.kind {
            FunctionKind::Method { .. } | FunctionKind::Deinit { .. } => 1,
            FunctionKind::Initializer { .. } => {
                if func.params.first().is_some_and(|p| p.name == "self") {
                    1
                } else {
                    0
                }
            },
            _ => 0,
        };

        for param in func.params.iter().skip(skip) {
            if let Some(ref label) = param.external_label {
                self.push('L');
                self.mangle_ident(label);
            }
            self.mangle_type(&param.ty);
        }

        self.push('E');
    }

    /// Mangle type argument instantiation: `I` types `E` if non-empty.
    pub fn mangle_instantiation(&mut self, type_args: &[MirTy]) {
        if !type_args.is_empty() {
            self.push('I');
            for ty in type_args {
                self.mangle_type(ty);
            }
            self.push('E');
        }
    }

    /// Mangle a MIR type.
    pub fn mangle_type(&mut self, ty: &MirTy) {
        match ty {
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
                self.mangle_type(inner);
            },
            MirTy::Ref(inner) => {
                self.push('R');
                self.mangle_type(inner);
            },
            MirTy::RefMut(inner) => {
                self.push('M');
                self.mangle_type(inner);
            },

            MirTy::Tuple(elems) => {
                self.push('T');
                for elem in elems {
                    self.mangle_type(elem);
                }
                self.push('E');
            },

            MirTy::Named { entity, type_args } => {
                let name = self.module.resolve_name(*entity);
                // Clone to release borrow on self.module before calling mangle methods
                let name = name.to_owned();
                self.mangle_name_path(&name);
                if !type_args.is_empty() {
                    self.push('I');
                    for ty in type_args {
                        self.mangle_type(ty);
                    }
                    self.push('E');
                }
            },

            MirTy::TypeParam(entity) => {
                let name = self.module.resolve_name(*entity).to_owned();
                self.mangle_ident(&name);
            },

            MirTy::FuncThin { params, ret } => {
                self.push('F');
                self.push_str(&params.len().to_string());
                self.push('_');
                for p in params {
                    self.mangle_type(p);
                }
                self.mangle_type(ret);
                self.push('E');
            },

            MirTy::FuncThick { params, ret } => {
                self.push('C');
                self.push_str(&params.len().to_string());
                self.push('_');
                for p in params {
                    self.mangle_type(p);
                }
                self.mangle_type(ret);
                self.push('E');
            },

            MirTy::SelfType => {
                if let Some(st) = self.self_type {
                    // Clone to avoid double borrow
                    let st = st.clone();
                    self.mangle_type(&st);
                } else {
                    self.push('S');
                }
            },

            // Includes protocol name to prevent collisions between different
            // protocols defining the same associated type name (e.g. Iterator.Item
            // vs Container.Item on the same base type).
            MirTy::AssociatedProjection {
                base,
                protocol,
                name,
            } => {
                self.push('Q');
                self.mangle_type(base);
                self.push('p');
                let protocol_name = self.module.resolve_name(*protocol).to_owned();
                self.mangle_name_path(&protocol_name);
                self.mangle_ident(name);
            },

            MirTy::Error => {
                self.push('X');
            },
        }
    }
}

/// Strip `$` disambiguation suffix from a path segment.
fn strip_dollar(segment: &str) -> &str {
    segment.split('$').next().unwrap_or(segment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_mir::{FunctionDef, FunctionKind, LocalId, ParamDef, ReceiverConvention};

    fn dummy_entity(id: u32) -> kestrel_hecs::Entity {
        kestrel_hecs::Entity::from_raw(id)
    }

    fn test_module() -> MirModule {
        MirModule::new("test")
    }

    // --- Name mangling ---

    #[test]
    fn single_segment_name() {
        let module = test_module();
        let result = mangle_name(&module, "main", &[]);
        assert_eq!(result, "_K04_main");
    }

    #[test]
    fn multi_segment_name() {
        let module = test_module();
        let result = mangle_name(&module, "std.collections.Array", &[]);
        assert_eq!(result, "_K0N3_std11_collections5_ArrayE");
    }

    #[test]
    fn name_with_type_args() {
        let module = test_module();
        let result = mangle_name(&module, "Array", &[MirTy::I64]);
        assert_eq!(result, "_K05_ArrayIi8E");
    }

    #[test]
    fn dollar_suffix_stripped() {
        let module = test_module();
        let result = mangle_name(&module, "std.foo$1", &[]);
        assert_eq!(result, "_K0N3_std3_fooE");
    }

    // --- Type encoding ---

    #[test]
    fn primitive_type_encoding() {
        let module = test_module();
        let mut m = Mangler::new(&module);
        m.mangle_type(&MirTy::I8);
        m.mangle_type(&MirTy::I16);
        m.mangle_type(&MirTy::I32);
        m.mangle_type(&MirTy::I64);
        m.mangle_type(&MirTy::F16);
        m.mangle_type(&MirTy::F32);
        m.mangle_type(&MirTy::F64);
        m.mangle_type(&MirTy::Bool);
        m.mangle_type(&MirTy::Unit);
        m.mangle_type(&MirTy::Never);
        m.mangle_type(&MirTy::Str);
        assert_eq!(m.finish(), "i1i2i4i8f2f4f8bvns");
    }

    #[test]
    fn pointer_type_encoding() {
        let module = test_module();
        let mut m = Mangler::new(&module);
        m.mangle_type(&MirTy::Pointer(Box::new(MirTy::I32)));
        assert_eq!(m.finish(), "Pi4");
    }

    #[test]
    fn ref_type_encoding() {
        let module = test_module();
        let mut m = Mangler::new(&module);
        m.mangle_type(&MirTy::Ref(Box::new(MirTy::I64)));
        m.mangle_type(&MirTy::RefMut(Box::new(MirTy::Bool)));
        assert_eq!(m.finish(), "Ri8Mb");
    }

    #[test]
    fn tuple_type_encoding() {
        let module = test_module();
        let mut m = Mangler::new(&module);
        m.mangle_type(&MirTy::Tuple(vec![MirTy::I32, MirTy::Bool]));
        assert_eq!(m.finish(), "Ti4bE");
    }

    #[test]
    fn named_type_encoding() {
        let mut module = test_module();
        let entity = dummy_entity(1);
        module.register_name(entity, "std.Int64");

        let mut m = Mangler::new(&module);
        m.mangle_type(&MirTy::Named {
            entity,
            type_args: vec![],
        });
        assert_eq!(m.finish(), "N3_std5_Int64E");
    }

    #[test]
    fn named_type_with_args() {
        let mut module = test_module();
        let entity = dummy_entity(1);
        module.register_name(entity, "Array");

        let mut m = Mangler::new(&module);
        m.mangle_type(&MirTy::Named {
            entity,
            type_args: vec![MirTy::I64],
        });
        assert_eq!(m.finish(), "5_ArrayIi8E");
    }

    #[test]
    fn func_thin_encoding() {
        let module = test_module();
        let mut m = Mangler::new(&module);
        m.mangle_type(&MirTy::FuncThin {
            params: vec![MirTy::I32, MirTy::I32],
            ret: Box::new(MirTy::Bool),
        });
        assert_eq!(m.finish(), "F2_i4i4bE");
    }

    #[test]
    fn func_thick_encoding() {
        let module = test_module();
        let mut m = Mangler::new(&module);
        m.mangle_type(&MirTy::FuncThick {
            params: vec![MirTy::I64],
            ret: Box::new(MirTy::Unit),
        });
        assert_eq!(m.finish(), "C1_i8vE");
    }

    #[test]
    fn self_type_unresolved() {
        let module = test_module();
        let mut m = Mangler::new(&module);
        m.mangle_type(&MirTy::SelfType);
        assert_eq!(m.finish(), "S");
    }

    #[test]
    fn self_type_resolved() {
        let module = test_module();
        let concrete = MirTy::I64;
        let mut m = Mangler::new(&module);
        m.self_type = Some(&concrete);
        m.mangle_type(&MirTy::SelfType);
        assert_eq!(m.finish(), "i8");
    }

    #[test]
    fn associated_projection_includes_protocol() {
        let mut module = test_module();
        let protocol_entity = dummy_entity(1);
        module.register_name(protocol_entity, "Iterator");

        let mut m = Mangler::new(&module);
        m.mangle_type(&MirTy::AssociatedProjection {
            base: Box::new(MirTy::SelfType),
            protocol: protocol_entity,
            name: "Item".into(),
        });
        // Q + base(S) + p + protocol(Iterator) + assoc(Item)
        assert_eq!(m.finish(), "QSp8_Iterator4_Item");
    }

    // --- Function mangling ---

    #[test]
    fn free_function() {
        let module = test_module();
        let func = FunctionDef::new(dummy_entity(1), "add", MirTy::I64);
        let result = mangle_function(&module, &func, &[]);
        assert_eq!(result, "_K03_addZE");
    }

    #[test]
    fn function_with_params() {
        let module = test_module();
        let mut func = FunctionDef::new(dummy_entity(1), "add", MirTy::I64);
        func.params
            .push(ParamDef::new("x", LocalId::new(0), MirTy::I64));
        func.params
            .push(ParamDef::new("y", LocalId::new(1), MirTy::I64));

        let result = mangle_function(&module, &func, &[]);
        assert_eq!(result, "_K03_addZi8i8E");
    }

    #[test]
    fn method_with_receiver() {
        let module = test_module();
        let mut func = FunctionDef::new(dummy_entity(1), "std.Array.count", MirTy::I64);
        func.kind = FunctionKind::Method {
            parent: dummy_entity(2),
            receiver: ReceiverConvention::Ref,
        };
        // Self param (skipped in signature)
        func.params
            .push(ParamDef::new("self", LocalId::new(0), MirTy::SelfType));

        let result = mangle_function(&module, &func, &[]);
        assert_eq!(result, "_K0N3_std5_Array5_countErZE");
    }

    #[test]
    fn labeled_params() {
        let module = test_module();
        let mut func = FunctionDef::new(dummy_entity(1), "insert", MirTy::Unit);
        func.params.push(ParamDef::with_label(
            "value",
            LocalId::new(0),
            MirTy::I64,
            Some("at".into()),
        ));

        let result = mangle_function(&module, &func, &[]);
        assert_eq!(result, "_K06_insertZL2_ati8E");
    }

    #[test]
    fn function_with_type_args() {
        let module = test_module();
        let func = FunctionDef::new(dummy_entity(1), "identity", MirTy::I64);
        let result = mangle_function(&module, &func, &[MirTy::I64]);
        assert_eq!(result, "_K08_identityZEIi8E");
    }

    #[test]
    fn self_type_disambiguation() {
        let module = test_module();
        let mut func = FunctionDef::new(dummy_entity(1), "std.Iterator.next", MirTy::Unit);
        func.kind = FunctionKind::Method {
            parent: dummy_entity(2),
            receiver: ReceiverConvention::RefMut,
        };
        func.params
            .push(ParamDef::new("self", LocalId::new(0), MirTy::SelfType));

        let self_type = MirTy::Named {
            entity: dummy_entity(3),
            type_args: vec![MirTy::I64],
        };

        let mut module_with_names = module;
        module_with_names.register_name(dummy_entity(3), "ArrayIterator");

        let result = mangle_function_with_self(&module_with_names, &func, &[], Some(&self_type));
        assert_eq!(
            result,
            "_K0N3_std8_Iterator4_nextEmZES_13_ArrayIteratorIi8E"
        );
    }

    #[test]
    fn initializer_skips_self() {
        let module = test_module();
        let mut func = FunctionDef::new(dummy_entity(1), "Point.init", MirTy::Unit);
        func.kind = FunctionKind::Initializer {
            parent: dummy_entity(2),
        };
        func.params
            .push(ParamDef::new("self", LocalId::new(0), MirTy::SelfType));
        func.params
            .push(ParamDef::new("x", LocalId::new(1), MirTy::I64));
        func.params
            .push(ParamDef::new("y", LocalId::new(2), MirTy::I64));

        let result = mangle_function(&module, &func, &[]);
        assert_eq!(result, "_K0N5_Point4_initEZi8i8E");
    }
}
