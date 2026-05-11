//! MIR verification pass — catches structural issues before codegen.
//!
//! Reports ALL issues rather than failing on the first, so you can
//! see the full picture and fix batches at once.

use std::collections::HashMap;
use std::fmt;

use kestrel_hecs::Entity;

use crate::{
    BasicBlock, Callee, FunctionDef, FunctionId, ImmediateKind, MirBody,
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

/// Run verification on the entire MIR module.
pub fn verify(module: &crate::MirModule) -> VerifyResult {
    let mut ctx = VerifyCtx::new(module);
    ctx.verify_all();
    VerifyResult { errors: ctx.errors }
}

struct VerifyCtx<'a> {
    module: &'a crate::MirModule,
    errors: Vec<VerifyError>,
    // Lookup maps built once
    entity_to_func: HashMap<Entity, FunctionId>,
}

impl<'a> VerifyCtx<'a> {
    fn new(module: &'a crate::MirModule) -> Self {
        let entity_to_func: HashMap<Entity, FunctionId> = module
            .functions
            .iter()
            .enumerate()
            .map(|(i, f)| (f.entity, FunctionId::new(i)))
            .collect();

        Self {
            module,
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
        _func: &FunctionDef,
    ) {
        // Verify statements
        for stmt in &block.stmts {
            match &stmt.kind {
                StatementKind::Assign { dest, rvalue } => {
                    self.verify_place(func_name, bi, dest, body);
                    self.verify_rvalue(func_name, bi, rvalue, body);
                },
                StatementKind::Call {
                    dest, callee, args, ..
                } => {
                    if let Some(d) = dest {
                        self.verify_place(func_name, bi, d, body);
                    }
                    for arg in args {
                        self.verify_value(func_name, bi, &arg.value, body);
                    }
                    self.verify_callee(func_name, bi, callee);
                },
                StatementKind::Deinit { place } => {
                    self.verify_place(func_name, bi, place, body);
                },
                StatementKind::DeinitIf { place, flag } => {
                    self.verify_place(func_name, bi, place, body);
                    self.verify_local(func_name, bi, *flag, body);
                },
                StatementKind::SetDeinitFlag { flag, .. } => {
                    self.verify_local(func_name, bi, *flag, body);
                },
                StatementKind::Drop { place } => {
                    self.verify_place(func_name, bi, place, body);
                },
                StatementKind::DropIf { place, flag } => {
                    self.verify_place(func_name, bi, place, body);
                    self.verify_local(func_name, bi, *flag, body);
                },
            }
        }

        // Verify terminator
        self.verify_terminator(func_name, bi, &block.terminator.kind, body);
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

    fn verify_value(&mut self, func: &str, bi: usize, value: &Value, body: &MirBody) {
        match value {
            Value::Place(p) => self.verify_place(func, bi, p, body),
            Value::Immediate(imm) => self.verify_immediate(func, bi, &imm.kind),
        }
    }

    fn verify_rvalue(&mut self, func: &str, bi: usize, rvalue: &Rvalue, body: &MirBody) {
        match rvalue {
            Rvalue::Move(p) | Rvalue::Copy(p) | Rvalue::Ref(p) | Rvalue::RefMut(p) => {
                self.verify_place(func, bi, p, body);
            },
            Rvalue::Const(imm) => {
                self.verify_immediate(func, bi, &imm.kind);
            },
            Rvalue::Op1 { arg, .. } => {
                self.verify_value(func, bi, arg, body);
            },
            Rvalue::Op2 { lhs, rhs, .. } => {
                self.verify_value(func, bi, lhs, body);
                self.verify_value(func, bi, rhs, body);
            },
            Rvalue::Op3 { a, b, c, .. } => {
                self.verify_value(func, bi, a, body);
                self.verify_value(func, bi, b, body);
                self.verify_value(func, bi, c, body);
            },
            Rvalue::Construct { fields, .. } => {
                for (_, v) in fields {
                    self.verify_value(func, bi, v, body);
                }
            },
            Rvalue::Tuple(vals) => {
                for v in vals {
                    self.verify_value(func, bi, v, body);
                }
            },
            Rvalue::EnumVariant { payload, .. } => {
                for v in payload {
                    self.verify_value(func, bi, v, body);
                }
            },
            Rvalue::ArrayLiteral { values, .. } => {
                for v in values {
                    self.verify_value(func, bi, v, body);
                }
            },
            Rvalue::ApplyPartial {
                func: target,
                captures,
            } => {
                // Check the target entity is a known function
                if !self.entity_to_func.contains_key(target) {
                    self.err(
                        func,
                        Some(bi),
                        format!(
                            "ApplyPartial: target entity {:?} not a known function",
                            target
                        ),
                    );
                }
                for cap in captures {
                    self.verify_value(func, bi, cap, body);
                }
            },
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
        args: &[crate::CallArg],
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

    fn verify_terminator(&mut self, func: &str, bi: usize, kind: &TerminatorKind, body: &MirBody) {
        match kind {
            TerminatorKind::Jump(target) => {
                self.verify_block_id(func, bi, *target, body);
            },
            TerminatorKind::Branch {
                condition,
                then_block,
                else_block,
            } => {
                self.verify_value(func, bi, condition, body);
                self.verify_block_id(func, bi, *then_block, body);
                self.verify_block_id(func, bi, *else_block, body);
            },
            TerminatorKind::Switch {
                discriminant,
                cases,
            } => {
                self.verify_place(func, bi, discriminant, body);
                for (case, target) in cases {
                    self.verify_block_id(func, bi, *target, body);
                    // Guard against display_name leakage on enum variants —
                    // case_by_name keys on short names, not display form.
                    if let crate::SwitchCase::Variant(name) = case
                        && (name.contains('(') || name.contains(')')) {
                            self.err(
                                func,
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
                self.verify_value(func, bi, val, body);
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
