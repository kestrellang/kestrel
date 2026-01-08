//! BFS collection of instantiations.
//!
//! This module implements the collection phase of monomorphization.
//! It uses BFS to discover all concrete instantiations of generic
//! functions, structs, and enums needed for compilation.

use super::error::MonomorphizeError;
use super::instantiation::{
    EnumInstantiation, FunctionInstantiation, MonomorphizationSet, StructInstantiation,
};
use super::substitute::{Substitution, build_substitution};
use super::witness;
use kestrel_execution_graph::{
    Callee, Function, Id, ImmediateKind, MirContext, MirTy, QualifiedName, Rvalue, StatementKind,
    Value,
};
use std::collections::{HashMap, VecDeque};

/// Collect all instantiations needed for compilation.
///
/// This runs BFS starting from non-generic functions, discovering
/// all generic instantiations that need to be compiled. During collection,
/// new types may be interned into the MirContext as substitutions are applied.
///
/// Returns a `MonomorphizationSet` containing all discovered instantiations,
/// or a list of errors if collection fails.
pub fn collect_all(mir: &mut MirContext) -> Result<MonomorphizationSet, Vec<MonomorphizeError>> {
    let mut ctx = CollectionContext::new(mir);
    ctx.collect()?;
    Ok(ctx.result)
}

/// Context for the collection phase.
struct CollectionContext<'a> {
    mir: &'a mut MirContext,
    /// Index: function name -> function id
    functions_by_name: HashMap<Id<QualifiedName>, Id<Function>>,
    /// The result set of instantiations
    result: MonomorphizationSet,
    /// Queue of function instantiations to process
    pending: VecDeque<FunctionInstantiation>,
    /// Accumulated errors
    errors: Vec<MonomorphizeError>,
}

impl<'a> CollectionContext<'a> {
    fn new(mir: &'a mut MirContext) -> Self {
        // Build function index
        let mut functions_by_name = HashMap::new();
        for (func_id, func_def) in mir.functions.iter() {
            functions_by_name.insert(func_def.name, func_id);
        }

        Self {
            mir,
            functions_by_name,
            result: MonomorphizationSet::new(),
            pending: VecDeque::new(),
            errors: Vec::new(),
        }
    }

    /// Run the collection algorithm.
    fn collect(&mut self) -> Result<(), Vec<MonomorphizeError>> {
        // Seed with non-generic functions
        self.seed_non_generic_functions();

        // BFS loop
        while let Some(inst) = self.pending.pop_front() {
            self.process_function_instantiation(&inst);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    /// Seed the queue with non-generic functions.
    fn seed_non_generic_functions(&mut self) {
        let non_generic: Vec<_> = self
            .mir
            .functions
            .iter()
            .filter(|(_, def)| def.type_params.is_empty())
            .map(|(id, _)| FunctionInstantiation::non_generic(id))
            .collect();

        for inst in non_generic {
            if self.result.add_function(inst.clone()) {
                self.pending.push_back(inst);
            }
        }
    }

    /// Process a single function instantiation.
    fn process_function_instantiation(&mut self, inst: &FunctionInstantiation) {
        // Get function definition (need to clone type_params to avoid borrow conflict)
        let func_def = &self.mir.functions[inst.func_id];
        let type_params = func_def.type_params.clone();
        let blocks = func_def.blocks.clone();
        let params = func_def.params.clone();
        let locals = func_def.locals.clone();
        let ret = func_def.ret;

        // Build substitution
        let subst = build_substitution(self.mir, &type_params, &inst.type_args);

        // Scan return type
        self.scan_type(ret, &subst);

        // Scan parameter types
        for &param_id in &params {
            let param = &self.mir.params[param_id];
            self.scan_type(param.ty, &subst);
        }

        // Scan local types
        for &local_id in &locals {
            let local = self.mir.local(local_id);
            self.scan_type(local.ty, &subst);
        }

        // Scan blocks
        for &block_id in &blocks {
            self.scan_block(block_id, &subst);
        }
    }

    /// Scan a block for instantiations.
    fn scan_block(&mut self, block_id: Id<kestrel_execution_graph::Block>, subst: &Substitution) {
        let block = self.mir.block(block_id);
        let statements = block.statements.clone();
        let terminator = block.terminator.clone();

        for &stmt_id in &statements {
            let stmt = self.mir.statement(stmt_id);
            let kind = stmt.kind.clone();
            self.scan_statement(&kind, subst);
        }

        // Scan terminator if present
        if let Some(ref term) = terminator {
            self.scan_terminator(term, subst);
        }
    }

    /// Scan a statement for instantiations.
    fn scan_statement(&mut self, stmt: &StatementKind, subst: &Substitution) {
        match stmt {
            StatementKind::Assign { dest: _, rvalue } => {
                self.scan_rvalue(rvalue, subst);
            }
            StatementKind::Call { callee, args } => {
                self.scan_callee(callee, subst);
                for arg in args {
                    self.scan_value(&arg.value, subst);
                }
            }
            StatementKind::Deinit { place: _ } => {}
            StatementKind::DeinitIf { place: _, flag: _ } => {}
            StatementKind::SetDeinitFlag { flag: _, value: _ } => {}
        }
    }

    /// Scan an rvalue for instantiations.
    fn scan_rvalue(&mut self, rvalue: &Rvalue, subst: &Substitution) {
        match rvalue {
            Rvalue::Move(_) | Rvalue::Copy(_) | Rvalue::Ref(_) | Rvalue::RefMut(_) => {}

            Rvalue::Use(imm) => {
                self.scan_immediate(&imm.kind, subst);
            }

            Rvalue::BinaryOp { lhs, rhs, .. } => {
                self.scan_value(lhs, subst);
                self.scan_value(rhs, subst);
            }

            Rvalue::UnaryOp { operand, .. } => {
                self.scan_value(operand, subst);
            }

            Rvalue::Construct { ty, fields } => {
                self.scan_type(*ty, subst);
                for (_, value) in fields {
                    self.scan_value(value, subst);
                }
            }

            Rvalue::Tuple(elements) => {
                for elem in elements {
                    self.scan_value(elem, subst);
                }
            }

            Rvalue::Array {
                element_ty,
                elements,
            } => {
                self.scan_type(*element_ty, subst);
                for elem in elements {
                    self.scan_value(elem, subst);
                }
            }

            Rvalue::EnumVariant {
                enum_ty, payload, ..
            } => {
                self.scan_type(*enum_ty, subst);
                for val in payload {
                    self.scan_value(val, subst);
                }
            }

            Rvalue::Call { callee, args } => {
                self.scan_callee(callee, subst);
                for arg in args {
                    self.scan_value(&arg.value, subst);
                }
            }

            Rvalue::Cast {
                operand, target, ..
            } => {
                self.scan_value(operand, subst);
                self.scan_type(*target, subst);
            }

            Rvalue::StrPtr(v)
            | Rvalue::StrLen(v)
            | Rvalue::IntToString(v)
            | Rvalue::PtrToRef(v)
            | Rvalue::PtrToRefMut(v)
            | Rvalue::RefToPtr(v) => {
                self.scan_value(v, subst);
            }

            Rvalue::StrFromParts { ptr, len } => {
                self.scan_value(ptr, subst);
                self.scan_value(len, subst);
            }

            Rvalue::PtrOffset { ptr, offset } => {
                self.scan_value(ptr, subst);
                self.scan_value(offset, subst);
            }

            Rvalue::FuncToEscaping(name) => {
                // Non-generic function reference
                if let Some(&func_id) = self.functions_by_name.get(name) {
                    let inst = FunctionInstantiation::non_generic(func_id);
                    if self.result.add_function(inst.clone()) {
                        self.pending.push_back(inst);
                    }
                }
            }

            Rvalue::ApplyPartial { func, captures } => {
                // Non-generic function reference
                if let Some(&func_id) = self.functions_by_name.get(func) {
                    let inst = FunctionInstantiation::non_generic(func_id);
                    if self.result.add_function(inst.clone()) {
                        self.pending.push_back(inst);
                    }
                }
                for cap in captures {
                    self.scan_value(cap, subst);
                }
            }
        }
    }

    /// Scan a callee for instantiations.
    fn scan_callee(&mut self, callee: &Callee, subst: &Substitution) {
        match callee {
            Callee::Direct { name, type_args } => {
                // Apply substitution to type args
                let concrete_args: Vec<_> = type_args
                    .iter()
                    .map(|ty| subst.apply_ty(self.mir, *ty))
                    .collect();

                // Record function instantiation
                if let Some(&func_id) = self.functions_by_name.get(name) {
                    let inst = FunctionInstantiation::new(func_id, concrete_args);
                    if self.result.add_function(inst.clone()) {
                        self.pending.push_back(inst);
                    }
                } else {
                    self.errors
                        .push(MonomorphizeError::FunctionNotFound { name: *name });
                }
            }

            Callee::Thin(_) | Callee::Thick(_) => {
                // Function pointer calls - we can't statically know what's being called
            }

            Callee::Witness {
                protocol,
                method,
                for_type,
            } => {
                // Apply substitution to for_type
                let concrete_for_type = subst.apply_ty(self.mir, *for_type);

                // Resolve the witness to find the actual implementation
                match witness::resolve_witness(self.mir, *protocol, method, concrete_for_type) {
                    Ok((impl_name, impl_type_args)) => {
                        // Record the implementation function instantiation
                        if let Some(&func_id) = self.functions_by_name.get(&impl_name) {
                            let inst = FunctionInstantiation::new(func_id, impl_type_args);
                            if self.result.add_function(inst.clone()) {
                                self.pending.push_back(inst);
                            }
                        } else {
                            self.errors
                                .push(MonomorphizeError::FunctionNotFound { name: impl_name });
                        }
                    }
                    Err(e) => {
                        self.errors.push(e);
                    }
                }
            }
        }
    }

    /// Scan an immediate for instantiations.
    fn scan_immediate(&mut self, imm: &ImmediateKind, subst: &Substitution) {
        match imm {
            ImmediateKind::FunctionRef { name, type_args } => {
                // Apply substitution to type args
                let concrete_args: Vec<_> = type_args
                    .iter()
                    .map(|ty| subst.apply_ty(self.mir, *ty))
                    .collect();

                // Record function instantiation
                if let Some(&func_id) = self.functions_by_name.get(name) {
                    let inst = FunctionInstantiation::new(func_id, concrete_args);
                    if self.result.add_function(inst.clone()) {
                        self.pending.push_back(inst);
                    }
                } else {
                    self.errors
                        .push(MonomorphizeError::FunctionNotFound { name: *name });
                }
            }

            ImmediateKind::WitnessMethod {
                protocol,
                method,
                for_type,
            } => {
                // Apply substitution to for_type
                let concrete_for_type = subst.apply_ty(self.mir, *for_type);

                // Resolve the witness
                match witness::resolve_witness(self.mir, *protocol, method, concrete_for_type) {
                    Ok((impl_name, impl_type_args)) => {
                        if let Some(&func_id) = self.functions_by_name.get(&impl_name) {
                            let inst = FunctionInstantiation::new(func_id, impl_type_args);
                            if self.result.add_function(inst.clone()) {
                                self.pending.push_back(inst);
                            }
                        } else {
                            self.errors
                                .push(MonomorphizeError::FunctionNotFound { name: impl_name });
                        }
                    }
                    Err(e) => {
                        self.errors.push(e);
                    }
                }
            }

            ImmediateKind::NullPtr(ty) => {
                self.scan_type(*ty, subst);
            }

            // Literals don't contain instantiations
            ImmediateKind::IntLiteral { .. }
            | ImmediateKind::FloatLiteral { .. }
            | ImmediateKind::BoolLiteral(_)
            | ImmediateKind::StringLiteral(_)
            | ImmediateKind::Unit
            | ImmediateKind::Error => {}
        }
    }

    /// Scan a value for instantiations.
    fn scan_value(&mut self, value: &Value, subst: &Substitution) {
        match value {
            Value::Place(_) => {}
            Value::Immediate(imm) => {
                self.scan_immediate(&imm.kind, subst);
            }
        }
    }

    /// Scan a type for struct/enum instantiations.
    fn scan_type(&mut self, ty: Id<kestrel_execution_graph::Ty>, subst: &Substitution) {
        // Apply substitution first
        let concrete_ty = subst.apply_ty(self.mir, ty);

        // Now scan the concrete type
        let mir_ty = self.mir.ty(concrete_ty).clone();
        match mir_ty {
            MirTy::Named { name, type_args } => {
                if !type_args.is_empty() {
                    // This is a generic instantiation - determine if it's a struct or enum
                    // Check structs first
                    for (struct_id, struct_def) in self.mir.structs.iter() {
                        if struct_def.name == name {
                            let inst = StructInstantiation::new(struct_id, type_args.clone());
                            self.result.add_struct(inst);
                            break;
                        }
                    }

                    // Check enums
                    for (enum_id, enum_def) in self.mir.enums.iter() {
                        if enum_def.name == name {
                            let inst = EnumInstantiation::new(enum_id, type_args.clone());
                            self.result.add_enum(inst);
                            break;
                        }
                    }
                }

                // Recurse into type args
                for arg in &type_args {
                    self.scan_type(*arg, &Substitution::new());
                }
            }

            MirTy::Pointer(inner)
            | MirTy::Ref(inner)
            | MirTy::RefMut(inner)
            | MirTy::Array(inner) => {
                self.scan_type(inner, &Substitution::new());
            }

            MirTy::Tuple(elems) => {
                for elem in elems {
                    self.scan_type(elem, &Substitution::new());
                }
            }

            MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
                for param in params {
                    self.scan_type(param, &Substitution::new());
                }
                self.scan_type(ret, &Substitution::new());
            }

            MirTy::AssociatedTypeProjection { base, .. } => {
                self.scan_type(base, &Substitution::new());
            }

            // Primitives and type params don't contain nested instantiations
            MirTy::I8
            | MirTy::I16
            | MirTy::I32
            | MirTy::I64
            | MirTy::F16
            | MirTy::F32
            | MirTy::F64
            | MirTy::Bool
            | MirTy::Unit
            | MirTy::Never
            | MirTy::Str
            | MirTy::TypeParam(_)
            | MirTy::SelfType
            | MirTy::Error => {}
        }
    }

    /// Scan a terminator for instantiations.
    fn scan_terminator(
        &mut self,
        term: &kestrel_execution_graph::Terminator,
        subst: &Substitution,
    ) {
        use kestrel_execution_graph::TerminatorKind;

        match &term.kind {
            TerminatorKind::Return(value) => {
                self.scan_value(value, subst);
            }
            TerminatorKind::Jump(_) => {}
            TerminatorKind::Branch { condition, .. } => {
                self.scan_value(condition, subst);
            }
            TerminatorKind::Switch { .. } => {}
            TerminatorKind::Panic(_) => {}
            TerminatorKind::Unreachable => {}
        }
    }
}
