use crate::immediate::ImmediateKind;
use crate::mono::types::{MonoModule, MonoFunction};
use crate::operand::Operand;
use crate::statement::{Callee, StatementKind};
use crate::ty::MirTy;
use crate::{BlockId, TyId};

// -- Verification result --

#[derive(Debug, Clone)]
pub struct MonoVerifyError {
    pub func_idx: usize,
    pub block: Option<BlockId>,
    pub stmt: Option<usize>,
    pub message: String,
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
    for (i, s) in module.structs.iter().enumerate() {
        if s.type_info.layout.is_none() {
            errors.push(MonoVerifyError {
                func_idx: 0,
                block: None,
                stmt: None,
                message: format!("MonoStruct[{i}] ({:?}) missing layout", s.source),
            });
        }
    }

    // Check all enums have layouts
    for (i, e) in module.enums.iter().enumerate() {
        if e.type_info.layout.is_none() {
            errors.push(MonoVerifyError {
                func_idx: 0,
                block: None,
                stmt: None,
                message: format!("MonoEnum[{i}] ({:?}) missing layout", e.source),
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
    // body must be present unless extern
    if func.body.is_none() && func.extern_info.is_none() {
        errors.push(MonoVerifyError {
            func_idx: fi,
            block: None,
            stmt: None,
            message: format!("MonoFunction '{}' has no body and no extern_info", func.name),
        });
        return;
    }

    // Check param types
    for (pi, param) in func.params.iter().enumerate() {
        check_type_concrete(module, fi, None, None, param.ty, errors, &format!("param {pi}"));
    }
    check_type_concrete(module, fi, None, None, func.ret, errors, "return type");

    let Some(body) = &func.body else { return };

    // Check local types
    for (li, local) in body.locals.iter().enumerate() {
        check_type_concrete(module, fi, None, None, local.ty, errors, &format!("local {li}"));
    }

    // Walk blocks
    let func_count = module.functions.len();
    for (bi, block) in body.blocks.iter().enumerate() {
        let block_id = BlockId::new(bi);

        for (si, stmt) in block.stmts.iter().enumerate() {
            match &stmt.kind {
                StatementKind::Call { callee, args, .. } => {
                    check_callee(fi, block_id, si, callee, func_count, errors);
                    for (op, _) in args {
                        check_operand(module, fi, block_id, si, op, func_count, errors);
                    }
                }
                StatementKind::Assign { rvalue, .. } => {
                    check_rvalue(module, fi, block_id, si, rvalue, func_count, errors);
                }
                StatementKind::Drop { .. } => {
                    errors.push(MonoVerifyError {
                        func_idx: fi,
                        block: Some(block_id),
                        stmt: Some(si),
                        message: "Drop statement not expanded".into(),
                    });
                }
                StatementKind::DropIf { .. } => {
                    errors.push(MonoVerifyError {
                        func_idx: fi,
                        block: Some(block_id),
                        stmt: Some(si),
                        message: "DropIf statement not expanded".into(),
                    });
                }
                StatementKind::SetDropFlag { .. } => {
                    errors.push(MonoVerifyError {
                        func_idx: fi,
                        block: Some(block_id),
                        stmt: Some(si),
                        message: "SetDropFlag statement not expanded".into(),
                    });
                }
                StatementKind::ScopeLive(_) => {}
            }
        }

        // Check terminator operands
        check_terminator_operands(module, fi, block_id, &block.terminator, func_count, errors);
    }
}

fn check_callee(
    fi: usize,
    block: BlockId,
    si: usize,
    callee: &Callee,
    func_count: usize,
    errors: &mut Vec<MonoVerifyError>,
) {
    match callee {
        Callee::Direct { .. } => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block: Some(block),
                stmt: Some(si),
                message: "Callee::Direct not resolved to Callee::Resolved".into(),
            });
        }
        Callee::Witness { .. } => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block: Some(block),
                stmt: Some(si),
                message: "Callee::Witness not resolved".into(),
            });
        }
        Callee::Resolved(id) => {
            if id.index() >= func_count {
                errors.push(MonoVerifyError {
                    func_idx: fi,
                    block: Some(block),
                    stmt: Some(si),
                    message: format!(
                        "Callee::Resolved({}) out of bounds ({})",
                        id.index(),
                        func_count
                    ),
                });
            }
        }
        Callee::Thin(_) | Callee::Thick(_) => {}
    }
}

fn check_operand(
    module: &MonoModule,
    fi: usize,
    block: BlockId,
    si: usize,
    operand: &Operand,
    func_count: usize,
    errors: &mut Vec<MonoVerifyError>,
) {
    if let Operand::Const(imm) = operand {
        check_immediate(fi, block, si, &imm.kind, func_count, errors);
        // Check types inside immediates
        match &imm.kind {
            ImmediateKind::FunctionRef { .. } => {
                errors.push(MonoVerifyError {
                    func_idx: fi,
                    block: Some(block),
                    stmt: Some(si),
                    message: "ImmediateKind::FunctionRef not resolved to MonoFunctionRef".into(),
                });
            }
            ImmediateKind::MonoFunctionRef(id) => {
                if id.index() >= func_count {
                    errors.push(MonoVerifyError {
                        func_idx: fi,
                        block: Some(block),
                        stmt: Some(si),
                        message: format!(
                            "MonoFunctionRef({}) out of bounds ({})",
                            id.index(),
                            func_count
                        ),
                    });
                }
            }
            ImmediateKind::SizeOf(ty) | ImmediateKind::AlignOf(ty) | ImmediateKind::NullPtr(ty) => {
                check_type_concrete(module, fi, Some(block), Some(si), *ty, errors, "immediate type");
            }
            _ => {}
        }
    }
}

fn check_immediate(
    _fi: usize,
    _block: BlockId,
    _si: usize,
    _kind: &ImmediateKind,
    _func_count: usize,
    _errors: &mut Vec<MonoVerifyError>,
) {
    // FunctionRef and MonoFunctionRef checks are handled in check_operand
}

fn check_rvalue(
    module: &MonoModule,
    fi: usize,
    block: BlockId,
    si: usize,
    rvalue: &crate::statement::Rvalue,
    func_count: usize,
    errors: &mut Vec<MonoVerifyError>,
) {
    use crate::statement::Rvalue;
    match rvalue {
        Rvalue::Use(op, _) => check_operand(module, fi, block, si, op, func_count, errors),
        Rvalue::Construct { ty, fields, .. } => {
            check_type_concrete(module, fi, Some(block), Some(si), *ty, errors, "Construct type");
            for (_, op, _) in fields {
                check_operand(module, fi, block, si, op, func_count, errors);
            }
        }
        Rvalue::EnumVariant { enum_ty, payload, .. } => {
            check_type_concrete(module, fi, Some(block), Some(si), *enum_ty, errors, "EnumVariant type");
            for (op, _) in payload {
                check_operand(module, fi, block, si, op, func_count, errors);
            }
        }
        Rvalue::ArrayLiteral { element_ty, values, .. } => {
            check_type_concrete(module, fi, Some(block), Some(si), *element_ty, errors, "ArrayLiteral type");
            for (op, _) in values {
                check_operand(module, fi, block, si, op, func_count, errors);
            }
        }
        Rvalue::Tuple(elems) => {
            for (op, _) in elems {
                check_operand(module, fi, block, si, op, func_count, errors);
            }
        }
        Rvalue::ApplyPartial { captures, .. } => {
            for (op, _) in captures {
                check_operand(module, fi, block, si, op, func_count, errors);
            }
        }
        Rvalue::Op1 { arg, .. } => check_operand(module, fi, block, si, arg, func_count, errors),
        Rvalue::Op2 { lhs, rhs, .. } => {
            check_operand(module, fi, block, si, lhs, func_count, errors);
            check_operand(module, fi, block, si, rhs, func_count, errors);
        }
        Rvalue::Op3 { a, b, c, .. } => {
            check_operand(module, fi, block, si, a, func_count, errors);
            check_operand(module, fi, block, si, b, func_count, errors);
            check_operand(module, fi, block, si, c, func_count, errors);
        }
        Rvalue::Ref(_) | Rvalue::RefMut(_) => {}
    }
}

fn check_terminator_operands(
    module: &MonoModule,
    fi: usize,
    block: BlockId,
    terminator: &crate::terminator::Terminator,
    func_count: usize,
    errors: &mut Vec<MonoVerifyError>,
) {
    use crate::terminator::TerminatorKind;
    match &terminator.kind {
        TerminatorKind::Return(op) => {
            check_operand(module, fi, block, 0, op, func_count, errors);
        }
        TerminatorKind::Branch { condition, .. } => {
            check_operand(module, fi, block, 0, condition, func_count, errors);
        }
        _ => {}
    }
}

fn check_type_concrete(
    module: &MonoModule,
    fi: usize,
    block: Option<BlockId>,
    stmt: Option<usize>,
    ty: TyId,
    errors: &mut Vec<MonoVerifyError>,
    context: &str,
) {
    match module.ty_arena.get(ty) {
        MirTy::TypeParam(e) => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block,
                stmt,
                message: format!("TypeParam({e:?}) in {context}"),
            });
        }
        MirTy::SelfType => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block,
                stmt,
                message: format!("SelfType in {context}"),
            });
        }
        MirTy::AssociatedProjection { .. } => {
            errors.push(MonoVerifyError {
                func_idx: fi,
                block,
                stmt,
                message: format!("AssociatedProjection in {context}"),
            });
        }
        MirTy::Pointer(inner) => {
            check_type_concrete(module, fi, block, stmt, *inner, errors, context);
        }
        MirTy::Tuple(elems) => {
            for &elem in elems {
                check_type_concrete(module, fi, block, stmt, elem, errors, context);
            }
        }
        MirTy::Named { type_args, .. } => {
            for &arg in type_args {
                check_type_concrete(module, fi, block, stmt, arg, errors, context);
            }
        }
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            for (p, _) in params {
                check_type_concrete(module, fi, block, stmt, *p, errors, context);
            }
            check_type_concrete(module, fi, block, stmt, *ret, errors, context);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{BasicBlock, LocalDef, MirBody};
    use crate::immediate::Immediate;
    use crate::item::function::ExternInfo;
    use crate::item::{Layout, TypeInfo};
    use crate::layout::StructLayout;
    use crate::mono::types::{MonoFunction, MonoModule, MonoParam, MonoStruct};
    use crate::operand::UseMode;
    use crate::place::Place;
    use crate::MonoFuncId;
    use crate::statement::{Rvalue, Statement};
    use crate::terminator::{Terminator, TerminatorKind};
    use crate::ty::{ParamConvention, TyArena};
    use crate::{BlockId, LocalId};
    use indexmap::IndexMap;
    use kestrel_hecs::Entity;

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    fn simple_body(arena: &mut TyArena) -> MirBody {
        let unit = arena.unit();
        let block = BasicBlock {
            stmts: vec![],
            terminator: Terminator {
                kind: TerminatorKind::Return(Operand::Const(Immediate::unit())),
                span: None,
            },
        };
        MirBody {
            locals: vec![LocalDef {
                name: "_ret".into(),
                ty: unit,
            }],
            blocks: vec![block],
            param_count: 0,
            entry: BlockId::new(0),
            local_scopes: std::collections::HashMap::new(),
            failure_return_blocks: std::collections::HashSet::new(),
        }
    }

    fn make_module() -> MonoModule {
        let arena = TyArena::new();
        MonoModule::new(arena, IndexMap::new())
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
        module.structs.push(MonoStruct {
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
        let block = BasicBlock {
            stmts: vec![Statement {
                kind: StatementKind::Call {
                    dest: None,
                    callee: Callee::Direct {
                        func: entity(99),
                        type_args: vec![],
                        self_type: None,
                    },
                    args: vec![],
                },
                span: None,
            }],
            terminator: Terminator {
                kind: TerminatorKind::Return(Operand::Const(Immediate::unit())),
                span: None,
            },
        };
        let body = MirBody {
            locals: vec![LocalDef { name: "_ret".into(), ty: unit }],
            blocks: vec![block],
            param_count: 0,
            entry: BlockId::new(0),
            local_scopes: std::collections::HashMap::new(),
            failure_return_blocks: std::collections::HashSet::new(),
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
    fn verify_rejects_drop_statement() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let block = BasicBlock {
            stmts: vec![Statement {
                kind: StatementKind::Drop {
                    place: Place::local(LocalId::new(0)),
                },
                span: None,
            }],
            terminator: Terminator {
                kind: TerminatorKind::Return(Operand::Const(Immediate::unit())),
                span: None,
            },
        };
        let body = MirBody {
            locals: vec![LocalDef { name: "x".into(), ty: unit }],
            blocks: vec![block],
            param_count: 0,
            entry: BlockId::new(0),
            local_scopes: std::collections::HashMap::new(),
            failure_return_blocks: std::collections::HashSet::new(),
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
        assert!(result.errors.iter().any(|e| e.message.contains("Drop")));
    }

    #[test]
    fn verify_rejects_function_ref() {
        let mut module = make_module();
        let unit = module.ty_arena.unit();
        let block = BasicBlock {
            stmts: vec![Statement {
                kind: StatementKind::Assign {
                    dest: Place::local(LocalId::new(0)),
                    rvalue: Rvalue::Use(
                        Operand::Const(Immediate::new(ImmediateKind::FunctionRef {
                            func: entity(99),
                            type_args: vec![],
                            self_type: None,
                        })),
                        UseMode::Copy,
                    ),
                },
                span: None,
            }],
            terminator: Terminator {
                kind: TerminatorKind::Return(Operand::Const(Immediate::unit())),
                span: None,
            },
        };
        let body = MirBody {
            locals: vec![LocalDef { name: "f".into(), ty: unit }],
            blocks: vec![block],
            param_count: 0,
            entry: BlockId::new(0),
            local_scopes: std::collections::HashMap::new(),
            failure_return_blocks: std::collections::HashSet::new(),
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
        let block = BasicBlock {
            stmts: vec![Statement {
                kind: StatementKind::Call {
                    dest: None,
                    callee: Callee::Resolved(MonoFuncId::new(999)),
                    args: vec![],
                },
                span: None,
            }],
            terminator: Terminator {
                kind: TerminatorKind::Return(Operand::Const(Immediate::unit())),
                span: None,
            },
        };
        let body = MirBody {
            locals: vec![LocalDef { name: "_ret".into(), ty: unit }],
            blocks: vec![block],
            param_count: 0,
            entry: BlockId::new(0),
            local_scopes: std::collections::HashMap::new(),
            failure_return_blocks: std::collections::HashSet::new(),
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
        module.structs.push(MonoStruct {
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
}
