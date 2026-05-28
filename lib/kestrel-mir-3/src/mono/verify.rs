use kestrel_span::Span;

use crate::callee::Callee;
use crate::immediate::ImmediateKind;
use crate::inst::InstKind;
use crate::mono::types::{MonoFunction, MonoModule};
use crate::ty::MirTy;
use crate::{BlockId, TyId};

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
                message: format!("MonoStruct({:?}, {:?}) missing layout", s.source, s.type_args),
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

    MonoVerifyResult { errors }
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
            message: format!("MonoFunction '{}' has no body and no extern_info", func.name),
            span: None,
        });
        return;
    }

    // Check param types
    for (pi, param) in func.params.iter().enumerate() {
        check_type_concrete(module, fi, None, None, None, param.ty, errors, &format!("param {pi}"));
    }
    check_type_concrete(module, fi, None, None, None, func.ret, errors, "return type");

    let Some(body) = &func.body else { return };

    // Check value types
    for (vi, value) in body.values.iter().enumerate() {
        check_type_concrete(module, fi, None, None, None, value.ty, errors, &format!("value {vi}"));
    }

    // Walk blocks
    let func_count = module.functions.len();
    for (bi, block) in body.blocks.iter().enumerate() {
        let block_id = BlockId::new(bi);

        // Check block param types
        for (pi, param) in block.params.iter().enumerate() {
            check_type_concrete(
                module, fi, Some(block_id), None, None, param.ty, errors,
                &format!("block {bi} param {pi}"),
            );
        }

        for (ii, inst) in block.insts.iter().enumerate() {
            let inst_span = inst.span.as_ref();
            match &inst.kind {
                // Check callees are resolved
                InstKind::Call { callee, .. } => {
                    check_callee(fi, block_id, ii, inst_span, callee, func_count, errors);
                }

                // Check FunctionRef is rewritten to MonoFunctionRef
                InstKind::Literal { value, .. } => {
                    check_literal(module, fi, block_id, ii, inst_span, &value.kind, func_count, errors);
                }

                // Walk InstKind variants with embedded TyId for concreteness
                InstKind::Struct { ty, .. } => {
                    check_type_concrete(module, fi, Some(block_id), Some(ii), inst_span, *ty, errors, "Struct type");
                }
                InstKind::Enum { enum_ty, .. } => {
                    check_type_concrete(module, fi, Some(block_id), Some(ii), inst_span, *enum_ty, errors, "Enum type");
                }
                InstKind::Array { element_ty, .. } => {
                    check_type_concrete(module, fi, Some(block_id), Some(ii), inst_span, *element_ty, errors, "Array element type");
                }
                InstKind::CopyAddr { ty, .. }
                | InstKind::Take { ty, .. }
                | InstKind::BeginBorrowAddr { ty, .. }
                | InstKind::BeginMutBorrowAddr { ty, .. }
                | InstKind::DestroyAddr { ty, .. } => {
                    check_type_concrete(module, fi, Some(block_id), Some(ii), inst_span, *ty, errors, "address type");
                }
                InstKind::FieldAddr { ty, .. } => {
                    check_type_concrete(module, fi, Some(block_id), Some(ii), inst_span, *ty, errors, "FieldAddr type");
                }
                InstKind::Uninit { ty, .. } => {
                    check_type_concrete(module, fi, Some(block_id), Some(ii), inst_span, *ty, errors, "Uninit type");
                }

                // All other instructions: no additional mono verification needed
                _ => {}
            }
        }
    }
}

fn check_callee(
    fi: usize,
    block: BlockId,
    ii: usize,
    span: Option<&Span>,
    callee: &Callee,
    func_count: usize,
    errors: &mut Vec<MonoVerifyError>,
) {
    match callee {
        Callee::Direct { .. } => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block: Some(block),
                inst: Some(ii),
                message: "Callee::Direct not resolved to Callee::Resolved".into(),
                span: span.cloned(),
            });
        }
        Callee::Witness { .. } => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block: Some(block),
                inst: Some(ii),
                message: "Callee::Witness not resolved".into(),
                span: span.cloned(),
            });
        }
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
        }
        Callee::Thin(_) | Callee::Thick(_) => {}
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
        }
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
        }
        ImmediateKind::SizeOf(ty) | ImmediateKind::AlignOf(ty) | ImmediateKind::NullPtr(ty) => {
            check_type_concrete(module, fi, Some(block), Some(ii), span, *ty, errors, "immediate type");
        }
        _ => {}
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
        }
        MirTy::AssociatedProjection { .. } => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block,
                inst,
                message: format!("AssociatedProjection in {context}"),
                span: span.cloned(),
            });
        }
        MirTy::Pointer(inner) => {
            check_type_concrete(module, fi, block, inst, span, *inner, errors, context);
        }
        MirTy::Tuple(elems) => {
            for &elem in elems {
                check_type_concrete(module, fi, block, inst, span, elem, errors, context);
            }
        }
        MirTy::Named { type_args, .. } => {
            for &arg in type_args {
                check_type_concrete(module, fi, block, inst, span, arg, errors, context);
            }
        }
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            for (p, _) in params {
                check_type_concrete(module, fi, block, inst, span, *p, errors, context);
            }
            check_type_concrete(module, fi, block, inst, span, *ret, errors, context);
        }
        _ => {}
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
    use crate::mono::types::{MonoFunction, MonoModule, MonoParam, MonoStruct};
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
        module.structs.insert((entity(2), vec![]), MonoStruct {
            source: entity(2),
            type_args: vec![],
            fields: vec![],
            type_info: TypeInfo {
                layout: Some(Layout::Struct(StructLayout::new())),
                ..TypeInfo::none()
            },
        });

        let result = verify_mono(&module);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
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
        assert!(result.errors.iter().any(|e| e.message.contains("TypeParam")));
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
        assert!(result.errors.iter().any(|e| e.message.contains("FunctionRef")));
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
        assert!(result.errors.iter().any(|e| e.message.contains("out of bounds")));
    }

    #[test]
    fn verify_struct_missing_layout() {
        let mut module = make_module();
        module.structs.insert((entity(1), vec![]), MonoStruct {
            source: entity(1),
            type_args: vec![],
            fields: vec![],
            type_info: TypeInfo::none(),
        });

        let result = verify_mono(&module);
        assert!(!result.is_ok());
        assert!(result.errors.iter().any(|e| e.message.contains("missing layout")));
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
        assert!(result.errors.iter().any(|e| e.message.contains("TypeParam")));
    }
}
