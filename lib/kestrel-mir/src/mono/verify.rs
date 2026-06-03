use kestrel_hecs::Entity;
use kestrel_span::Span;

use crate::callee::Callee;
use crate::immediate::ImmediateKind;
use crate::inst::InstKind;
use crate::mono::types::{MonoFunction, MonoModule};
use crate::ty::MirTy;
use crate::{BlockId, CopyBehavior, TyId};

// -- Verification result --

#[derive(Debug, Clone)]
pub struct MonoVerifyError {
    pub func_idx: usize,
    pub block: Option<BlockId>,
    pub inst: Option<usize>,
    pub message: String,
    /// Source span from the instruction, if available.
    pub span: Option<Span>,
}

#[derive(Debug)]
pub struct MonoVerifyResult {
    pub errors: Vec<MonoVerifyError>,
}

impl MonoVerifyResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

// -- Verification --

pub fn verify_mono(module: &MonoModule) -> MonoVerifyResult {
    let mut errors = Vec::new();

    // Check all function bodies
    for (fi, func) in module.functions.iter().enumerate() {
        verify_function(module, fi, func, &mut errors);
    }

    // Check all structs have layouts
    for s in module.structs.values() {
        if s.type_info.layout.is_none() {
            errors.push(MonoVerifyError {
                func_idx: 0,
                block: None,
                inst: None,
                message: format!(
                    "MonoStruct({:?}, {:?}) missing layout",
                    s.source, s.type_args
                ),
                span: None,
            });
        }
    }

    // Check all enums have layouts
    for e in module.enums.values() {
        if e.type_info.layout.is_none() {
            errors.push(MonoVerifyError {
                func_idx: 0,
                block: None,
                inst: None,
                message: format!("MonoEnum({:?}, {:?}) missing layout", e.source, e.type_args),
                span: None,
            });
        }
    }

    // Invariant 3b: a Copyable/Cloneable type must not contain a non-Copyable
    // child. The frontend now enforces the implicit `T: Copyable` bound, so the
    // bad instantiations this would flag (e.g. `Box[NotCopyable]`) can no longer
    // be constructed — this is pure defense-in-depth.
    verify_copyable_containment(module, &mut errors);

    MonoVerifyResult { errors }
}

/// Invariant 3b (defense-in-depth): no `Bitwise`/`Clone` type may contain a
/// `None` (non-Copyable) child. The frontend classifier maintains this — a
/// non-copyable child forces the container `NotCopyable` — so this should never
/// fire in a correct build; it converts a silent inconsistency (a bit-copyable
/// type aliasing a move-only resource) into a loud verification error.
fn verify_copyable_containment(module: &MonoModule, errors: &mut Vec<MonoVerifyError>) {
    let child_copy = |ty: TyId| -> Option<CopyBehavior> {
        if let MirTy::Named { entity, type_args } = module.ty_arena.get(ty) {
            let key = (*entity, type_args.clone());
            module
                .structs
                .get(&key)
                .map(|s| s.type_info.copy.clone())
                .or_else(|| module.enums.get(&key).map(|e| e.type_info.copy.clone()))
        } else {
            None
        }
    };

    for s in module.structs.values() {
        if !matches!(
            s.type_info.copy,
            CopyBehavior::Bitwise | CopyBehavior::Clone(_)
        ) {
            continue;
        }
        for f in &s.fields {
            if matches!(child_copy(f.ty), Some(CopyBehavior::None)) {
                errors.push(MonoVerifyError {
                    func_idx: 0,
                    block: None,
                    inst: None,
                    message: format!(
                        "Copyable type MonoStruct({:?}, {:?}) contains non-Copyable field '{}'",
                        s.source, s.type_args, f.name
                    ),
                    span: None,
                });
            }
        }
    }
    for e in module.enums.values() {
        if !matches!(
            e.type_info.copy,
            CopyBehavior::Bitwise | CopyBehavior::Clone(_)
        ) {
            continue;
        }
        for case in &e.cases {
            for f in &case.payload_fields {
                if matches!(child_copy(f.ty), Some(CopyBehavior::None)) {
                    errors.push(MonoVerifyError {
                        func_idx: 0,
                        block: None,
                        inst: None,
                        message: format!(
                            "Copyable type MonoEnum({:?}, {:?}) case '{}' contains non-Copyable field '{}'",
                            e.source, e.type_args, case.name, f.name
                        ),
                        span: None,
                    });
                }
            }
        }
    }
}

fn verify_function(
    module: &MonoModule,
    fi: usize,
    func: &MonoFunction,
    errors: &mut Vec<MonoVerifyError>,
) {
    // Body must be present unless extern
    if func.body.is_none() && func.extern_info.is_none() {
        errors.push(MonoVerifyError {
            func_idx: fi,
            block: None,
            inst: None,
            message: format!(
                "MonoFunction '{}' has no body and no extern_info",
                func.name
            ),
            span: None,
        });
        return;
    }

    // Check param types
    for (pi, param) in func.params.iter().enumerate() {
        check_type_concrete(
            module,
            fi,
            None,
            None,
            None,
            param.ty,
            errors,
            &format!("param {pi}"),
        );
    }
    check_type_concrete(
        module,
        fi,
        None,
        None,
        None,
        func.ret,
        errors,
        "return type",
    );

    let Some(body) = &func.body else { return };

    // Check value types — inherit the value's defining span (T1: interned types
    // can't carry a span, so the best location is where the value was defined).
    for (vi, value) in body.values.iter().enumerate() {
        check_type_concrete(
            module,
            fi,
            None,
            None,
            value.span.as_ref(),
            value.ty,
            errors,
            &format!("value {vi}"),
        );
    }

    // Walk blocks
    let func_count = module.functions.len();
    for (bi, block) in body.blocks.iter().enumerate() {
        let block_id = BlockId::new(bi);

        // Check block param types — inherit the param value's defining span.
        for (pi, param) in block.params.iter().enumerate() {
            let pspan = body
                .values
                .get(param.value.index())
                .and_then(|vd| vd.span.as_ref());
            check_type_concrete(
                module,
                fi,
                Some(block_id),
                None,
                pspan,
                param.ty,
                errors,
                &format!("block {bi} param {pi}"),
            );
        }

        for (ii, inst) in block.insts.iter().enumerate() {
            let inst_span = inst.span.as_ref();
            match &inst.kind {
                // Check callees are resolved
                InstKind::Call { callee, .. } => {
                    check_callee(module, fi, block_id, ii, inst_span, callee, func_count, errors);
                },

                // Check FunctionRef is rewritten to MonoFunctionRef
                InstKind::Literal { value, .. } => {
                    check_literal(
                        module,
                        fi,
                        block_id,
                        ii,
                        inst_span,
                        &value.kind,
                        func_count,
                        errors,
                    );
                },

                // Walk InstKind variants with embedded TyId for concreteness
                InstKind::Struct { ty, .. } => {
                    check_type_concrete(
                        module,
                        fi,
                        Some(block_id),
                        Some(ii),
                        inst_span,
                        *ty,
                        errors,
                        "Struct type",
                    );
                },
                InstKind::Enum { enum_ty, .. } => {
                    check_type_concrete(
                        module,
                        fi,
                        Some(block_id),
                        Some(ii),
                        inst_span,
                        *enum_ty,
                        errors,
                        "Enum type",
                    );
                },
                InstKind::Array { element_ty, .. } => {
                    check_type_concrete(
                        module,
                        fi,
                        Some(block_id),
                        Some(ii),
                        inst_span,
                        *element_ty,
                        errors,
                        "Array element type",
                    );
                },
                InstKind::CopyAddr { ty, .. }
                | InstKind::Take { ty, .. }
                | InstKind::BeginBorrowAddr { ty, .. }
                | InstKind::BeginMutBorrowAddr { ty, .. }
                | InstKind::DestroyAddr { ty, .. } => {
                    check_type_concrete(
                        module,
                        fi,
                        Some(block_id),
                        Some(ii),
                        inst_span,
                        *ty,
                        errors,
                        "address type",
                    );
                },
                InstKind::FieldAddr { ty, .. } => {
                    check_type_concrete(
                        module,
                        fi,
                        Some(block_id),
                        Some(ii),
                        inst_span,
                        *ty,
                        errors,
                        "FieldAddr type",
                    );
                },
                InstKind::Uninit { ty, .. } => {
                    check_type_concrete(
                        module,
                        fi,
                        Some(block_id),
                        Some(ii),
                        inst_span,
                        *ty,
                        errors,
                        "Uninit type",
                    );
                },

                // All other instructions: no additional mono verification needed
                _ => {},
            }
        }
    }
}

fn check_callee(
    module: &MonoModule,
    fi: usize,
    block: BlockId,
    ii: usize,
    span: Option<&Span>,
    callee: &Callee,
    func_count: usize,
    errors: &mut Vec<MonoVerifyError>,
) {
    match callee {
        Callee::Direct {
            func,
            type_args,
            self_type,
        } => {
            let name = module
                .entity_names
                .get(func)
                .map(|s| s.as_str())
                .unwrap_or("<unknown>");
            let targs: Vec<String> = type_args
                .iter()
                .map(|&t| describe_mono_ty(module, t))
                .collect();
            let stype = self_type
                .map(|t| describe_mono_ty(module, t))
                .unwrap_or_else(|| "None".into());
            errors.push(MonoVerifyError {
                func_idx: fi,
                block: Some(block),
                inst: Some(ii),
                message: format!(
                    "Callee::Direct not resolved to Callee::Resolved \
                     (callee='{name}' {func:?}, type_args=[{}], self_type={stype})",
                    targs.join(", ")
                ),
                span: span.cloned(),
            });
        },
        Callee::Witness { .. } => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block: Some(block),
                inst: Some(ii),
                message: "Callee::Witness not resolved".into(),
                span: span.cloned(),
            });
        },
        Callee::Resolved(id) => {
            if id.index() >= func_count {
                errors.push(MonoVerifyError {
                    func_idx: fi,
                    block: Some(block),
                    inst: Some(ii),
                    message: format!(
                        "Callee::Resolved({}) out of bounds ({})",
                        id.index(),
                        func_count
                    ),
                    span: span.cloned(),
                });
            }
        },
        Callee::Thin(_) | Callee::Thick(_) => {},
    }
}

/// Compact, readable description of a TyId for diagnostics — surfaces the
/// non-concrete shapes (TypeParam / AssociatedProjection / Error) that cause a
/// mono key to miss `func_id_map`.
fn describe_mono_ty(module: &MonoModule, ty: TyId) -> String {
    use crate::ty::MirTy;
    let name_of = |e: &Entity| {
        module
            .entity_names
            .get(e)
            .cloned()
            .unwrap_or_else(|| format!("{e:?}"))
    };
    match module.ty_arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            if type_args.is_empty() {
                name_of(entity)
            } else {
                let args: Vec<String> =
                    type_args.iter().map(|&a| describe_mono_ty(module, a)).collect();
                format!("{}[{}]", name_of(entity), args.join(", "))
            }
        },
        MirTy::Pointer(inner) => format!("Pointer[{}]", describe_mono_ty(module, *inner)),
        MirTy::TypeParam(e) => format!("TypeParam({})", name_of(e)),
        MirTy::AssociatedProjection {
            base,
            protocol,
            assoc_type,
        } => format!(
            "AssocProj({}::{}.{})",
            describe_mono_ty(module, *base),
            name_of(protocol),
            name_of(assoc_type)
        ),
        MirTy::Error => "Error".into(),
        other => format!("{other:?}"),
    }
}

/// Check literal immediates for unresolved FunctionRef and type concreteness.
fn check_literal(
    module: &MonoModule,
    fi: usize,
    block: BlockId,
    ii: usize,
    span: Option<&Span>,
    kind: &ImmediateKind,
    func_count: usize,
    errors: &mut Vec<MonoVerifyError>,
) {
    match kind {
        ImmediateKind::FunctionRef { .. } => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block: Some(block),
                inst: Some(ii),
                message: "ImmediateKind::FunctionRef not resolved to MonoFunctionRef".into(),
                span: span.cloned(),
            });
        },
        ImmediateKind::MonoFunctionRef(id) => {
            if id.index() >= func_count {
                errors.push(MonoVerifyError {
                    func_idx: fi,
                    block: Some(block),
                    inst: Some(ii),
                    message: format!(
                        "MonoFunctionRef({}) out of bounds ({})",
                        id.index(),
                        func_count
                    ),
                    span: span.cloned(),
                });
            }
        },
        ImmediateKind::SizeOf(ty) | ImmediateKind::AlignOf(ty) | ImmediateKind::NullPtr(ty) => {
            check_type_concrete(
                module,
                fi,
                Some(block),
                Some(ii),
                span,
                *ty,
                errors,
                "immediate type",
            );
        },
        _ => {},
    }
}

fn check_type_concrete(
    module: &MonoModule,
    fi: usize,
    block: Option<BlockId>,
    inst: Option<usize>,
    span: Option<&Span>,
    ty: TyId,
    errors: &mut Vec<MonoVerifyError>,
    context: &str,
) {
    match module.ty_arena.get(ty) {
        MirTy::TypeParam(e) => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block,
                inst,
                message: format!("TypeParam({e:?}) in {context}"),
                span: span.cloned(),
            });
        },
        MirTy::AssociatedProjection { .. } => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block,
                inst,
                message: format!("AssociatedProjection in {context}"),
                span: span.cloned(),
            });
        },
        MirTy::Pointer(inner) => {
            check_type_concrete(module, fi, block, inst, span, *inner, errors, context);
        },
        MirTy::Tuple(elems) => {
            for &elem in elems {
                check_type_concrete(module, fi, block, inst, span, elem, errors, context);
            }
        },
        MirTy::Named { type_args, .. } => {
            for &arg in type_args {
                check_type_concrete(module, fi, block, inst, span, arg, errors, context);
            }
        },
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            for (p, _) in params {
                check_type_concrete(module, fi, block, inst, span, *p, errors, context);
            }
            check_type_concrete(module, fi, block, inst, span, *ret, errors, context);
        },
        _ => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BasicBlock;
    use crate::body::OssaBody;
    use crate::immediate::Immediate;
    use crate::inst::Instruction;
    use crate::item::function::ExternInfo;
    use crate::item::{Layout, TypeInfo};
    use crate::layout::StructLayout;
    use crate::mono::types::{MonoField, MonoFunction, MonoModule, MonoParam, MonoStruct};
    use crate::terminator::{Terminator, TerminatorKind};
    use crate::ty::{ParamConvention, TyArena};
    use crate::value::{Ownership, ValueDef};
    use crate::{MonoFuncId, ValueId};
    use indexmap::IndexMap;
    use kestrel_hecs::Entity;

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    /// Build a minimal OssaBody: one value (unit), one block returning it.
    fn simple_body(arena: &mut TyArena) -> OssaBody {
        let unit = arena.unit();
        let ret_val = ValueId::new(0);
        let mut block = BasicBlock::new();
        block.terminator = Terminator::new(TerminatorKind::Return(ret_val));
        OssaBody {
            values: vec![ValueDef::owned(unit)],
            blocks: vec![block],
            entry: BlockId::new(0),
            param_count: 0,
        }
    }

    fn make_module() -> MonoModule {
        let arena = TyArena::new();
        MonoModule::new(arena)
    }

    // -- Tests --

    #[test]
    fn verify_clean_module() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let body = simple_body(&mut module.ty_arena);
        module.add_function(MonoFunction {
            name: "_K04_main".into(),
            source: entity(1),
            type_args: vec![],
            self_type: None,
            params: vec![],
            ret: unit,
            body: Some(body),
            extern_info: None,
        });
        module.structs.insert(
            (entity(2), vec![]),
            MonoStruct {
                source: entity(2),
                type_args: vec![],
                fields: vec![],
                type_info: TypeInfo {
                    layout: Some(Layout::Struct(StructLayout::new())),
                    ..TypeInfo::none()
                },
            },
        );

        let result = verify_mono(&module);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn copyable_containment_flags_non_copyable_child() {
        let mut module = make_module();
        let resource_ty = module.ty_arena.named(entity(2), vec![]);
        // Resource: not Copyable.
        module.structs.insert(
            (entity(2), vec![]),
            MonoStruct {
                source: entity(2),
                type_args: vec![],
                fields: vec![],
                type_info: TypeInfo {
                    copy: CopyBehavior::None,
                    ..TypeInfo::none()
                },
            },
        );
        // Wrapper: Bitwise (Copyable) but contains a non-Copyable Resource field.
        module.structs.insert(
            (entity(3), vec![]),
            MonoStruct {
                source: entity(3),
                type_args: vec![],
                fields: vec![MonoField {
                    name: "r".into(),
                    ty: resource_ty,
                }],
                type_info: TypeInfo {
                    copy: CopyBehavior::Bitwise,
                    ..TypeInfo::none()
                },
            },
        );

        let mut errors = Vec::new();
        verify_copyable_containment(&module, &mut errors);
        assert_eq!(errors.len(), 1, "{:?}", errors);
        assert!(errors[0].message.contains("non-Copyable field 'r'"));
    }

    #[test]
    fn copyable_containment_allows_copyable_child() {
        let mut module = make_module();
        let i64t = module.ty_arena.i64();
        module.structs.insert(
            (entity(3), vec![]),
            MonoStruct {
                source: entity(3),
                type_args: vec![],
                fields: vec![MonoField {
                    name: "x".into(),
                    ty: i64t,
                }],
                type_info: TypeInfo {
                    copy: CopyBehavior::Bitwise,
                    ..TypeInfo::none()
                },
            },
        );
        let mut errors = Vec::new();
        verify_copyable_containment(&module, &mut errors);
        assert!(errors.is_empty(), "{:?}", errors);
    }

    #[test]
    fn verify_rejects_type_param() {
        let mut module = make_module();
        let tp = module.ty_arena.intern(MirTy::TypeParam(entity(99)));
        let unit = module.ty_arena.unit();
        let body = simple_body(&mut module.ty_arena);
        module.add_function(MonoFunction {
            name: "bad".into(),
            source: entity(1),
            type_args: vec![],
            self_type: None,
            params: vec![MonoParam::new("x", tp, ParamConvention::Consuming)],
            ret: unit,
            body: Some(body),
            extern_info: None,
        });

        let result = verify_mono(&module);
        assert!(!result.is_ok());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("TypeParam"))
        );
    }

    #[test]
    fn verify_rejects_callee_direct() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let ret_val = ValueId::new(0);
        let mut block = BasicBlock::new();
        block.insts.push(Instruction::new(InstKind::Call {
            result: None,
            callee: Callee::Direct {
                func: entity(99),
                type_args: vec![],
                self_type: None,
            },
            args: vec![],
        }));
        block.terminator = Terminator::new(TerminatorKind::Return(ret_val));
        let body = OssaBody {
            values: vec![ValueDef::owned(unit)],
            blocks: vec![block],
            entry: BlockId::new(0),
            param_count: 0,
        };
        module.add_function(MonoFunction {
            name: "bad".into(),
            source: entity(1),
            type_args: vec![],
            self_type: None,
            params: vec![],
            ret: unit,
            body: Some(body),
            extern_info: None,
        });

        let result = verify_mono(&module);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.message.contains("Direct")));
    }

    #[test]
    fn verify_rejects_function_ref() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let ret_val = ValueId::new(0);
        let lit_val = ValueId::new(1);
        let mut block = BasicBlock::new();
        block.insts.push(Instruction::new(InstKind::Literal {
            result: lit_val,
            value: Immediate::new(ImmediateKind::FunctionRef {
                func: entity(99),
                type_args: vec![],
                self_type: None,
            }),
        }));
        block.terminator = Terminator::new(TerminatorKind::Return(ret_val));
        let body = OssaBody {
            values: vec![ValueDef::owned(unit), ValueDef::owned(unit)],
            blocks: vec![block],
            entry: BlockId::new(0),
            param_count: 0,
        };
        module.add_function(MonoFunction {
            name: "bad".into(),
            source: entity(1),
            type_args: vec![],
            self_type: None,
            params: vec![],
            ret: unit,
            body: Some(body),
            extern_info: None,
        });

        let result = verify_mono(&module);
        assert!(!result.is_ok());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("FunctionRef"))
        );
    }

    #[test]
    fn verify_mono_func_id_bounds() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let ret_val = ValueId::new(0);
        let mut block = BasicBlock::new();
        block.insts.push(Instruction::new(InstKind::Call {
            result: None,
            callee: Callee::Resolved(MonoFuncId::new(999)),
            args: vec![],
        }));
        block.terminator = Terminator::new(TerminatorKind::Return(ret_val));
        let body = OssaBody {
            values: vec![ValueDef::owned(unit)],
            blocks: vec![block],
            entry: BlockId::new(0),
            param_count: 0,
        };
        module.add_function(MonoFunction {
            name: "bad".into(),
            source: entity(1),
            type_args: vec![],
            self_type: None,
            params: vec![],
            ret: unit,
            body: Some(body),
            extern_info: None,
        });

        let result = verify_mono(&module);
        assert!(!result.is_ok());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("out of bounds"))
        );
    }

    #[test]
    fn verify_struct_missing_layout() {
        let mut module = make_module();
        module.structs.insert(
            (entity(1), vec![]),
            MonoStruct {
                source: entity(1),
                type_args: vec![],
                fields: vec![],
                type_info: TypeInfo::none(),
            },
        );

        let result = verify_mono(&module);
        assert!(!result.is_ok());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("missing layout"))
        );
    }

    #[test]
    fn verify_extern_no_body_ok() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        module.add_function(MonoFunction {
            name: "malloc".into(),
            source: entity(1),
            type_args: vec![],
            self_type: None,
            params: vec![],
            ret: unit,
            body: None,
            extern_info: Some(ExternInfo {
                calling_convention: crate::item::function::CallingConvention::C,
                symbol_name: "malloc".into(),
            }),
        });

        let result = verify_mono(&module);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn verify_no_body_no_extern_rejected() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        module.add_function(MonoFunction {
            name: "bad".into(),
            source: entity(1),
            type_args: vec![],
            self_type: None,
            params: vec![],
            ret: unit,
            body: None,
            extern_info: None,
        });

        let result = verify_mono(&module);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.message.contains("no body")));
    }

    #[test]
    fn verify_block_param_type_checked() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let tp = module.ty_arena.intern(MirTy::TypeParam(entity(42)));
        let ret_val = ValueId::new(0);
        let param_val = ValueId::new(1);
        let mut block = BasicBlock::new();
        block.params.push(crate::block::BlockParam {
            value: param_val,
            ty: tp,
            ownership: Ownership::Owned,
        });
        block.terminator = Terminator::new(TerminatorKind::Return(ret_val));
        let body = OssaBody {
            values: vec![ValueDef::owned(unit), ValueDef::owned(tp)],
            blocks: vec![block],
            entry: BlockId::new(0),
            param_count: 0,
        };
        module.add_function(MonoFunction {
            name: "bad".into(),
            source: entity(1),
            type_args: vec![],
            self_type: None,
            params: vec![],
            ret: unit,
            body: Some(body),
            extern_info: None,
        });

        let result = verify_mono(&module);
        assert!(!result.is_ok());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("TypeParam"))
        );
    }
}
