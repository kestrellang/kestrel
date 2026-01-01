//! Statement patterns for MIR testing.

use kestrel_execution_graph::{
    BinOp, Callee, CastKind, MirContext, PassingMode, Rvalue, StatementData, StatementKind, UnOp,
};

/// Pattern for matching statements in MIR.
#[derive(Debug, Clone)]
pub enum StatementPattern {
    /// Any assignment statement
    Assign,

    /// Assignment to a specific local (by name, e.g., "result" or "prim_0")
    AssignTo(String),

    /// Copy rvalue
    Copy,

    /// Move rvalue
    Move,

    /// Ref rvalue (&)
    Ref,

    /// RefMut rvalue (&var)
    RefMut,

    /// Construct a struct (type name pattern)
    Construct { ty: String },

    /// Create an enum variant
    EnumVariant { enum_ty: String, variant: String },

    /// Create a tuple with specific arity
    Tuple { arity: usize },

    /// Create an array
    Array,

    /// Specific binary operation
    BinOp(BinOp),

    /// Any binary operation
    AnyBinOp,

    /// Specific unary operation
    UnOp(UnOp),

    /// Any unary operation
    AnyUnOp,

    /// Direct call to a specific function
    Call { callee: String },

    /// Direct call with type arguments
    CallGeneric {
        callee: String,
        type_arg_count: usize,
    },

    /// Witness method call
    CallWitness { protocol: String, method: String },

    /// Escaping (thick) call
    CallEscaping,

    /// Any call
    AnyCall,

    /// func.to.escaping
    FuncToEscaping { func: String },

    /// apply partial
    ApplyPartial { func: String, capture_count: usize },

    /// Type cast
    Cast { kind: CastKind },

    /// Call with specific passing modes for arguments
    /// Each element specifies the expected passing mode for that argument position
    CallWithModes {
        callee: String,
        arg_modes: Vec<PassingMode>,
    },

    /// String operations
    StrPtr,
    StrLen,
    StrFromParts,

    /// Pointer operations
    PtrOffset,
    PtrToRef,
    PtrToRefMut,
    RefToPtr,

    /// Deinit operations
    /// Unconditional deinit of a local
    Deinit { local: String },
    /// Any unconditional deinit
    AnyDeinit,
    /// Conditional deinit
    DeinitIf { local: String, flag: String },
    /// Any conditional deinit
    AnyDeinitIf,
    /// Set deinit flag to a specific value
    SetDeinitFlag { flag: String, value: bool },
    /// Any set deinit flag
    AnySetDeinitFlag,

    /// Deinit call - matches `call Module.Type.deinit(...)` patterns
    /// This is used when a struct/enum has a custom deinit block
    DeinitCall { ty: String },
    /// Any deinit call (matches any call to a `.deinit` function)
    AnyDeinitCall,
}

impl StatementPattern {
    /// Check if this pattern matches a statement.
    pub(crate) fn matches(&self, stmt: &StatementData, ctx: &MirContext) -> bool {
        match &stmt.kind {
            StatementKind::Assign { dest, rvalue } => self.matches_assign(dest, rvalue, ctx),
            StatementKind::Call { callee, args: _ } => self.matches_call(callee, ctx),
            StatementKind::Deinit { place } => self.matches_deinit(place, ctx),
            StatementKind::DeinitIf { place, flag } => self.matches_deinit_if(place, flag, ctx),
            StatementKind::SetDeinitFlag { flag, value } => {
                self.matches_set_deinit_flag(flag, *value, ctx)
            }
        }
    }

    fn matches_assign(
        &self,
        dest: &kestrel_execution_graph::Place,
        rvalue: &Rvalue,
        ctx: &MirContext,
    ) -> bool {
        match self {
            StatementPattern::Assign => true,

            StatementPattern::AssignTo(expected_name) => {
                // Check if destination local matches
                if let kestrel_execution_graph::PlaceKind::Local(local_id) = &dest.kind {
                    let local = ctx.local(*local_id);
                    local.name == *expected_name
                } else {
                    false
                }
            }

            StatementPattern::Copy => matches!(rvalue, Rvalue::Copy(_)),
            StatementPattern::Move => matches!(rvalue, Rvalue::Move(_)),
            StatementPattern::Ref => matches!(rvalue, Rvalue::Ref(_)),
            StatementPattern::RefMut => matches!(rvalue, Rvalue::RefMut(_)),

            StatementPattern::Construct { ty } => {
                if let Rvalue::Construct { ty: actual_ty, .. } = rvalue {
                    let actual_ty_str = ctx.ty(*actual_ty).display(ctx).to_string();
                    actual_ty_str == *ty
                } else {
                    false
                }
            }

            StatementPattern::EnumVariant { enum_ty, variant } => {
                if let Rvalue::EnumVariant {
                    enum_ty: actual_ty,
                    variant: actual_variant,
                    ..
                } = rvalue
                {
                    let actual_ty_str = ctx.ty(*actual_ty).display(ctx).to_string();
                    actual_ty_str == *enum_ty && actual_variant == variant
                } else {
                    false
                }
            }

            StatementPattern::Tuple { arity } => {
                if let Rvalue::Tuple(elements) = rvalue {
                    elements.len() == *arity
                } else {
                    false
                }
            }

            StatementPattern::Array => matches!(rvalue, Rvalue::Array { .. }),

            StatementPattern::BinOp(expected_op) => {
                if let Rvalue::BinaryOp { op, .. } = rvalue {
                    op == expected_op
                } else {
                    false
                }
            }

            StatementPattern::AnyBinOp => matches!(rvalue, Rvalue::BinaryOp { .. }),

            StatementPattern::UnOp(expected_op) => {
                if let Rvalue::UnaryOp { op, .. } = rvalue {
                    op == expected_op
                } else {
                    false
                }
            }

            StatementPattern::AnyUnOp => matches!(rvalue, Rvalue::UnaryOp { .. }),

            StatementPattern::Call { callee } => {
                if let Rvalue::Call {
                    callee: actual_callee,
                    ..
                } = rvalue
                {
                    self.callee_matches_name(actual_callee, callee, ctx)
                } else {
                    false
                }
            }

            StatementPattern::CallGeneric {
                callee,
                type_arg_count,
            } => {
                if let Rvalue::Call {
                    callee: actual_callee,
                    ..
                } = rvalue
                {
                    if let Callee::Direct { name, type_args } = actual_callee {
                        let actual_name = ctx.name(*name).to_string();
                        actual_name == *callee && type_args.len() == *type_arg_count
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            StatementPattern::CallWitness { protocol, method } => {
                if let Rvalue::Call {
                    callee: actual_callee,
                    ..
                } = rvalue
                {
                    if let Callee::Witness {
                        protocol: actual_protocol,
                        method: actual_method,
                        ..
                    } = actual_callee
                    {
                        let actual_protocol_name = ctx.name(*actual_protocol).to_string();
                        actual_protocol_name == *protocol && actual_method == method
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            StatementPattern::CallEscaping => {
                if let Rvalue::Call {
                    callee: actual_callee,
                    ..
                } = rvalue
                {
                    matches!(actual_callee, Callee::Thick(_))
                } else {
                    false
                }
            }

            StatementPattern::AnyCall => matches!(rvalue, Rvalue::Call { .. }),

            StatementPattern::FuncToEscaping { func } => {
                if let Rvalue::FuncToEscaping(name) = rvalue {
                    ctx.name(*name).to_string() == *func
                } else {
                    false
                }
            }

            StatementPattern::ApplyPartial { func, capture_count } => {
                if let Rvalue::ApplyPartial {
                    func: actual_func,
                    captures,
                } = rvalue
                {
                    ctx.name(*actual_func).to_string() == *func && captures.len() == *capture_count
                } else {
                    false
                }
            }

            StatementPattern::Cast { kind } => {
                if let Rvalue::Cast {
                    kind: actual_kind, ..
                } = rvalue
                {
                    actual_kind == kind
                } else {
                    false
                }
            }

            StatementPattern::CallWithModes { callee, arg_modes } => {
                if let Rvalue::Call {
                    callee: actual_callee,
                    args,
                } = rvalue
                {
                    // Check callee name matches
                    if !self.callee_matches_name(actual_callee, callee, ctx) {
                        return false;
                    }
                    // Check argument count matches
                    if args.len() != arg_modes.len() {
                        return false;
                    }
                    // Check each argument's passing mode
                    for (arg, expected_mode) in args.iter().zip(arg_modes.iter()) {
                        if arg.mode != *expected_mode {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            }

            StatementPattern::StrPtr => matches!(rvalue, Rvalue::StrPtr(_)),
            StatementPattern::StrLen => matches!(rvalue, Rvalue::StrLen(_)),
            StatementPattern::StrFromParts => matches!(rvalue, Rvalue::StrFromParts { .. }),
            StatementPattern::PtrOffset => matches!(rvalue, Rvalue::PtrOffset { .. }),
            StatementPattern::PtrToRef => matches!(rvalue, Rvalue::PtrToRef(_)),
            StatementPattern::PtrToRefMut => matches!(rvalue, Rvalue::PtrToRefMut(_)),
            StatementPattern::RefToPtr => matches!(rvalue, Rvalue::RefToPtr(_)),

            // Deinit patterns don't match assignments
            StatementPattern::Deinit { .. }
            | StatementPattern::AnyDeinit
            | StatementPattern::DeinitIf { .. }
            | StatementPattern::AnyDeinitIf
            | StatementPattern::SetDeinitFlag { .. }
            | StatementPattern::AnySetDeinitFlag
            | StatementPattern::DeinitCall { .. }
            | StatementPattern::AnyDeinitCall => false,
        }
    }

    fn matches_call(&self, callee: &Callee, ctx: &MirContext) -> bool {
        match self {
            StatementPattern::AnyCall => true,

            StatementPattern::Call { callee: expected } => {
                self.callee_matches_name(callee, expected, ctx)
            }

            StatementPattern::CallGeneric {
                callee: expected,
                type_arg_count,
            } => {
                if let Callee::Direct { name, type_args } = callee {
                    let actual_name = ctx.name(*name).to_string();
                    actual_name == *expected && type_args.len() == *type_arg_count
                } else {
                    false
                }
            }

            StatementPattern::CallWitness { protocol, method } => {
                if let Callee::Witness {
                    protocol: actual_protocol,
                    method: actual_method,
                    ..
                } = callee
                {
                    let actual_protocol_name = ctx.name(*actual_protocol).to_string();
                    actual_protocol_name == *protocol && actual_method == method
                } else {
                    false
                }
            }

            StatementPattern::CallEscaping => matches!(callee, Callee::Thick(_)),

            StatementPattern::AnyDeinitCall => {
                if let Callee::Direct { name, .. } = callee {
                    let callee_name = ctx.name(*name).to_string();
                    callee_name.ends_with(".deinit")
                } else {
                    false
                }
            }

            StatementPattern::DeinitCall { ty } => {
                if let Callee::Direct { name, .. } = callee {
                    let callee_name = ctx.name(*name).to_string();
                    let expected = format!("{}.deinit", ty);
                    callee_name == expected
                } else {
                    false
                }
            }

            _ => false,
        }
    }

    fn callee_matches_name(&self, callee: &Callee, expected: &str, ctx: &MirContext) -> bool {
        match callee {
            Callee::Direct { name, .. } => ctx.name(*name).to_string() == expected,
            _ => false,
        }
    }

    fn matches_deinit(&self, place: &kestrel_execution_graph::Place, ctx: &MirContext) -> bool {
        match self {
            StatementPattern::AnyDeinit => true,
            StatementPattern::Deinit { local } => {
                if let kestrel_execution_graph::PlaceKind::Local(local_id) = &place.kind {
                    let actual_local = ctx.local(*local_id);
                    actual_local.name == *local
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn matches_deinit_if(
        &self,
        place: &kestrel_execution_graph::Place,
        flag: &kestrel_execution_graph::Id<kestrel_execution_graph::Local>,
        ctx: &MirContext,
    ) -> bool {
        match self {
            StatementPattern::AnyDeinitIf => true,
            StatementPattern::DeinitIf {
                local: expected_local,
                flag: expected_flag,
            } => {
                let local_matches =
                    if let kestrel_execution_graph::PlaceKind::Local(local_id) = &place.kind {
                        let actual_local = ctx.local(*local_id);
                        actual_local.name == *expected_local
                    } else {
                        false
                    };
                let flag_matches = {
                    let actual_flag = ctx.local(*flag);
                    actual_flag.name == *expected_flag
                };
                local_matches && flag_matches
            }
            _ => false,
        }
    }

    fn matches_set_deinit_flag(
        &self,
        flag: &kestrel_execution_graph::Id<kestrel_execution_graph::Local>,
        value: bool,
        ctx: &MirContext,
    ) -> bool {
        match self {
            StatementPattern::AnySetDeinitFlag => true,
            StatementPattern::SetDeinitFlag {
                flag: expected_flag,
                value: expected_value,
            } => {
                let flag_matches = {
                    let actual_flag = ctx.local(*flag);
                    actual_flag.name == *expected_flag
                };
                flag_matches && value == *expected_value
            }
            _ => false,
        }
    }

    /// Format this pattern for display in error messages.
    pub(crate) fn display(&self) -> String {
        match self {
            StatementPattern::Assign => "any assignment".to_string(),
            StatementPattern::AssignTo(name) => format!("assignment to '{}'", name),
            StatementPattern::Copy => "copy".to_string(),
            StatementPattern::Move => "move".to_string(),
            StatementPattern::Ref => "ref".to_string(),
            StatementPattern::RefMut => "ref var".to_string(),
            StatementPattern::Construct { ty } => format!("construct {}", ty),
            StatementPattern::EnumVariant { enum_ty, variant } => {
                format!("enum {}.{}", enum_ty, variant)
            }
            StatementPattern::Tuple { arity } => format!("tuple of arity {}", arity),
            StatementPattern::Array => "array".to_string(),
            StatementPattern::BinOp(op) => format!("binop {:?}", op),
            StatementPattern::AnyBinOp => "any binop".to_string(),
            StatementPattern::UnOp(op) => format!("unop {:?}", op),
            StatementPattern::AnyUnOp => "any unop".to_string(),
            StatementPattern::Call { callee } => format!("call {}", callee),
            StatementPattern::CallGeneric {
                callee,
                type_arg_count,
            } => format!("call {}[{} type args]", callee, type_arg_count),
            StatementPattern::CallWitness { protocol, method } => {
                format!("witness call {}.{}", protocol, method)
            }
            StatementPattern::CallEscaping => "escaping call".to_string(),
            StatementPattern::AnyCall => "any call".to_string(),
            StatementPattern::FuncToEscaping { func } => format!("func.to.escaping {}", func),
            StatementPattern::ApplyPartial { func, capture_count } => {
                format!("apply partial {}({} captures)", func, capture_count)
            }
            StatementPattern::Cast { kind } => format!("cast {:?}", kind),
            StatementPattern::CallWithModes { callee, arg_modes } => {
                let modes_str: Vec<_> = arg_modes.iter().map(|m| m.as_str()).collect();
                format!("call {}({})", callee, modes_str.join(", "))
            }
            StatementPattern::StrPtr => "str.ptr".to_string(),
            StatementPattern::StrLen => "str.len".to_string(),
            StatementPattern::StrFromParts => "str.from_parts".to_string(),
            StatementPattern::PtrOffset => "ptr.offset".to_string(),
            StatementPattern::PtrToRef => "ptr.to.ref".to_string(),
            StatementPattern::PtrToRefMut => "ptr.to.ref_var".to_string(),
            StatementPattern::RefToPtr => "ref.to.ptr".to_string(),
            StatementPattern::Deinit { local } => format!("deinit %{}", local),
            StatementPattern::AnyDeinit => "any deinit".to_string(),
            StatementPattern::DeinitIf { local, flag } => {
                format!("deinit %{} if %{}", local, flag)
            }
            StatementPattern::AnyDeinitIf => "any conditional deinit".to_string(),
            StatementPattern::SetDeinitFlag { flag, value } => {
                format!("%{} = {}", flag, value)
            }
            StatementPattern::AnySetDeinitFlag => "any set deinit flag".to_string(),
            StatementPattern::DeinitCall { ty } => format!("call {}.deinit", ty),
            StatementPattern::AnyDeinitCall => "any deinit call".to_string(),
        }
    }
}
