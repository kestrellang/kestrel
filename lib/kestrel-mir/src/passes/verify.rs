//! MIR verification pass — catches structural issues before codegen.
//!
//! Reports ALL issues rather than failing on the first, so you can
//! see the full picture and fix batches at once.

use std::collections::HashMap;
use std::fmt;

use kestrel_hecs::Entity;

use crate::{
    BasicBlock, Callee, CopyBehavior, FunctionDef, FunctionId, ImmediateKind, MirBody,
    MirTy, Place, Rvalue, StatementKind, TerminatorKind, Value,
};

/// A single verification diagnostic.
#[derive(Debug)]
pub struct VerifyError {
    /// Which function (by index) the error occurs in.
    pub function: String,
    /// Block index within the function (None for function-level issues).
    pub block: Option<usize>,
    /// Human-readable description.
    pub message: String,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(b) = self.block {
            write!(f, "[{}:bb{}] {}", self.function, b, self.message)
        } else {
            write!(f, "[{}] {}", self.function, self.message)
        }
    }
}

/// Result of running the verification pass.
pub struct VerifyResult {
    pub errors: Vec<VerifyError>,
}

impl VerifyResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Like [`Self::dump`] but silent on success — used by pipeline hooks
    /// that want to surface verifier diagnostics without spamming stderr
    /// for every successful compile.
    pub fn dump_if_errors(&self) {
        if !self.errors.is_empty() {
            self.dump();
        }
    }

    /// Print all errors to stderr, grouped by category.
    pub fn dump(&self) {
        if self.errors.is_empty() {
            eprintln!("MIR verify: OK (no issues)");
            return;
        }

        // Group by message prefix for readability
        let mut by_kind: HashMap<String, Vec<&VerifyError>> = HashMap::new();
        for e in &self.errors {
            let kind = e.message.split(':').next().unwrap_or("other").to_string();
            by_kind.entry(kind).or_default().push(e);
        }

        eprintln!("MIR verify: {} issues found", self.errors.len());
        for (kind, errs) in &by_kind {
            eprintln!("  {} ({}):", kind, errs.len());
            for e in errs.iter().take(5) {
                eprintln!("    {}", e);
            }
            if errs.len() > 5 {
                eprintln!("    ... and {} more", errs.len() - 5);
            }
        }

        // Write full details to temp file for analysis
        if !self.errors.is_empty()
            && let Ok(mut f) = std::fs::File::create("/tmp/mir_verify.txt") {
                use std::io::Write;
                for e in &self.errors {
                    let _ = writeln!(f, "{}", e);
                }
            }
    }
}

/// Where in the pipeline this verification runs.
///
/// `PreDropElab` is used between lowering and `kestrel_ownership::run`. At
/// that point the MIR must contain no `Drop` / `DropIf` statements — those
/// are exclusively emitted by drop-elaboration. `PostDropElab` is used
/// afterwards (the normal `verify(module)` entry point).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyStage {
    /// MIR straight out of lowering (and the legacy deinit pass). At this
    /// point lowering must not have emitted any `Drop`/`DropIf` — those
    /// are the exclusive job of `kestrel-ownership`.
    PreDropElab,
    /// Final MIR ready for codegen. `Drop`/`DropIf` are accepted.
    PostDropElab,
}

/// Run verification on the entire MIR module, post-drop-elab.
pub fn verify(module: &crate::MirModule) -> VerifyResult {
    verify_with_stage(module, VerifyStage::PostDropElab)
}

/// Run verification with an explicit stage marker.
pub fn verify_with_stage(module: &crate::MirModule, stage: VerifyStage) -> VerifyResult {
    let mut ctx = VerifyCtx::new(module, stage);
    ctx.verify_all();
    VerifyResult { errors: ctx.errors }
}

struct VerifyCtx<'a> {
    module: &'a crate::MirModule,
    stage: VerifyStage,
    errors: Vec<VerifyError>,
    // Lookup maps built once
    entity_to_func: HashMap<Entity, FunctionId>,
}

impl<'a> VerifyCtx<'a> {
    fn new(module: &'a crate::MirModule, stage: VerifyStage) -> Self {
        let entity_to_func: HashMap<Entity, FunctionId> = module
            .functions
            .iter()
            .enumerate()
            .map(|(i, f)| (f.entity, FunctionId::new(i)))
            .collect();

        Self {
            module,
            stage,
            errors: Vec::new(),
            entity_to_func,
        }
    }

    fn err(&mut self, func: &str, block: Option<usize>, msg: String) {
        self.errors.push(VerifyError {
            function: func.to_string(),
            block,
            message: msg,
        });
    }

    fn verify_all(&mut self) {
        for (i, func) in self.module.functions.iter().enumerate() {
            self.verify_function(i, func);
        }
    }

    fn verify_function(&mut self, _idx: usize, func: &FunctionDef) {
        let name = &func.name;

        // Check: function with body should have entry block
        let Some(body) = &func.body else { return };

        if body.blocks.is_empty() {
            self.err(name, None, "empty body: no basic blocks".into());
            return;
        }

        // Check: param count consistency
        if body.param_count != func.params.len() {
            self.err(
                name,
                None,
                format!(
                    "param count mismatch: body.param_count={} but func.params.len()={}",
                    body.param_count,
                    func.params.len()
                ),
            );
        }

        // Check: locals count >= param count
        if body.locals.len() < body.param_count {
            self.err(
                name,
                None,
                format!(
                    "locals ({}) fewer than params ({})",
                    body.locals.len(),
                    body.param_count
                ),
            );
        }

        // Module/static init bodies (synthetic `__init$<name>` functions
        // generated for static initializers) must contain no `Drop`/
        // `DropIf`. The architecture explicitly rules out module/static
        // deinit — there's no point where it could run.
        if self.stage == VerifyStage::PostDropElab && name.starts_with("__init$") {
            for (bi, block) in body.blocks.iter().enumerate() {
                for stmt in &block.stmts {
                    match &stmt.kind {
                        StatementKind::Drop { .. } | StatementKind::DropIf { .. } => {
                            self.err(
                                name,
                                Some(bi),
                                "module-init: Drop/DropIf emitted in a static-init body; \
                                 static-init bodies must not own values that need dropping"
                                    .into(),
                            );
                        },
                        _ => {},
                    }
                }
            }
        }

        // Check each basic block
        for (bi, block) in body.blocks.iter().enumerate() {
            self.verify_block(name, bi, block, body, func);
        }

        // Check: call arg counts match callee param counts
        for (bi, block) in body.blocks.iter().enumerate() {
            for stmt in &block.stmts {
                if let StatementKind::Call { callee, args, .. } = &stmt.kind {
                    self.verify_call_args(name, bi, callee, args, func, body);
                }
            }
        }
    }

    fn verify_block(
        &mut self,
        func_name: &str,
        bi: usize,
        block: &BasicBlock,
        body: &MirBody,
        func: &FunctionDef,
    ) {
        // Verify statements
        for stmt in &block.stmts {
            match &stmt.kind {
                StatementKind::Assign { dest, rvalue } => {
                    self.verify_place(func_name, bi, dest, body);
                    self.verify_rvalue(func_name, bi, rvalue, body, func);
                },
                StatementKind::Call {
                    dest, callee, args, ..
                } => {
                    if let Some(d) = dest {
                        self.verify_place(func_name, bi, d, body);
                    }
                    for arg in args {
                        self.verify_value(func_name, bi, arg, body, func);
                    }
                    self.verify_callee(func_name, bi, callee);
                },
                StatementKind::Drop { place } => {
                    self.verify_place(func_name, bi, place, body);
                    if self.stage == VerifyStage::PreDropElab {
                        self.err(
                            func_name,
                            Some(bi),
                            "drop: Drop statement present before drop-elaboration; \
                             lowering must never emit Drop/DropIf"
                                .into(),
                        );
                    }
                },
                StatementKind::DropIf { place, flag } => {
                    self.verify_place(func_name, bi, place, body);
                    self.verify_local(func_name, bi, *flag, body);
                    if self.stage == VerifyStage::PreDropElab {
                        self.err(
                            func_name,
                            Some(bi),
                            "drop: DropIf statement present before drop-elaboration; \
                             lowering must never emit Drop/DropIf"
                                .into(),
                        );
                    }
                },
            }
        }

        // Verify terminator
        self.verify_terminator(func_name, bi, &block.terminator.kind, body, func);

        // Panic-edge invariant: blocks ending in `Panic(_)` or
        // `Unreachable` must not own any `Drop`/`DropIf` statements.
        // Per the memory-model architecture, panic = abort; drops on a
        // path that aborts are unreachable code that DropElab should
        // have skipped. If they're here it's a placement bug.
        if matches!(
            block.terminator.kind,
            TerminatorKind::Panic(_) | TerminatorKind::Unreachable
        ) && self.stage == VerifyStage::PostDropElab
        {
            for stmt in &block.stmts {
                match &stmt.kind {
                    StatementKind::Drop { .. } | StatementKind::DropIf { .. } => {
                        self.err(
                            func_name,
                            Some(bi),
                            "panic-edge: Drop/DropIf in a block that terminates with \
                             panic/unreachable; panic = abort, those drops never run"
                                .into(),
                        );
                    },
                    _ => {},
                }
            }
        }
    }

    fn verify_place(&mut self, func: &str, bi: usize, place: &Place, body: &MirBody) {
        match place {
            Place::Local(id) => {
                self.verify_local(func, bi, *id, body);
            },
            Place::Field { parent, name } => {
                self.verify_place(func, bi, parent, body);
                // Check field exists — we'd need the type of `parent` to look it up,
                // which requires type tracking. For now, just flag obviously bad names.
                if name.is_empty() {
                    self.err(func, Some(bi), "field access: empty field name".into());
                }
            },
            Place::Index { parent, .. } => {
                self.verify_place(func, bi, parent, body);
            },
            Place::Downcast { parent, variant } => {
                self.verify_place(func, bi, parent, body);
                // Check for common display_name bug patterns
                if variant.contains('(') || variant.contains(')') {
                    self.err(
                        func,
                        Some(bi),
                        format!(
                            "downcast: variant name '{}' contains parens (likely display_name leak)",
                            variant
                        ),
                    );
                }
            },
            Place::Deref(inner) => {
                self.verify_place(func, bi, inner, body);
            },
            Place::Global(_) => {},
        }
    }

    fn verify_local(&mut self, func: &str, bi: usize, id: crate::LocalId, body: &MirBody) {
        if id.index() >= body.locals.len() {
            self.err(
                func,
                Some(bi),
                format!(
                    "local out of bounds: local_{} but only {} locals",
                    id.index(),
                    body.locals.len()
                ),
            );
        }
    }

    fn verify_value(
        &mut self,
        func_name: &str,
        bi: usize,
        value: &Value,
        body: &MirBody,
        func: &FunctionDef,
    ) {
        match value {
            Value::Copy(p) => {
                self.verify_place(func_name, bi, p, body);
                self.verify_copy_legality(func_name, bi, p, body, func);
            },
            Value::Move(p) => {
                self.verify_place(func_name, bi, p, body);
                self.verify_move_legality(func_name, bi, p, body, func);
            },
            Value::Ref(p) | Value::RefMut(p) => {
                self.verify_place(func_name, bi, p, body);
            },
            Value::Const(imm) => self.verify_immediate(func_name, bi, &imm.kind),
        }
    }

    fn verify_rvalue(
        &mut self,
        func_name: &str,
        bi: usize,
        rvalue: &Rvalue,
        body: &MirBody,
        func: &FunctionDef,
    ) {
        match rvalue {
            Rvalue::Copy(p) => {
                self.verify_place(func_name, bi, p, body);
                self.verify_copy_legality(func_name, bi, p, body, func);
            },
            Rvalue::Move(p) => {
                self.verify_place(func_name, bi, p, body);
                self.verify_move_legality(func_name, bi, p, body, func);
            },
            Rvalue::Ref(p) | Rvalue::RefMut(p) => {
                self.verify_place(func_name, bi, p, body);
            },
            Rvalue::Const(imm) => {
                self.verify_immediate(func_name, bi, &imm.kind);
            },
            Rvalue::Op1 { arg, .. } => {
                self.verify_value(func_name, bi, arg, body, func);
            },
            Rvalue::Op2 { lhs, rhs, .. } => {
                self.verify_value(func_name, bi, lhs, body, func);
                self.verify_value(func_name, bi, rhs, body, func);
            },
            Rvalue::Op3 { a, b, c, .. } => {
                self.verify_value(func_name, bi, a, body, func);
                self.verify_value(func_name, bi, b, body, func);
                self.verify_value(func_name, bi, c, body, func);
            },
            Rvalue::Construct { fields, .. } => {
                for (_, v) in fields {
                    self.verify_value(func_name, bi, v, body, func);
                }
            },
            Rvalue::Tuple(vals) => {
                for v in vals {
                    self.verify_value(func_name, bi, v, body, func);
                }
            },
            Rvalue::EnumVariant { payload, .. } => {
                for v in payload {
                    self.verify_value(func_name, bi, v, body, func);
                }
            },
            Rvalue::ArrayLiteral { values, .. } => {
                for v in values {
                    self.verify_value(func_name, bi, v, body, func);
                }
            },
            Rvalue::ApplyPartial {
                func: target,
                captures,
            } => {
                // Check the target entity is a known function
                if !self.entity_to_func.contains_key(target) {
                    self.err(
                        func_name,
                        Some(bi),
                        format!(
                            "ApplyPartial: target entity {:?} not a known function",
                            target
                        ),
                    );
                }
                for cap in captures {
                    self.verify_value(func_name, bi, cap, body, func);
                }
            },
        }
    }

    /// `Value::Move(p)` is only legal when:
    ///   - `p.ty.copy_behavior_with_constraints == None` (the type is
    ///     affine), and
    ///   - `p` is rooted in an owned local — not a `Deref` of a `&T` or
    ///     `&var T` reference.
    fn verify_move_legality(
        &mut self,
        func_name: &str,
        bi: usize,
        place: &Place,
        body: &MirBody,
        func: &FunctionDef,
    ) {
        if let Some(ty) = place_type(self.module, body, func, place) {
            let behavior = ty.copy_behavior_with_constraints(self.module, func.where_clause.as_ref());
            if !matches!(behavior, CopyBehavior::None) {
                self.err(
                    func_name,
                    Some(bi),
                    format!(
                        "move: Value::Move on copyable type `{}` (CopyBehavior::{:?}); lowering should emit Copy instead",
                        ty.display(self.module),
                        copy_behavior_name(&behavior),
                    ),
                );
            }
        }
        if let Some(reason) = move_root_violation(self.module, body, func, place) {
            self.err(
                func_name,
                Some(bi),
                format!("move: cannot move out of a borrow ({reason})"),
            );
        }
    }

    /// `Value::Copy(p)` requires the place's type to be copyable.
    fn verify_copy_legality(
        &mut self,
        func_name: &str,
        bi: usize,
        place: &Place,
        body: &MirBody,
        func: &FunctionDef,
    ) {
        let Some(ty) = place_type(self.module, body, func, place) else {
            return;
        };
        let behavior = ty.copy_behavior_with_constraints(self.module, func.where_clause.as_ref());
        if matches!(behavior, CopyBehavior::None) {
            self.err(
                func_name,
                Some(bi),
                format!(
                    "copy: Value::Copy on non-copyable type `{}`; lowering should emit Move",
                    ty.display(self.module),
                ),
            );
        }
    }

    fn verify_callee(&mut self, func: &str, bi: usize, callee: &Callee) {
        match callee {
            Callee::Direct { func: target, .. } => {
                if !self.entity_to_func.contains_key(target) {
                    let target_name = self.module.resolve_name(*target);
                    self.err(
                        func,
                        Some(bi),
                        format!(
                            "call: Direct callee entity {:?} ({}) not a known function",
                            target, target_name
                        ),
                    );
                }
            },
            Callee::Witness {
                protocol,
                method,
                self_type,
                ..
            } => {
                // Check protocol entity is known
                let proto_name = self.module.resolve_name(*protocol);
                if proto_name.starts_with("<entity:") {
                    self.err(
                        func,
                        Some(bi),
                        format!(
                            "call: Witness protocol {:?} has no registered name",
                            protocol
                        ),
                    );
                }
                // Check self_type isn't unresolved
                if matches!(self_type, MirTy::Error) {
                    self.err(
                        func,
                        Some(bi),
                        format!(
                            "call: Witness {}.{} has Error self_type",
                            proto_name, method
                        ),
                    );
                }
            },
            Callee::Thin(p) | Callee::Thick(p) => {
                // Can't verify much without type info, just check place is well-formed
                let _ = p; // place check already done by caller
            },
        }
    }

    fn verify_call_args(
        &mut self,
        func_name: &str,
        bi: usize,
        callee: &Callee,
        args: &[Value],
        _func: &FunctionDef,
        _body: &MirBody,
    ) {
        // For Direct calls, check arg count matches the target function's param count
        if let Callee::Direct { func: target, .. } = callee
            && let Some(&func_id) = self.entity_to_func.get(target) {
                let target_def = &self.module.functions[func_id.index()];
                let expected = target_def.params.len();
                let got = args.len();
                if got != expected {
                    self.err(
                        func_name,
                        Some(bi),
                        format!(
                            "call arg count: calling '{}' with {} args, expected {} params",
                            target_def.name, got, expected
                        ),
                    );
                }
            }
    }

    fn verify_immediate(&mut self, func: &str, bi: usize, kind: &ImmediateKind) {
        if let ImmediateKind::FunctionRef { func: target, .. } = kind
            && !self.entity_to_func.contains_key(target)
        {
            let target_name = self.module.resolve_name(*target);
            self.err(
                func,
                Some(bi),
                format!(
                    "FunctionRef: entity {:?} ({}) not a known function",
                    target, target_name
                ),
            );
        }
    }

    fn verify_terminator(
        &mut self,
        func_name: &str,
        bi: usize,
        kind: &TerminatorKind,
        body: &MirBody,
        func: &FunctionDef,
    ) {
        match kind {
            TerminatorKind::Jump(target) => {
                self.verify_block_id(func_name, bi, *target, body);
            },
            TerminatorKind::Branch {
                condition,
                then_block,
                else_block,
            } => {
                self.verify_value(func_name, bi, condition, body, func);
                self.verify_block_id(func_name, bi, *then_block, body);
                self.verify_block_id(func_name, bi, *else_block, body);
            },
            TerminatorKind::Switch {
                discriminant,
                cases,
            } => {
                self.verify_place(func_name, bi, discriminant, body);
                for (case, target) in cases {
                    self.verify_block_id(func_name, bi, *target, body);
                    // Guard against display_name leakage on enum variants —
                    // case_by_name keys on short names, not display form.
                    if let crate::SwitchCase::Variant(name) = case
                        && (name.contains('(') || name.contains(')')) {
                            self.err(
                                func_name,
                                Some(bi),
                                format!(
                                    "switch: case name '{}' contains parens (likely display_name leak)",
                                    name
                                ),
                            );
                        }
                }
            },
            TerminatorKind::Return(val) => {
                self.verify_value(func_name, bi, val, body, func);
            },
            TerminatorKind::Panic(_) | TerminatorKind::Unreachable => {},
        }
    }

    fn verify_block_id(&mut self, func: &str, bi: usize, target: crate::BlockId, body: &MirBody) {
        if target.index() >= body.blocks.len() {
            self.err(
                func,
                Some(bi),
                format!(
                    "block out of bounds: bb{} but only {} blocks",
                    target.index(),
                    body.blocks.len()
                ),
            );
        }
    }
}

/// Display name for a [`CopyBehavior`] variant — used in diagnostic
/// messages.
fn copy_behavior_name(behavior: &CopyBehavior) -> &'static str {
    match behavior {
        CopyBehavior::None => "None",
        CopyBehavior::Bitwise => "Bitwise",
        CopyBehavior::Clone(_) => "Clone",
    }
}

/// Resolve a [`Place`] to its [`MirTy`] by walking the projection chain.
///
/// Returns `None` if the type can't be determined (out-of-bounds local,
/// missing struct, unknown enum variant, etc.). The verifier treats
/// `None` as "skip this check" rather than emitting a confusing
/// secondary diagnostic — the root cause already surfaces via other
/// rules.
fn place_type(
    module: &crate::MirModule,
    body: &MirBody,
    func: &FunctionDef,
    place: &Place,
) -> Option<MirTy> {
    match place {
        Place::Local(id) => body.locals.get(id.index()).map(|l| l.ty.clone()),
        Place::Global(entity) => module
            .statics
            .iter()
            .find(|s| s.entity == *entity)
            .map(|s| s.ty.clone()),
        Place::Field { parent, name } => {
            let parent_ty = place_type(module, body, func, parent)?;
            let MirTy::Named { entity, type_args } = parent_ty else {
                return None;
            };
            let s = module.structs.iter().find(|s| s.entity == entity)?;
            let field_id = s.field_by_name(name)?;
            let field_ty = s.fields.get(field_id.index()).map(|f| f.ty.clone())?;
            Some(substitute_struct_field_ty(&field_ty, s, &type_args))
        },
        Place::Index { parent, index } => {
            let parent_ty = place_type(module, body, func, parent)?;
            match parent_ty {
                MirTy::Tuple(mut elems) if *index < elems.len() => Some(elems.swap_remove(*index)),
                MirTy::Named { entity, type_args } => {
                    let s = module.structs.iter().find(|s| s.entity == entity)?;
                    let field = s.fields.get(*index)?;
                    Some(substitute_struct_field_ty(&field.ty, s, &type_args))
                },
                _ => None,
            }
        },
        Place::Downcast { parent, variant } => {
            let parent_ty = place_type(module, body, func, parent)?;
            let MirTy::Named { entity, type_args } = parent_ty else {
                return None;
            };
            let e = module.enums.iter().find(|e| e.entity == entity)?;
            let case = e.cases.iter().find(|c| c.name == *variant)?;
            // The downcast yields the payload struct type, parameterized
            // by the enum's type args.
            let payload_def = module.structs.get(case.payload_struct.index())?;
            Some(MirTy::Named {
                entity: payload_def.entity,
                type_args,
            })
        },
        Place::Deref(inner) => {
            let inner_ty = place_type(module, body, func, inner)?;
            match inner_ty {
                MirTy::Ref(t) | MirTy::RefMut(t) | MirTy::Pointer(t) => Some(*t),
                _ => None,
            }
        },
    }
}

/// Apply a struct's type-param substitutions to a field type.
fn substitute_struct_field_ty(
    field_ty: &MirTy,
    struct_def: &crate::StructDef,
    type_args: &[MirTy],
) -> MirTy {
    let subst: std::collections::HashMap<Entity, MirTy> = struct_def
        .type_params
        .iter()
        .zip(type_args.iter())
        .map(|(tp, arg)| (tp.entity, arg.clone()))
        .collect();
    substitute_type_locally(field_ty, &subst)
}

/// Single-shot type-param substitution — duplicates `kestrel-codegen`'s
/// `substitute_type` to keep the verifier free of that dependency.
fn substitute_type_locally(ty: &MirTy, subst: &std::collections::HashMap<Entity, MirTy>) -> MirTy {
    match ty {
        MirTy::TypeParam(e) => subst.get(e).cloned().unwrap_or_else(|| ty.clone()),
        MirTy::Ref(inner) => MirTy::Ref(Box::new(substitute_type_locally(inner, subst))),
        MirTy::RefMut(inner) => MirTy::RefMut(Box::new(substitute_type_locally(inner, subst))),
        MirTy::Pointer(inner) => MirTy::Pointer(Box::new(substitute_type_locally(inner, subst))),
        MirTy::Tuple(elems) => MirTy::Tuple(
            elems
                .iter()
                .map(|e| substitute_type_locally(e, subst))
                .collect(),
        ),
        MirTy::Named { entity, type_args } => MirTy::Named {
            entity: *entity,
            type_args: type_args
                .iter()
                .map(|a| substitute_type_locally(a, subst))
                .collect(),
        },
        MirTy::FuncThin { params, ret } => MirTy::FuncThin {
            params: params.iter().map(|p| substitute_type_locally(p, subst)).collect(),
            ret: Box::new(substitute_type_locally(ret, subst)),
        },
        MirTy::FuncThick { params, ret } => MirTy::FuncThick {
            params: params.iter().map(|p| substitute_type_locally(p, subst)).collect(),
            ret: Box::new(substitute_type_locally(ret, subst)),
        },
        _ => ty.clone(),
    }
}

/// Walk a place's projection chain looking for a `Deref(p)` whose inner
/// type is a `Ref(_)` or `RefMut(_)`. Returns `Some(reason)` if such a
/// projection is found, else `None`.
///
/// Moving through a borrow would invalidate the caller's reference, so
/// the verifier flags it.
fn move_root_violation(
    module: &crate::MirModule,
    body: &MirBody,
    func: &FunctionDef,
    place: &Place,
) -> Option<String> {
    match place {
        Place::Local(_) | Place::Global(_) => None,
        Place::Field { parent, .. }
        | Place::Index { parent, .. }
        | Place::Downcast { parent, .. } => move_root_violation(module, body, func, parent),
        Place::Deref(inner) => {
            let inner_ty = place_type(module, body, func, inner)?;
            match &inner_ty {
                MirTy::Ref(_) => Some("deref of `&T`".into()),
                MirTy::RefMut(_) => Some("deref of `&var T`".into()),
                // Raw pointer deref is allowed (unsafe, but not a borrow).
                _ => move_root_violation(module, body, func, inner),
            }
        },
    }
}
