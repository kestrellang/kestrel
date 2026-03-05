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
    Ty, TypeParam, Value,
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
        // Collect non-generic function IDs first to avoid borrow issues
        let non_generic_ids: Vec<_> = self
            .mir
            .functions
            .iter()
            .filter(|(_, def)| def.type_params.is_empty())
            .map(|(id, _)| id)
            .collect();

        for func_id in non_generic_ids {
            let func_def = &self.mir.functions[func_id];

            // Skip functions that need Self type - they'll be processed when called with concrete types
            let needs_self_type = func_def.params.iter().any(|&param_id| {
                let param = &self.mir.params[param_id];
                self.type_needs_self(self.mir.ty(param.ty))
            }) || self.type_needs_self(self.mir.ty(func_def.ret));

            if needs_self_type {
                continue;
            }

            let inst = FunctionInstantiation::non_generic(func_id);
            if self.result.add_function(inst.clone()) {
                self.pending.push_back(inst);
            }
        }
    }

    /// Process a single function instantiation.
    fn process_function_instantiation(&mut self, inst: &FunctionInstantiation) {
        // Get function definition (need to clone type_params to avoid borrow conflict)
        let func_def = &self.mir.functions[inst.func_id];
        let func_name = self.mir.name(func_def.name).to_string();
        let type_params = func_def.type_params.clone();
        let blocks = func_def.blocks.clone();
        let params = func_def.params.clone();
        let locals = func_def.locals.clone();
        let ret = func_def.ret;

        // Build substitution. Some method instantiations rely on self_type and do not
        // carry explicit type_args for owner generics (e.g. Buffer[T, A].deinit).
        let mut effective_type_args = inst.type_args.clone();
        if type_params.len() != effective_type_args.len()
            && effective_type_args.is_empty()
            && let Some(st) = inst.self_type
            && let Some(inferred_args) =
                Self::extract_named_type_args_from_self_type(self.mir, st, type_params.len())
        {
            effective_type_args = inferred_args;
        }

        if type_params.len() != effective_type_args.len() {
            eprintln!("MISMATCH in function: {}", func_name);
            eprintln!("  type_params: {:?}", type_params);
            eprintln!("  type_args: {:?}", effective_type_args);
        }
        let mut subst = build_substitution(self.mir, &type_params, &effective_type_args);

        // Set self_type if this instantiation has one (protocol extension methods)
        if let Some(st) = inst.self_type {
            subst.set_self_type(st);
        } else {
            // Check if this function needs a self_type but doesn't have one
            // This happens when protocol extension methods are seeded as non-generic
            // We skip them here - they'll be processed later when called with concrete types
            let needs_self_type = params.iter().any(|&param_id| {
                let param = &self.mir.params[param_id];
                self.type_needs_self(self.mir.ty(param.ty))
            }) || self.type_needs_self(self.mir.ty(ret));

            if needs_self_type {
                // Skip this instantiation - it will be processed when called with a concrete type
                return;
            }
        }

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

    fn extract_named_type_args_from_self_type(
        mir: &MirContext,
        self_ty: Id<kestrel_execution_graph::Ty>,
        expected_len: usize,
    ) -> Option<Vec<Id<kestrel_execution_graph::Ty>>> {
        let mut current = self_ty;
        loop {
            match mir.ty(current) {
                MirTy::Ref(inner) | MirTy::RefMut(inner) | MirTy::Pointer(inner) => {
                    current = *inner;
                },
                MirTy::Named { type_args, .. } if type_args.len() == expected_len => {
                    return Some(type_args.clone());
                },
                _ => return None,
            }
        }
    }

    fn infer_type_args_from_subst(
        &mut self,
        type_params: &[Id<TypeParam>],
        subst: &Substitution,
    ) -> Vec<Id<Ty>> {
        type_params
            .iter()
            .map(|&tp| {
                let tp_ty = self.mir.intern_type(MirTy::TypeParam(tp));
                subst.apply_ty(self.mir, tp_ty)
            })
            .collect()
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
            },
            StatementKind::Call { callee, args } => {
                // For protocol extension method calls, we need to track Self type.
                // Check if this is a direct call to a function with Self-typed parameters.
                let self_type = self.infer_self_type_from_call(callee, args, subst);
                let mut call_subst = subst.clone();
                if let Some(st) = self_type {
                    call_subst.set_self_type(st);
                }
                self.scan_callee(callee, &call_subst);
                for arg in args {
                    self.scan_value(&arg.value, subst);
                }
            },
            StatementKind::Deinit { place: _ } => {},
            StatementKind::DeinitIf { place: _, flag: _ } => {},
            StatementKind::SetDeinitFlag { flag: _, value: _ } => {},
        }
    }

    /// Scan an rvalue for instantiations.
    fn scan_rvalue(&mut self, rvalue: &Rvalue, subst: &Substitution) {
        match rvalue {
            Rvalue::Move(_) | Rvalue::Copy(_) | Rvalue::Ref(_) | Rvalue::RefMut(_) => {},

            Rvalue::Use(imm) => {
                self.scan_immediate(&imm.kind, subst);
            },

            Rvalue::BinaryOp { lhs, rhs, .. } => {
                self.scan_value(lhs, subst);
                self.scan_value(rhs, subst);
            },

            Rvalue::UnaryOp { operand, .. } => {
                self.scan_value(operand, subst);
            },

            Rvalue::Construct { ty, fields } => {
                self.scan_type(*ty, subst);
                for (_, value) in fields {
                    self.scan_value(value, subst);
                }
            },

            Rvalue::Tuple(elements) => {
                for elem in elements {
                    self.scan_value(elem, subst);
                }
            },

            Rvalue::StackAlloc { element_ty, count } => {
                self.scan_type(*element_ty, subst);
                self.scan_value(count, subst);
            },

            Rvalue::EnumVariant {
                enum_ty, payload, ..
            } => {
                self.scan_type(*enum_ty, subst);
                for val in payload {
                    self.scan_value(val, subst);
                }
            },

            Rvalue::Call { callee, args } => {
                // For protocol extension method calls, we need to track Self type.
                // Check if this is a direct call to a function with Self-typed parameters.
                let self_type = self.infer_self_type_from_call(callee, args, subst);
                let mut call_subst = subst.clone();
                if let Some(st) = self_type {
                    call_subst.set_self_type(st);
                }
                self.scan_callee(callee, &call_subst);
                for arg in args {
                    self.scan_value(&arg.value, subst);
                }
            },

            Rvalue::Cast {
                operand, target, ..
            } => {
                self.scan_value(operand, subst);
                self.scan_type(*target, subst);
            },

            Rvalue::StrPtr(v)
            | Rvalue::StrLen(v)
            | Rvalue::IntToString(v)
            | Rvalue::RefToPtr(v) => {
                self.scan_value(v, subst);
            },

            Rvalue::PtrOffset { ptr, offset } => {
                self.scan_value(ptr, subst);
                self.scan_value(offset, subst);
            },

            Rvalue::ApplyPartial { func, captures } => {
                // Function reference (closure or partial application)
                if let Some(&func_id) = self.functions_by_name.get(func) {
                    // Clone the data we need to avoid borrow conflicts
                    let func_def = &self.mir.functions[func_id];
                    let type_params = func_def.type_params.clone();
                    let params = func_def.params.clone();
                    let ret = func_def.ret;

                    // Closures inherit type parameters from their parent function.
                    // When we see ApplyPartial for a closure inside a generic function,
                    // the closure will have type_params matching the parent's.
                    // We need to instantiate the closure with the same type args
                    // that the parent was instantiated with (from the current substitution).
                    let inst = if !type_params.is_empty() {
                        // Get type args by applying current substitution to the closure's type params
                        let type_args: Vec<_> = type_params
                            .iter()
                            .map(|&tp| {
                                // The closure's type param should be the same MIR ID as the parent's.
                                // Look it up in the substitution to get the concrete type.
                                let tp_ty = self.mir.intern_type(MirTy::TypeParam(tp));
                                subst.apply_ty(self.mir, tp_ty)
                            })
                            .collect();

                        // Check if closure needs self_type
                        let needs_self = params.iter().any(|&param_id| {
                            let param = &self.mir.params[param_id];
                            self.type_needs_self(self.mir.ty(param.ty))
                        }) || self.type_needs_self(self.mir.ty(ret));

                        if needs_self {
                            if let Some(st) = subst.get_self_type() {
                                FunctionInstantiation::with_self_type(func_id, type_args, st)
                            } else {
                                // Skip - will be processed later when called with concrete type
                                for cap in captures {
                                    self.scan_value(cap, subst);
                                }
                                return;
                            }
                        } else {
                            FunctionInstantiation::new(func_id, type_args)
                        }
                    } else {
                        // Non-generic function
                        let needs_self = params.iter().any(|&param_id| {
                            let param = &self.mir.params[param_id];
                            self.type_needs_self(self.mir.ty(param.ty))
                        }) || self.type_needs_self(self.mir.ty(ret));

                        if needs_self {
                            // Non-generic closures can still need Self type if they're defined in
                            // a protocol extension method. Try to use self_type from substitution.
                            if let Some(st) = subst.get_self_type() {
                                FunctionInstantiation::with_self_type(func_id, Vec::new(), st)
                            } else {
                                self.errors
                                    .push(MonomorphizeError::UnsupportedFunctionReference {
                                        name: *func,
                                        reason: "function reference requires Self type".to_string(),
                                    });
                                return;
                            }
                        } else {
                            FunctionInstantiation::non_generic(func_id)
                        }
                    };

                    if self.result.add_function(inst.clone()) {
                        self.pending.push_back(inst);
                    }
                }
                for cap in captures {
                    self.scan_value(cap, subst);
                }
            },

            // Float intrinsics
            Rvalue::FloatConst { .. } => {},

            Rvalue::FloatPred { operand, .. } => {
                self.scan_value(operand, subst);
            },

            Rvalue::FloatMath { operand, .. } => {
                self.scan_value(operand, subst);
            },

            Rvalue::FloatFma { a, b, c, .. } => {
                self.scan_value(a, subst);
                self.scan_value(b, subst);
                self.scan_value(c, subst);
            },

            Rvalue::FloatCopysign {
                magnitude,
                sign_source,
                ..
            } => {
                self.scan_value(magnitude, subst);
                self.scan_value(sign_source, subst);
            },

            // Pointer intrinsics
            Rvalue::PtrNull { ty } | Rvalue::SizeOf { ty } | Rvalue::AlignOf { ty } => {
                self.scan_type(*ty, subst);
            },
            Rvalue::PtrFromAddress { ty, address } => {
                self.scan_type(*ty, subst);
                self.scan_value(address, subst);
            },
            Rvalue::PtrToAddress { ptr } | Rvalue::PtrIsNull { ptr } => {
                self.scan_value(ptr, subst);
            },
            Rvalue::PtrRead { ptr, ty } => {
                self.scan_value(ptr, subst);
                self.scan_type(*ty, subst);
            },
            Rvalue::PtrWrite { ptr, value } => {
                self.scan_value(ptr, subst);
                self.scan_value(value, subst);
            },
            Rvalue::PtrCast { ptr, target_ty } => {
                self.scan_value(ptr, subst);
                self.scan_type(*target_ty, subst);
            },

            // Boolean (i1) intrinsics
            Rvalue::I1Eq { lhs, rhs } | Rvalue::I1And { lhs, rhs } | Rvalue::I1Or { lhs, rhs } => {
                self.scan_value(lhs, subst);
                self.scan_value(rhs, subst);
            },
            Rvalue::I1Not { operand } => {
                self.scan_value(operand, subst);
            },

            // Atomic intrinsics
            Rvalue::AtomicAdd { ptr, delta } | Rvalue::AtomicSub { ptr, delta } => {
                self.scan_value(ptr, subst);
                self.scan_value(delta, subst);
            },

        }
    }

    /// Scan a callee for instantiations.
    fn scan_callee(&mut self, callee: &Callee, subst: &Substitution) {
        match callee {
            Callee::Direct { name, type_args } => {
                // Apply substitution to type args
                let mut concrete_args: Vec<_> = type_args
                    .iter()
                    .map(|ty| subst.apply_ty(self.mir, *ty))
                    .collect();

                // Record function instantiation (only include self_type if callee needs it)
                if let Some(&func_id) = self.functions_by_name.get(name) {
                    let (callee_type_params, callee_params, callee_ret) = {
                        let callee_def = &self.mir.functions[func_id];
                        (
                            callee_def.type_params.clone(),
                            callee_def.params.clone(),
                            callee_def.ret,
                        )
                    };

                    // Check if the callee actually uses Self in its signature
                    if concrete_args.is_empty() && !callee_type_params.is_empty() {
                        concrete_args = self.infer_type_args_from_subst(&callee_type_params, subst);
                        let unresolved = concrete_args
                            .iter()
                            .any(|&ty| matches!(self.mir.ty(ty), MirTy::TypeParam(_)));
                        if unresolved
                            && let Some(st) = subst.get_self_type()
                            && let Some(from_self) = Self::extract_named_type_args_from_self_type(
                                self.mir,
                                st,
                                callee_type_params.len(),
                            )
                        {
                            concrete_args = from_self;
                        }
                    }
                    let callee_needs_self = callee_params.iter().any(|&param_id| {
                        let param = &self.mir.params[param_id];
                        self.type_needs_self(self.mir.ty(param.ty))
                    }) || self.type_needs_self(self.mir.ty(callee_ret));

                    let inst = if callee_needs_self {
                        let st = if let Some(st) = subst.get_self_type() {
                            st
                        } else if let Some(st) = self.infer_self_type_from_method_name(*name) {
                            // Try to infer Self from the method's containing type
                            // e.g., Test.Widget.create -> Self = Test.Widget
                            st
                        } else {
                            // Callee needs Self but we can't infer it - skip for now,
                            // will be processed later when called with concrete type
                            return;
                        };
                        FunctionInstantiation::with_self_type(func_id, concrete_args, st)
                    } else {
                        FunctionInstantiation::new(func_id, concrete_args)
                    };
                    if self.result.add_function(inst.clone()) {
                        self.pending.push_back(inst);
                    }
                } else {
                    self.errors
                        .push(MonomorphizeError::FunctionNotFound { name: *name });
                }
            },

            Callee::Thin(_) | Callee::Thick(_) => {
                // Function pointer calls - we can't statically know what's being called
            },

            Callee::Witness {
                protocol,
                method,
                for_type,
                method_type_args,
            } => {
                // Apply substitution to for_type
                let concrete_for_type = subst.apply_ty(self.mir, *for_type);

                // Apply substitution to method_type_args (the method's own type parameters)
                let concrete_method_type_args: Vec<_> = method_type_args
                    .iter()
                    .map(|ty| subst.apply_ty(self.mir, *ty))
                    .collect();

                // Resolve the witness to find the actual implementation
                match witness::resolve_witness(self.mir, *protocol, method, concrete_for_type) {
                    Ok((impl_name, mut impl_type_args)) => {
                        // Append the method's own type arguments (e.g., H in hash[H])
                        // to the witness type args (e.g., K, V from Dictionary[K, V, H])
                        impl_type_args.extend(concrete_method_type_args);

                        // Record the implementation function instantiation
                        // For protocol extension methods, set self_type to concrete_for_type
                        // so that MirTy::SelfType gets properly substituted.
                        if let Some(&func_id) = self.functions_by_name.get(&impl_name) {
                            let inst = FunctionInstantiation::with_self_type(
                                func_id,
                                impl_type_args,
                                concrete_for_type,
                            );
                            if self.result.add_function(inst.clone()) {
                                self.pending.push_back(inst);
                            }
                        } else {
                            let _name_str = self.mir.name(impl_name);
                            self.errors
                                .push(MonomorphizeError::FunctionNotFound { name: impl_name });
                        }
                    },
                    Err(e) => {
                        self.errors.push(e);
                    },
                }
            },
        }
    }

    /// Scan an immediate for instantiations.
    fn scan_immediate(&mut self, imm: &ImmediateKind, subst: &Substitution) {
        match imm {
            ImmediateKind::FunctionRef { name, type_args } => {
                // Apply substitution to type args
                let mut concrete_args: Vec<_> = type_args
                    .iter()
                    .map(|ty| subst.apply_ty(self.mir, *ty))
                    .collect();

                // Record function instantiation
                if let Some(&func_id) = self.functions_by_name.get(name) {
                    let (func_type_params, func_params, func_ret) = {
                        let func_def = &self.mir.functions[func_id];
                        (
                            func_def.type_params.clone(),
                            func_def.params.clone(),
                            func_def.ret,
                        )
                    };

                    if concrete_args.is_empty() && !func_type_params.is_empty() {
                        concrete_args = self.infer_type_args_from_subst(&func_type_params, subst);
                        let unresolved = concrete_args
                            .iter()
                            .any(|&ty| matches!(self.mir.ty(ty), MirTy::TypeParam(_)));
                        if unresolved
                            && let Some(st) = subst.get_self_type()
                            && let Some(from_self) = Self::extract_named_type_args_from_self_type(
                                self.mir,
                                st,
                                func_type_params.len(),
                            )
                        {
                            concrete_args = from_self;
                        }
                    }
                    if !func_type_params.is_empty() && func_type_params.len() != concrete_args.len()
                    {
                        self.errors
                            .push(MonomorphizeError::UnsupportedFunctionReference {
                                name: *name,
                                reason: "missing or mismatched type arguments".to_string(),
                            });
                        return;
                    }

                    let callee_needs_self = func_params.iter().any(|&param_id| {
                        let param = &self.mir.params[param_id];
                        self.type_needs_self(self.mir.ty(param.ty))
                    }) || self.type_needs_self(self.mir.ty(func_ret));

                    let inst = if callee_needs_self {
                        if let Some(st) = subst.get_self_type() {
                            FunctionInstantiation::with_self_type(func_id, concrete_args, st)
                        } else {
                            self.errors
                                .push(MonomorphizeError::UnsupportedFunctionReference {
                                    name: *name,
                                    reason: "missing Self type for function reference".to_string(),
                                });
                            return;
                        }
                    } else {
                        FunctionInstantiation::new(func_id, concrete_args)
                    };
                    if self.result.add_function(inst.clone()) {
                        self.pending.push_back(inst);
                    }
                } else {
                    self.errors
                        .push(MonomorphizeError::FunctionNotFound { name: *name });
                }
            },

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
                        // For protocol extension methods, set self_type to concrete_for_type
                        if let Some(&func_id) = self.functions_by_name.get(&impl_name) {
                            let inst = FunctionInstantiation::with_self_type(
                                func_id,
                                impl_type_args,
                                concrete_for_type,
                            );
                            if self.result.add_function(inst.clone()) {
                                self.pending.push_back(inst);
                            }
                        } else {
                            let _name_str = self.mir.name(impl_name);
                            self.errors
                                .push(MonomorphizeError::FunctionNotFound { name: impl_name });
                        }
                    },
                    Err(e) => {
                        self.errors.push(e);
                    },
                }
            },

            ImmediateKind::NullPtr(ty) => {
                self.scan_type(*ty, subst);
            },

            // Literals don't contain instantiations
            ImmediateKind::IntLiteral { .. }
            | ImmediateKind::FloatLiteral { .. }
            | ImmediateKind::BoolLiteral(_)
            | ImmediateKind::StringLiteral(_)
            | ImmediateKind::StringPointer(_)
            | ImmediateKind::Unit
            | ImmediateKind::Error => {},
        }
    }

    /// Scan a value for instantiations.
    fn scan_value(&mut self, value: &Value, subst: &Substitution) {
        match value {
            Value::Place(_) => {},
            Value::Immediate(imm) => {
                self.scan_immediate(&imm.kind, subst);
            },
            Value::Unreachable => {},
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
                // Collect struct field info before mutating (to avoid borrow issues)
                let struct_field_info: Option<(Vec<Id<TypeParam>>, Vec<Id<Ty>>)> =
                    if !type_args.is_empty() {
                        // This is a generic instantiation - determine if it's a struct or enum
                        // Check structs first
                        let mut field_info = None;
                        for (struct_id, struct_def) in self.mir.structs.iter() {
                            if struct_def.name == name {
                                let inst = StructInstantiation::new(struct_id, type_args.clone());
                                self.result.add_struct(inst);
                                // Collect field types and type params for later scanning
                                if !struct_def.type_params.is_empty() {
                                    let type_params = struct_def.type_params.clone();
                                    let field_types: Vec<_> = struct_def
                                        .fields
                                        .iter()
                                        .map(|&fid| self.mir.fields[fid].ty)
                                        .collect();
                                    field_info = Some((type_params, field_types));
                                }
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
                        field_info
                    } else {
                        None
                    };

                // Scan struct field types with substitution (after loop to avoid borrow issues)
                if let Some((type_params, field_types)) = struct_field_info {
                    let field_subst = build_substitution(self.mir, &type_params, &type_args);
                    for field_ty in field_types {
                        self.scan_type(field_ty, &field_subst);
                    }
                }

                // Recurse into type args
                for arg in &type_args {
                    self.scan_type(*arg, &Substitution::new());
                }
            },

            MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => {
                // Pass through the substitution so nested types get properly substituted
                self.scan_type(inner, subst);
            },

            MirTy::Tuple(elems) => {
                for elem in elems {
                    self.scan_type(elem, subst);
                }
            },

            MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
                for param in params {
                    self.scan_type(param, subst);
                }
                self.scan_type(ret, subst);
            },

            MirTy::AssociatedTypeProjection { base, .. } => {
                self.scan_type(base, subst);
            },

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
            | MirTy::Error => {},
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
            },
            TerminatorKind::Jump(_) => {},
            TerminatorKind::Branch { condition, .. } => {
                self.scan_value(condition, subst);
            },
            TerminatorKind::Switch { .. } => {},
            TerminatorKind::Panic(_) => {},
            TerminatorKind::Unreachable => {},
        }
    }

    /// Infer the Self type from a call's arguments.
    ///
    /// For protocol extension methods, the first parameter is often `self: &Self` or similar.
    /// We need to extract the concrete type from the actual argument to substitute Self.
    fn infer_self_type_from_call(
        &self,
        callee: &Callee,
        args: &[kestrel_execution_graph::CallArg],
        subst: &Substitution,
    ) -> Option<Id<kestrel_execution_graph::Ty>> {
        match callee {
            Callee::Direct { name, .. } => {
                let func_id = self.functions_by_name.get(name).copied()?;
                self.infer_self_type_from_direct_call(func_id, args, subst)
            },
            Callee::Witness {
                for_type,
                protocol: _,
                method: _,
                ..
            } => {
                // For witness calls where for_type is SelfType, prefer existing self_type from subst
                let ty = self.mir.ty(*for_type);
                if matches!(ty, MirTy::SelfType) {
                    // If we already have a self_type in the substitution, use it
                    // (this happens when processing protocol extension methods)
                    if let Some(existing_st) = subst.get_self_type() {
                        return Some(existing_st);
                    }
                    // Otherwise try to extract from the first argument
                    if let Some(first_arg) = args.first() {
                        let arg_ty = self.get_value_type(&first_arg.value, subst);
                        if let Some(ty_id) = arg_ty {
                            // Apply substitution first to resolve any SelfType in the argument's type
                            if let Ok(substituted_ty) = subst.apply_ty_readonly(self.mir, ty_id) {
                                return self.extract_concrete_type_from_arg(substituted_ty);
                            }
                            return None;
                        }
                    }
                }
                None
            },
            _ => None,
        }
    }

    /// Infer Self type from a direct call
    fn infer_self_type_from_direct_call(
        &self,
        func_id: Id<Function>,
        args: &[kestrel_execution_graph::CallArg],
        subst: &Substitution,
    ) -> Option<Id<kestrel_execution_graph::Ty>> {
        // Get the function definition
        let func_def = &self.mir.functions[func_id];

        // Check if the function has any parameters with Self type
        if func_def.params.is_empty() || args.is_empty() {
            return None;
        }

        // Get the first parameter's type
        let first_param = &self.mir.params[func_def.params[0]];
        let param_ty = self.mir.ty(first_param.ty);

        // Check if the parameter type involves Self (could be &Self, &mut Self, Self, etc.)
        let needs_self = self.type_contains_self(param_ty);

        if !needs_self {
            return None;
        }

        // If we're already in a Self-typed context, reuse it.
        if let Some(existing_st) = subst.get_self_type() {
            return Some(existing_st);
        }

        // Extract the concrete type from the first argument
        let first_arg = &args[0];
        let arg_ty = self.get_value_type(&first_arg.value, subst)?;

        // Apply substitution to resolve any SelfType in the argument's type
        if let Ok(substituted_ty) = subst.apply_ty_readonly(self.mir, arg_ty) {
            return self.extract_concrete_type_from_arg(substituted_ty);
        }
        None
    }

    /// Check if a type contains Self or is a Named type that's a protocol (stands for Self in protocol extensions)
    fn type_contains_self(&self, ty: &MirTy) -> bool {
        match ty {
            MirTy::SelfType => true,
            // Named types that are protocols are used for Self in protocol extension methods
            // We detect this by checking if the name ends with a protocol marker or looks like a protocol
            // For now, we'll be conservative and check if the function has Self-typed params at all
            MirTy::Named { .. } => {
                // Named types could be protocols in protocol extension methods
                // We need to check if this is actually a protocol acting as Self
                // For now, return true for Named types to be conservative
                // TODO: Add proper protocol detection
                true
            },
            MirTy::Ref(inner) | MirTy::RefMut(inner) | MirTy::Pointer(inner) => {
                self.type_contains_self(self.mir.ty(*inner))
            },
            MirTy::AssociatedTypeProjection { base, .. } => {
                self.type_contains_self(self.mir.ty(*base))
            },
            _ => false,
        }
    }

    /// Try to infer the Self type from a method's qualified name.
    ///
    /// For a function like `Test.Widget.create`, this returns the type `Test.Widget`
    /// by looking up the parent name in structs and enums.
    fn infer_self_type_from_method_name(
        &self,
        func_name: Id<QualifiedName>,
    ) -> Option<Id<kestrel_execution_graph::Ty>> {
        let name_data = self.mir.name(func_name);
        let parent = name_data.parent()?;

        // Try to find a struct with this name
        for (_, struct_def) in self.mir.structs.iter() {
            if self.mir.name(struct_def.name) == &parent {
                // Only works for non-generic types
                if struct_def.type_params.is_empty() {
                    let mir_ty = MirTy::Named {
                        name: struct_def.name,
                        type_args: vec![],
                    };
                    return self.mir.lookup_type(&mir_ty);
                }
            }
        }

        // Try to find an enum with this name
        for (_, enum_def) in self.mir.enums.iter() {
            if self.mir.name(enum_def.name) == &parent && enum_def.type_params.is_empty() {
                let mir_ty = MirTy::Named {
                    name: enum_def.name,
                    type_args: vec![],
                };
                return self.mir.lookup_type(&mir_ty);
            }
        }

        None
    }

    /// Check if a type directly uses SelfType (not just any Named type)
    /// This is stricter than type_contains_self and is used to detect protocol extension methods
    fn type_needs_self(&self, ty: &MirTy) -> bool {
        match ty {
            MirTy::SelfType => true,
            MirTy::Ref(inner) | MirTy::RefMut(inner) | MirTy::Pointer(inner) => {
                self.type_needs_self(self.mir.ty(*inner))
            },
            MirTy::Tuple(elems) => elems.iter().any(|e| self.type_needs_self(self.mir.ty(*e))),
            MirTy::Named { type_args, .. } => type_args
                .iter()
                .any(|a| self.type_needs_self(self.mir.ty(*a))),
            MirTy::AssociatedTypeProjection { base, .. } => {
                self.type_needs_self(self.mir.ty(*base))
            },
            MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
                params.iter().any(|p| self.type_needs_self(self.mir.ty(*p)))
                    || self.type_needs_self(self.mir.ty(*ret))
            },
            _ => false,
        }
    }

    /// Get the type of a value
    fn get_value_type(
        &self,
        value: &Value,
        subst: &Substitution,
    ) -> Option<Id<kestrel_execution_graph::Ty>> {
        match value {
            Value::Place(place) => self.get_place_type(place, subst),
            Value::Immediate(_) => None,
            Value::Unreachable => None,
        }
    }

    /// Get the type of a place
    #[allow(clippy::only_used_in_recursion)]
    fn get_place_type(
        &self,
        place: &kestrel_execution_graph::Place,
        subst: &Substitution,
    ) -> Option<Id<kestrel_execution_graph::Ty>> {
        use kestrel_execution_graph::PlaceKind;

        match &place.kind {
            PlaceKind::Local(local_id) => {
                let local = self.mir.local(*local_id);
                Some(local.ty)
            },
            PlaceKind::Global(name_id) => {
                // Find the static definition to get its type
                self.mir
                    .statics
                    .iter()
                    .find(|(_, def)| def.name == *name_id)
                    .map(|(_, def)| def.ty)
            },
            PlaceKind::Deref(inner) => {
                // For deref, get the inner place's type and unwrap the pointer/ref
                let inner_ty_id = self.get_place_type(inner, subst)?;
                let inner_ty = self.mir.ty(inner_ty_id);
                match inner_ty {
                    MirTy::Ref(pointee) | MirTy::RefMut(pointee) | MirTy::Pointer(pointee) => {
                        Some(*pointee)
                    },
                    _ => Some(inner_ty_id), // Unexpected, return as-is
                }
            },
            PlaceKind::Field { parent: _, .. } => {
                // For field access, we'd need to look up the struct definition
                // For now, return None as this is complex
                None
            },
            PlaceKind::Index { parent: _, .. } => {
                // For index access, we'd need to look up element type
                None
            },
            PlaceKind::Downcast { parent, .. } => {
                // For enum downcast, return parent type (the enum)
                self.get_place_type(parent, subst)
            },
        }
    }

    /// Extract the concrete type from an argument type (unwrap refs)
    fn extract_concrete_type_from_arg(
        &self,
        ty: Id<kestrel_execution_graph::Ty>,
    ) -> Option<Id<kestrel_execution_graph::Ty>> {
        let mir_ty = self.mir.ty(ty);
        match mir_ty {
            MirTy::Ref(inner) | MirTy::RefMut(inner) => {
                // Recursively unwrap
                self.extract_concrete_type_from_arg(*inner)
            },
            MirTy::SelfType | MirTy::TypeParam(_) => {
                // Can't extract concrete type from abstract type
                None
            },
            _ => Some(ty),
        }
    }
}
