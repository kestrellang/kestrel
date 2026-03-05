//! Path expression resolution.
//!
//! This module handles resolving path expressions (variable references, function
//! references, qualified names) including local variable lookup and module path resolution.

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_model::{
    ResolveTypePath, ResolveValuePath, SymbolFor, TypePathResolution, ValuePathResolution,
};
use kestrel_semantic_tree::expr::Expression;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyKind};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

use crate::diagnostics::{
    AmbiguousNameError, MaybeMovedError, NotGenericError, SelfOutsideInstanceMethodError,
    TooFewTypeArgumentsError, TooManyTypeArgumentsError, TypeArgsOnNonGenericError,
    UndefinedNameError, UseAfterMoveError,
};
use crate::resolution::type_resolver::TypeResolver;
use kestrel_syntax_tree::utils::get_node_span;

use super::context::BodyResolutionContext;
use super::expressions::resolve_expression;
use super::members::resolve_member_chain;
use super::utils::{get_callable_behavior, is_expression_kind};

/// Resolve a path expression (variable reference, function reference, or member access)
pub fn resolve_path_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Check for nested expression inside the path (happens with member access on call expressions)
    // e.g., `obj.method().field` is parsed as ExprPath containing ExprCall
    if let Some(nested_expr) = find_nested_expression(node) {
        let base = resolve_expression(&nested_expr, ctx);
        let trailing_members = extract_trailing_identifiers(node, ctx.source, ctx.file_id);
        if trailing_members.is_empty() {
            return base;
        }
        return resolve_member_chain(base, &trailing_members, ctx);
    }

    // Extract the path segments with their spans
    let path_with_spans = extract_path_segments_with_spans(node, ctx.source, ctx.file_id);

    if path_with_spans.is_empty() {
        return Expression::error(span);
    }

    // Extract just the names for lookups
    let path: Vec<String> = path_with_spans
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    let first_name = &path[0];
    let first_span = path_with_spans[0].1.clone();

    // First, check if it's a local variable
    if let Some(local_id) = ctx.local_scope.lookup(first_name) {
        // Check for use-after-move: if this variable has been moved, emit an error
        if let Some(moved_span) = ctx.move_tracker().get_move_span(local_id) {
            ctx.diagnostics.add_diagnostic(
                UseAfterMoveError {
                    use_span: first_span.clone(),
                    name: first_name.clone(),
                    moved_at: moved_span,
                }
                .into_diagnostic(),
            );
            return Expression::error(span);
        }

        // Check for use-after-maybe-move: if this variable may have been moved, emit an error
        if let Some(moved_span) = ctx.move_tracker().get_maybe_move_span(local_id) {
            ctx.diagnostics.add_diagnostic(
                MaybeMovedError {
                    use_span: first_span.clone(),
                    name: first_name.clone(),
                    moved_at: moved_span,
                }
                .into_diagnostic(),
            );
            return Expression::error(span);
        }

        // Check for type arguments on the variable itself (first segment only) - not allowed
        // Only check if this is a single-segment path (just `x[T]`), not `x.member[T]`
        // because has_type_arguments_on_first_segment has a fallback that can false-positive
        // on multi-segment paths where type args are on a later segment.
        if path_with_spans.len() == 1 && has_type_arguments_on_first_segment(node) {
            ctx.diagnostics.add_diagnostic(
                TypeArgsOnNonGenericError {
                    span: span.clone(),
                    callee_description: "a variable".to_string(),
                }
                .into_diagnostic(),
            );
            return Expression::error(span);
        }

        // Check for type arguments on the last segment of a multi-segment path.
        // e.g., `self.field[T]` or `x.member[T]` — brackets on member access are not valid.
        // BUT: skip this check if the path is the callee of a call expression (e.g., `ptr.cast[T]()`)
        // because the type args belong to the method call, handled by resolve_call_expression.
        if path_with_spans.len() > 1 && has_type_arguments_on_last_segment(node) && !is_callee_of_call(node) {
            ctx.diagnostics.add_diagnostic(
                TypeArgsOnNonGenericError {
                    span: span.clone(),
                    callee_description: "a member access".to_string(),
                }
                .into_diagnostic(),
            );
            return Expression::error(span);
        }

        // Get the type and mutability from the local
        let local = ctx.local_scope.get_local(local_id);
        let local_ty = local
            .as_ref()
            .map(|l: &kestrel_semantic_tree::symbol::local::Local| l.ty().clone())
            .unwrap_or_else(|| Ty::error(span.clone()));
        let is_mutable = local
            .as_ref()
            .map(|l: &kestrel_semantic_tree::symbol::local::Local| l.is_mutable())
            .unwrap_or(false);

        let base_expr = Expression::local_ref(local_id, local_ty, is_mutable, first_span);

        // If there are more segments, they are member accesses
        if path_with_spans.len() == 1 {
            return base_expr;
        } else {
            let result = resolve_member_chain(base_expr, &path_with_spans[1..], ctx);
            return result;
        }
    }

    // Check if this is 'self' being used outside an instance method
    if first_name == "self" {
        // 'self' was not found in local scope, which means we're not in an instance method
        let context = get_function_context(ctx);
        let error = SelfOutsideInstanceMethodError {
            span: first_span.clone(),
            context,
        };
        ctx.diagnostics.add_diagnostic(error.into_diagnostic());
        return Expression::error(span);
    }

    // Check for lang.* intrinsic functions
    // These are handled specially by the compiler and don't exist as real symbols
    if path.len() == 2 && path[0] == kestrel_prelude::lang::LANG {
        use kestrel_semantic_tree::expr::{LangIntrinsic, LangPrimitive};

        // lang.panic_unwind(message: String) -> Never
        // lang.panic is an alias for panic_unwind
        if path[1] == kestrel_prelude::lang::PANIC_UNWIND || path[1] == "panic" {
            return Expression::lang_intrinsic_ref(LangIntrinsic::PanicUnwind, span);
        }

        // lang.cast_<from>_<to>(value: From) -> To
        // e.g., lang.cast_i64_i32, lang.cast_f64_i64
        if let Some(suffix) = path[1].strip_prefix("cast_") {
            // Parse "i64_i32" -> (from: i64, to: i32)
            if let Some((from_str, to_str)) = suffix.split_once('_')
                && let (Some(from), Some(to)) = (
                    LangPrimitive::from_str(from_str),
                    LangPrimitive::from_str(to_str),
                )
            {
                return Expression::lang_intrinsic_ref(LangIntrinsic::Cast { from, to }, span);
            }
        }

        // Pointer intrinsics with type arguments
        // Extract type argument from path (e.g., cast_ptr[Int], sizeof[T])
        let type_arg =
            extract_type_arguments_from_path(node, ctx).and_then(|args| args.into_iter().next());
        let infer_ty = || Ty::infer(span.clone());

        match path[1].as_str() {
            "ptr_null" => {
                return Expression::lang_intrinsic_ref(
                    LangIntrinsic::PtrNull {
                        pointee_ty: type_arg.unwrap_or_else(infer_ty),
                    },
                    span,
                );
            },
            "ptr_from_address" => {
                return Expression::lang_intrinsic_ref(
                    LangIntrinsic::PtrFromAddress {
                        pointee_ty: type_arg.unwrap_or_else(infer_ty),
                    },
                    span,
                );
            },
            "ptr_to_address" => {
                return Expression::lang_intrinsic_ref(LangIntrinsic::PtrToAddress, span);
            },
            "ptr_to" => {
                return Expression::lang_intrinsic_ref(
                    LangIntrinsic::PtrTo {
                        pointee_ty: type_arg.unwrap_or_else(infer_ty),
                    },
                    span,
                );
            },
            "ptr_read" => {
                return Expression::lang_intrinsic_ref(
                    LangIntrinsic::PtrRead {
                        pointee_ty: type_arg.unwrap_or_else(infer_ty),
                    },
                    span,
                );
            },
            "ptr_write" => {
                return Expression::lang_intrinsic_ref(
                    LangIntrinsic::PtrWrite {
                        pointee_ty: type_arg.unwrap_or_else(infer_ty),
                    },
                    span,
                );
            },
            "ptr_offset" => {
                return Expression::lang_intrinsic_ref(LangIntrinsic::PtrOffset, span);
            },
            "ptr_is_null" => {
                return Expression::lang_intrinsic_ref(LangIntrinsic::PtrIsNull, span);
            },
            "cast_ptr" => {
                return Expression::lang_intrinsic_ref(
                    LangIntrinsic::CastPtr {
                        target_ty: type_arg.unwrap_or_else(infer_ty),
                    },
                    span,
                );
            },
            "sizeof" => {
                return Expression::lang_intrinsic_ref(
                    LangIntrinsic::SizeOf {
                        ty: type_arg.unwrap_or_else(infer_ty),
                    },
                    span,
                );
            },
            "alignof" => {
                return Expression::lang_intrinsic_ref(
                    LangIntrinsic::AlignOf {
                        ty: type_arg.unwrap_or_else(infer_ty),
                    },
                    span,
                );
            },
            // Boolean (i1) intrinsics
            "i1_eq" => {
                return Expression::lang_intrinsic_ref(LangIntrinsic::I1Eq, span);
            },
            "i1_and" => {
                return Expression::lang_intrinsic_ref(LangIntrinsic::I1And, span);
            },
            "i1_or" => {
                return Expression::lang_intrinsic_ref(LangIntrinsic::I1Or, span);
            },
            "i1_not" => {
                return Expression::lang_intrinsic_ref(LangIntrinsic::I1Not, span);
            },
            // Atomic intrinsics
            "atomic_add" => {
                return Expression::lang_intrinsic_ref(LangIntrinsic::AtomicAdd, span);
            },
            "atomic_sub" => {
                return Expression::lang_intrinsic_ref(LangIntrinsic::AtomicSub, span);
            },
            _ => {},
        }

        // Parse lang intrinsics: i64_add, i64_signed_div, f64_mul, etc.
        if let Some(intrinsic) = parse_lang_intrinsic(&path[1]) {
            return Expression::lang_intrinsic_ref(intrinsic, span);
        }
    }

    // Extract type arguments from the path if present
    let explicit_type_args = extract_type_arguments_from_path(node, ctx);

    // Not a local - resolve as a value path (module path)
    let resolution = ctx.model.query(ResolveValuePath {
        path: path.clone(),
        context: ctx.function_id,
    });
    // Debug: print what we're resolving
    // eprintln!("ResolveValuePath({:?}) = {:?}", path, resolution);
    match resolution {
        ValuePathResolution::Symbol { symbol_id, ty } => {
            // Check if this is an enum case - always use EnumCase expression
            // (both generic like Option[Int].None and non-generic like Color.Red)
            if let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id })
                && symbol.metadata().kind() == KestrelSymbolKind::EnumCase
            {
                // Check if this enum case is accessed via a qualified type path with explicit type args
                // e.g., Result[Int, Bool].Ok or Option[String].Some
                if let Some(qualified_ty) = extract_qualified_type_from_path(node, ctx)
                    && let Some((_, substitutions)) = qualified_ty.as_enum_with_subs()
                    && !substitutions.is_empty()
                {
                    // Apply substitutions to the type (callable type for cases with values,
                    // or enum type for simple cases)
                    let substituted_ty = ty.apply_substitutions(substitutions);

                    if get_callable_behavior(&symbol).is_some() {
                        // Enum case with associated values (like Ok, Some)
                        // Return SymbolRef with substituted callable type
                        return Expression::symbol_ref(symbol_id, substituted_ty, false, span);
                    } else {
                        // Simple enum case (like None)
                        return Expression::enum_case(symbol_id, substituted_ty, span);
                    }
                }

                // No explicit type args - handle simple cases without CallableBehavior
                if get_callable_behavior(&symbol).is_none() {
                    return Expression::enum_case(symbol_id, ty, span);
                }
            }

            // Check if this is a static method accessed via a qualified type path
            // e.g., Box[Int].wrap where wrap is a static method
            if let Some(qualified_ty) = extract_qualified_type_from_path(node, ctx)
                && let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id })
                && let Some(callable) = get_callable_behavior(&symbol)
                && callable.is_static()
            {
                // Get struct symbol from qualified type
                if let Some((struct_sym, _)) = qualified_ty.as_struct_with_subs() {
                    // Create TypeRef receiver with qualified type
                    let type_ref = Expression::type_ref(
                        struct_sym.metadata().id(),
                        qualified_ty,
                        span.clone(),
                    );
                    // Return MethodRef for call resolution to handle
                    let method_name = symbol.metadata().name().value.clone();
                    return Expression::method_ref(type_ref, vec![symbol_id], method_name, span);
                }
            }

            // Check if this is a field - fields need special handling for mutability
            if let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id })
                && symbol.metadata().kind() == KestrelSymbolKind::Field
            {
                use kestrel_semantic_tree::symbol::field::FieldSymbol;

                let is_mutable = symbol
                    .as_ref()
                    .downcast_ref::<FieldSymbol>()
                    .map(|f| f.is_mutable())
                    .unwrap_or(false);

                // For module-level fields, create a SymbolRef with proper mutability
                return Expression::symbol_ref(symbol_id, ty, is_mutable, span);
            }

            // Original handling for non-static-method cases
            // Check if type arguments were provided
            let final_ty = if let Some(ref type_args) = explicit_type_args {
                if !type_args.is_empty() {
                    // Apply type arguments to the function type
                    apply_type_args_to_function(symbol_id, &ty, type_args, &span, ctx).unwrap_or(ty)
                } else {
                    ty
                }
            } else {
                ty
            };

            // For now, module-level symbols (functions) are not mutable lvalues
            Expression::symbol_ref(symbol_id, final_ty, false, span)
        },
        ValuePathResolution::Overloaded { candidates } => {
            Expression::overloaded_ref(candidates, span)
        },
        ValuePathResolution::NotFound { segment, index } => {
            // Report undefined name error
            let error_span = if index < path_with_spans.len() {
                path_with_spans[index].1.clone()
            } else {
                first_span.clone()
            };
            let error = UndefinedNameError {
                span: error_span,
                name: segment,
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            Expression::error(span)
        },
        ValuePathResolution::Ambiguous {
            segment,
            index,
            candidates,
        } => {
            // Report ambiguous name error
            let error_span = if index < path_with_spans.len() {
                path_with_spans[index].1.clone()
            } else {
                first_span.clone()
            };
            let error = AmbiguousNameError {
                span: error_span,
                name: segment,
                candidate_count: candidates.len(),
            };
            ctx.diagnostics.add_diagnostic(error.into_diagnostic());
            Expression::error(span)
        },
        ValuePathResolution::NotAValue { symbol_id } => {
            // Check if this is a field that needs implicit self access
            if let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id })
                && symbol.metadata().kind() == KestrelSymbolKind::Field
            {
                // Check if this is a static field - static fields don't use implicit self
                use kestrel_semantic_tree::behavior::StaticBehavior;
                let is_static = symbol.metadata().get_behavior::<StaticBehavior>().is_some();

                // This is a field reference like `x` that should become `self.x`
                // Look for 'self' in local scope (but not for static fields)
                if !is_static && let Some(self_local_id) = ctx.local_scope.lookup("self") {
                    let self_local = ctx.local_scope.get_local(self_local_id);
                    let self_ty = self_local
                        .as_ref()
                        .map(|l: &kestrel_semantic_tree::symbol::local::Local| l.ty().clone())
                        .unwrap_or_else(|| Ty::error(span.clone()));
                    let self_mutable = self_local
                        .as_ref()
                        .map(|l: &kestrel_semantic_tree::symbol::local::Local| l.is_mutable())
                        .unwrap_or(false);

                    // Create self reference
                    let self_expr =
                        Expression::local_ref(self_local_id, self_ty, self_mutable, span.clone());

                    // Get field type and mutability from FieldSymbol
                    use kestrel_semantic_tree::symbol::field::FieldSymbol;
                    let (field_ty, field_mutable) = symbol
                        .downcast_ref::<FieldSymbol>()
                        .map(|f| (f.field_type().clone(), f.is_mutable()))
                        .unwrap_or_else(|| (Ty::error(span.clone()), false));

                    let field_name = symbol.metadata().name().value.clone();
                    return Expression::field_access(
                        self_expr,
                        field_name,
                        field_mutable,
                        field_ty,
                        span,
                    );
                }
                // If no self, fall through to create type_ref
                // For static fields, we need to handle them specially
                if is_static {
                    // Get the parent (struct/enum) to create a TypeRef base
                    if let Some(parent) = symbol.metadata().parent() {
                        use super::utils::create_struct_type_with_type_args;
                        use kestrel_semantic_tree::behavior::typed::TypedBehavior;
                        use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;

                        let parent_id = parent.metadata().id();

                        // Try to create parent type - handle both struct and enum
                        let parent_ty = if let Ok(struct_sym) =
                            parent.clone().downcast_arc::<StructSymbol>()
                        {
                            // Parent is a struct
                            create_struct_type_with_type_args(
                                &(struct_sym
                                    as std::sync::Arc<
                                        dyn Symbol<
                                            kestrel_semantic_tree::language::KestrelLanguage,
                                        >,
                                    >),
                                &[],
                                span.clone(),
                                ctx,
                            )
                        } else if let Ok(enum_sym) = parent.clone().downcast_arc::<EnumSymbol>() {
                            // Parent is an enum
                            Ty::r#enum(enum_sym, span.clone())
                        } else {
                            // Unknown parent type - fallback to inference
                            Ty::infer(span.clone())
                        };

                        // Create TypeRef for the parent type
                        let type_ref = Expression::type_ref(parent_id, parent_ty, span.clone());

                        // Get field type from TypedBehavior
                        let field_ty = symbol
                            .metadata()
                            .get_behavior::<TypedBehavior>()
                            .map(|tb| tb.ty().clone())
                            .unwrap_or_else(|| Ty::error(span.clone()));

                        let field_name = symbol.metadata().name().value.clone();

                        // Get field mutability from FieldSymbol
                        use kestrel_semantic_tree::symbol::field::FieldSymbol;
                        let field_mutable = symbol
                            .as_ref()
                            .downcast_ref::<FieldSymbol>()
                            .map(|f| f.is_mutable())
                            .unwrap_or(false);

                        return Expression::field_access(
                            type_ref,
                            field_name,
                            field_mutable, // use actual mutability of the field
                            field_ty,
                            span,
                        );
                    }
                }
            }
            // This is a type reference (e.g., struct name) - may be used for initialization
            // For generic types, create struct type with inference variables for each type param
            // This enables proper type inference when the struct is used without explicit type args
            use super::utils::create_struct_type_with_type_args;
            let ty = ctx
                .model
                .query(SymbolFor { id: symbol_id })
                .and_then(|symbol| {
                    symbol
                        .clone()
                        .downcast_arc::<StructSymbol>()
                        .ok()
                        .map(|struct_sym| {
                            create_struct_type_with_type_args(
                                &(struct_sym
                                    as std::sync::Arc<
                                        dyn Symbol<
                                            kestrel_semantic_tree::language::KestrelLanguage,
                                        >,
                                    >),
                                &[],
                                span.clone(),
                                ctx,
                            )
                        })
                })
                .unwrap_or_else(|| Ty::infer(span.clone()));
            Expression::type_ref(symbol_id, ty, span)
        },
        ValuePathResolution::TypeParameter { symbol_id } => {
            // This is a type parameter reference (e.g., T in `T()` or `T.create()`)
            // For multi-segment paths like T.create, the db returns TypeParameter
            // for just the first segment, and we need to handle the rest as member accesses

            // Look up the type parameter symbol to create proper type
            let type_param_ty = if let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id }) {
                if let Ok(type_param_arc) = symbol.clone().downcast_arc::<TypeParameterSymbol>() {
                    Ty::type_parameter(type_param_arc, first_span.clone())
                } else {
                    Ty::infer(first_span.clone())
                }
            } else {
                Ty::infer(first_span.clone())
            };

            let base = Expression::type_parameter_ref(symbol_id, type_param_ty, first_span.clone());

            // If there are more segments, resolve them as member accesses
            if path_with_spans.len() > 1 {
                resolve_member_chain(base, &path_with_spans[1..], ctx)
            } else {
                base
            }
        },
        ValuePathResolution::AssociatedType {
            symbol_id,
            container,
        } => {
            // This is an associated type reference (e.g., Item in `Item.zero`)
            // For multi-segment paths like Item.zero, the db returns AssociatedType
            // for just the first segment, and we need to handle the rest as member accesses
            // This enables accessing static members from protocol bounds (e.g., Addable.zero)

            // Look up the associated type symbol to create proper type
            let assoc_type_ty = if let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id }) {
                if let Ok(assoc_type_arc) = symbol.clone().downcast_arc::<AssociatedTypeSymbol>() {
                    // Create the associated type with its container
                    match container {
                        Some(container_ty) => Ty::qualified_associated_type(
                            assoc_type_arc,
                            container_ty,
                            first_span.clone(),
                        ),
                        None => Ty::associated_type(assoc_type_arc, first_span.clone()),
                    }
                } else {
                    Ty::infer(first_span.clone())
                }
            } else {
                Ty::infer(first_span.clone())
            };

            let base = Expression::associated_type_ref(assoc_type_ty, first_span.clone());

            // If there are more segments, resolve them as member accesses
            if path_with_spans.len() > 1 {
                resolve_member_chain(base, &path_with_spans[1..], ctx)
            } else {
                base
            }
        },
        ValuePathResolution::EnumCaseValue {
            symbol_id,
            ty,
            resolved_index,
        } => {
            // This is an enum case value followed by more path segments
            // e.g., `Player.player1.description()` where player1 is a case
            // and description is a method on the enum type
            let case_span = if resolved_index < path_with_spans.len() {
                path_with_spans[resolved_index].1.clone()
            } else {
                first_span.clone()
            };

            // Create the enum case expression
            let base = Expression::enum_case(symbol_id, ty, case_span);

            // Resolve remaining segments as member accesses
            if resolved_index + 1 < path_with_spans.len() {
                resolve_member_chain(base, &path_with_spans[resolved_index + 1..], ctx)
            } else {
                base
            }
        },
        ValuePathResolution::FieldValue {
            symbol_id,
            ty,
            resolved_index,
        } => {
            // This is a field/getter value followed by more path segments
            // e.g., `Float64.e.subtract(1.0)` where `e` is a static field
            // and `subtract()` is a method call on that value
            let field_span = if resolved_index < path_with_spans.len() {
                path_with_spans[resolved_index].1.clone()
            } else {
                first_span.clone()
            };

            // Build the field access expression the same way as static field access
            // in the NotAValue arm: create a TypeRef for the parent, then field_access
            if let Some(symbol) = ctx.model.query(SymbolFor { id: symbol_id }) {
                use kestrel_semantic_tree::symbol::field::FieldSymbol;

                let is_mutable = symbol
                    .as_ref()
                    .downcast_ref::<FieldSymbol>()
                    .map(|f| f.is_mutable())
                    .unwrap_or(false);

                // Get the parent type to build a TypeRef base
                if let Some(parent) = symbol.metadata().parent() {
                    use super::utils::create_struct_type_with_type_args;
                    use kestrel_semantic_tree::behavior::typed::TypedBehavior;
                    use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;

                    let parent_id = parent.metadata().id();

                    let parent_ty = if let Ok(struct_sym) =
                        parent.clone().downcast_arc::<StructSymbol>()
                    {
                        create_struct_type_with_type_args(
                            &(struct_sym
                                as std::sync::Arc<
                                    dyn Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
                                >),
                            &[],
                            field_span.clone(),
                            ctx,
                        )
                    } else if let Ok(enum_sym) = parent.clone().downcast_arc::<EnumSymbol>() {
                        Ty::r#enum(enum_sym, field_span.clone())
                    } else {
                        Ty::infer(field_span.clone())
                    };

                    let type_ref = Expression::type_ref(parent_id, parent_ty, field_span.clone());

                    let field_ty = symbol
                        .metadata()
                        .get_behavior::<TypedBehavior>()
                        .map(|tb| tb.ty().clone())
                        .unwrap_or_else(|| ty.clone());

                    let field_name = symbol.metadata().name().value.clone();

                    let base = Expression::field_access(
                        type_ref, field_name, is_mutable, field_ty, field_span,
                    );

                    // Resolve remaining segments as member accesses
                    if resolved_index + 1 < path_with_spans.len() {
                        resolve_member_chain(base, &path_with_spans[resolved_index + 1..], ctx)
                    } else {
                        base
                    }
                } else {
                    // No parent - fall back to simple symbol ref
                    let base = Expression::symbol_ref(symbol_id, ty, is_mutable, field_span);
                    if resolved_index + 1 < path_with_spans.len() {
                        resolve_member_chain(base, &path_with_spans[resolved_index + 1..], ctx)
                    } else {
                        base
                    }
                }
            } else {
                // Symbol not found - error
                Expression::error(span)
            }
        },
    }
}

/// Get a description of the function context for error messages.
///
/// Returns descriptions like "static method", "free function", etc.
fn get_function_context(ctx: &BodyResolutionContext) -> String {
    let Some(function) = ctx.model.query(SymbolFor {
        id: ctx.function_id,
    }) else {
        return "this context".to_string();
    };

    // Check if the function is in a struct or protocol
    let parent = function.metadata().parent();
    match parent.as_ref().map(|p| p.metadata().kind()) {
        Some(KestrelSymbolKind::Struct) | Some(KestrelSymbolKind::Protocol) => {
            // It's a method - check if static
            // We can check by looking for 'self' in local scope, but we already know
            // 'self' wasn't found, so this must be a static method
            "static method".to_string()
        },
        _ => {
            // Not in a struct/protocol, so it's a free function
            "free function".to_string()
        },
    }
}

/// Extract path segments with their spans from a path expression node
fn extract_path_segments_with_spans(
    node: &SyntaxNode,
    source: &str,
    file_id: usize,
) -> Vec<(String, Span)> {
    let mut segments = Vec::new();

    // ExprPath may contain Path or direct PathElements
    if let Some(path_node) = node.children().find(|c| c.kind() == SyntaxKind::Path) {
        // Path contains PathElements
        for element in path_node.children() {
            if element.kind() == SyntaxKind::PathElement
                && let Some((name, span)) =
                    extract_path_element_name_with_span(&element, source, file_id)
            {
                segments.push((name, span));
            }
        }
    } else {
        // Direct identifiers
        for child in node.children() {
            if child.kind() == SyntaxKind::PathElement
                && let Some((name, span)) =
                    extract_path_element_name_with_span(&child, source, file_id)
            {
                segments.push((name, span));
            }
        }

        // Fallback: look for Name or Identifier tokens
        if segments.is_empty() {
            for elem in node.children_with_tokens() {
                if let Some(token) = elem.as_token()
                    && token.kind() == SyntaxKind::Identifier
                {
                    let range = token.text_range();
                    let name = token.text().to_string();
                    let range_start: usize = range.start().into();
                    let range_end: usize = range.end().into();

                    // Validate span
                    let span =
                        if range_end <= source.len() && source[range_start..range_end] == name {
                            Span::new(file_id, range_start..range_end)
                        } else {
                            // Invalid span - use node range as fallback
                            let node_range = node.text_range();
                            let node_start: usize = node_range.start().into();
                            let node_end: usize = node_range.end().into();
                            if node_end <= source.len() {
                                Span::new(file_id, node_start..node_end)
                            } else {
                                Span::new(file_id, 0..0)
                            }
                        };

                    segments.push((name, span));
                }
            }
        }
    }

    segments
}

/// Extract the name and span from a PathElement node
///
/// Validates that the extracted span actually points to the token text in the source.
/// If the span is invalid (out of bounds or mismatched text), uses a zero-length
/// span at position 0 as a fallback to prevent incorrect error locations.
fn extract_path_element_name_with_span(
    element: &SyntaxNode,
    source: &str,
    file_id: usize,
) -> Option<(String, Span)> {
    // Helper to validate and potentially correct a span
    let validate_span = |name: &str, range_start: usize, range_end: usize| -> Span {
        // Check if span is within source bounds and matches the token text
        if range_end <= source.len() {
            let text_at_span = &source[range_start..range_end];
            if text_at_span == name {
                // Span is valid
                return Span::new(file_id, range_start..range_end);
            }
        }
        // Span is invalid - use a fallback based on the element's range
        // This at least keeps the span in a reasonable location
        let elem_range = element.text_range();
        let elem_start: usize = elem_range.start().into();
        let elem_end: usize = elem_range.end().into();
        if elem_end <= source.len() {
            Span::new(file_id, elem_start..elem_end)
        } else {
            // Last resort: use a zero-length span at a reasonable position
            // Use source length as the position to at least point to end of file
            let end_pos = source.len().saturating_sub(1);
            Span::new(file_id, end_pos..end_pos)
        }
    };

    // PathElement contains Name or Identifier
    if let Some(name_node) = element.children().find(|c| c.kind() == SyntaxKind::Name) {
        return name_node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)
            .map(|t| {
                let range = t.text_range();
                let name = t.text().to_string();
                let span = validate_span(&name, range.start().into(), range.end().into());
                (name, span)
            });
    }

    // Direct Identifier token
    element
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| {
            let range = t.text_range();
            let name = t.text().to_string();
            let span = validate_span(&name, range.start().into(), range.end().into());
            (name, span)
        })
}

/// Find a nested expression inside a path node
///
/// This handles the case where member access on a call expression is emitted as
/// an ExprPath containing an Expression/ExprCall child.
/// e.g., `obj.method().field` is parsed as ExprPath containing ExprCall
fn find_nested_expression(node: &SyntaxNode) -> Option<SyntaxNode> {
    // We're looking inside an ExprPath node. Normally it contains only identifiers and dots.
    // But when member access is on a complex expression, the parser emits it inside the ExprPath.
    // We need to find such nested expressions (calls, groupings, literals, etc.).

    fn is_complex_expression(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::ExprCall
                | SyntaxKind::ExprGrouping
                | SyntaxKind::ExprBinary
                // Include literal expressions to support method calls on literals
                // like "hello".toCString() or 42.toString()
                | SyntaxKind::ExprString
                | SyntaxKind::ExprRawString
                | SyntaxKind::ExprInteger
                | SyntaxKind::ExprFloat
                | SyntaxKind::ExprChar
                | SyntaxKind::ExprBool
                | SyntaxKind::ExprArray
                | SyntaxKind::ExprDictionary
                | SyntaxKind::ExprTuple
                | SyntaxKind::ExprTupleIndex
        )
    }

    for child in node.children() {
        // Look for Expression wrapper containing a non-path expression
        if child.kind() == SyntaxKind::Expression {
            // Check if this Expression contains a complex (non-path) expression
            for inner in child.children() {
                // Return if it's a complex expression type, not just another path
                if is_complex_expression(inner.kind()) {
                    return Some(child);
                }
            }
        }
        // Also check for direct complex expression nodes
        if is_complex_expression(child.kind()) {
            return Some(child);
        }
    }
    None
}

/// Extract trailing identifier tokens after a nested expression in a path
///
/// When a path contains a nested expression (e.g., from member access on a call),
/// this extracts the identifiers that appear after the expression.
fn extract_trailing_identifiers(
    node: &SyntaxNode,
    source: &str,
    file_id: usize,
) -> Vec<(String, Span)> {
    let mut identifiers = Vec::new();
    let mut found_expression = false;

    for elem in node.children_with_tokens() {
        if let Some(child) = elem.as_node() {
            // Mark when we see the nested expression
            if child.kind() == SyntaxKind::Expression || is_expression_kind(child.kind()) {
                found_expression = true;
            }
        } else if let Some(token) = elem.as_token() {
            // Only collect identifiers after the expression
            if found_expression && token.kind() == SyntaxKind::Identifier {
                let range = token.text_range();
                let name = token.text().to_string();
                let range_start: usize = range.start().into();
                let range_end: usize = range.end().into();

                // Validate span
                let span = if range_end <= source.len() && source[range_start..range_end] == name {
                    Span::new(file_id, range_start..range_end)
                } else {
                    // Invalid span - use node range as fallback
                    let node_range = node.text_range();
                    let node_start: usize = node_range.start().into();
                    let node_end: usize = node_range.end().into();
                    if node_end <= source.len() {
                        Span::new(file_id, node_start..node_end)
                    } else {
                        Span::new(file_id, 0..0)
                    }
                };

                identifiers.push((name, span));
            }
        }
    }

    identifiers
}

/// Check if a path expression contains type arguments on the first segment
/// This is for detecting `x[T]` where x is a variable
fn has_type_arguments_on_first_segment(node: &SyntaxNode) -> bool {
    // For a path like `x[T]`, we look for TypeArgumentList directly in the first PathElement
    // or directly in the ExprPath if there's no Path wrapper

    // First check if there's a Path child
    if let Some(path_node) = node.children().find(|c| c.kind() == SyntaxKind::Path) {
        // Get the first PathElement
        if let Some(first_elem) = path_node
            .children()
            .find(|c| c.kind() == SyntaxKind::PathElement)
        {
            return first_elem
                .children()
                .any(|c| c.kind() == SyntaxKind::TypeArgumentList);
        }
    }

    // Also check directly in ExprPath for simpler paths
    for child in node.children() {
        if child.kind() == SyntaxKind::PathElement {
            if child
                .children()
                .any(|c| c.kind() == SyntaxKind::TypeArgumentList)
            {
                return true;
            }
            // Only check the first PathElement
            return false;
        }
        if child.kind() == SyntaxKind::TypeArgumentList {
            return true;
        }
    }

    false
}

/// Check if a path expression contains type arguments on the last segment.
/// This is for detecting `x.field[T]` where the brackets are on a member access.
fn has_type_arguments_on_last_segment(node: &SyntaxNode) -> bool {
    // Check if there's a Path child
    if let Some(path_node) = node.children().find(|c| c.kind() == SyntaxKind::Path) {
        // Get the last PathElement
        if let Some(last_elem) = path_node
            .children()
            .filter(|c| c.kind() == SyntaxKind::PathElement)
            .last()
        {
            return last_elem
                .children()
                .any(|c| c.kind() == SyntaxKind::TypeArgumentList);
        }
    }

    // Also check directly in ExprPath for simpler paths
    let mut last_element: Option<SyntaxNode> = None;
    for child in node.children() {
        if child.kind() == SyntaxKind::PathElement {
            last_element = Some(child);
        }
    }
    if let Some(elem) = last_element {
        return elem
            .children()
            .any(|c| c.kind() == SyntaxKind::TypeArgumentList);
    }

    // Check for TypeArgumentList directly (no PathElement wrapper)
    let mut has_type_args = false;
    for child in node.children() {
        if child.kind() == SyntaxKind::TypeArgumentList {
            has_type_args = true;
        }
    }
    has_type_args
}

/// Check if this path expression node is the callee of a call expression.
/// e.g., in `ptr.cast[T]()`, the ExprPath `ptr.cast[T]` is the callee of ExprCall.
/// We check by looking at the parent (or grandparent) node for ExprCall.
fn is_callee_of_call(node: &SyntaxNode) -> bool {
    // The ExprPath may be directly inside ExprCall, or wrapped in an Expression node first
    if let Some(parent) = node.parent() {
        if parent.kind() == SyntaxKind::ExprCall {
            return true;
        }
        // Check grandparent (ExprPath > Expression > ExprCall)
        if parent.kind() == SyntaxKind::Expression {
            if let Some(grandparent) = parent.parent() {
                if grandparent.kind() == SyntaxKind::ExprCall {
                    return true;
                }
            }
        }
    }
    false
}

/// Extract type arguments from a path expression node.
///
/// Handles paths like `identity[String]` or `module.func[Int, Bool]`.
/// Returns None if no type arguments are present, or Some(vec) with the resolved types.
///
/// IMPORTANT: Only extracts type arguments from the FINAL path segment.
/// For `Box[Int].zero`, the type args `[Int]` belong to `Box`, not to `zero`.
/// Type args on intermediate segments are handled during type resolution of those segments.
///
/// ExprPath structure for "Box[Int].zero":
/// ExprPath
///   Identifier "Box"
///   TypeArgumentList [Int]
///   Dot
///   Identifier "zero"
///
/// We want to only extract type args that come AFTER the last dot (i.e., on the final segment).
fn extract_type_arguments_from_path(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Vec<Ty>> {
    // Look for TypeArgumentList only in the FINAL path segment
    fn find_type_args_on_final_segment(node: &SyntaxNode) -> Option<SyntaxNode> {
        // Find the ExprPath node (either this node or a child)
        let expr_path = if node.kind() == SyntaxKind::ExprPath {
            Some(node.clone())
        } else {
            node.children().find(|c| c.kind() == SyntaxKind::ExprPath)
        };

        if let Some(expr_path) = expr_path {
            // Collect all children to analyze the structure
            let children: Vec<_> = expr_path.children_with_tokens().collect();

            // Find the last Dot token position (if any)
            let mut last_dot_pos = None;
            for (i, child) in children.iter().enumerate() {
                if let Some(token) = child.as_token()
                    && token.kind() == SyntaxKind::Dot
                {
                    last_dot_pos = Some(i);
                }
            }

            // If there's a dot, only look for TypeArgumentList AFTER the last dot
            if let Some(dot_pos) = last_dot_pos {
                for child in children.iter().skip(dot_pos + 1) {
                    if let Some(node) = child.as_node()
                        && node.kind() == SyntaxKind::TypeArgumentList
                    {
                        return Some(node.clone());
                    }
                }
                // Multi-segment path but no type args after last dot
                return None;
            }

            // No dot - single segment path, check for direct TypeArgumentList
            for child in children.iter() {
                if let Some(node) = child.as_node()
                    && node.kind() == SyntaxKind::TypeArgumentList
                {
                    return Some(node.clone());
                }
            }

            return None;
        }

        // For Path nodes (used in type paths), check PathElements
        if let Some(path_node) = node.children().find(|c| c.kind() == SyntaxKind::Path) {
            let path_elements: Vec<_> = path_node
                .children()
                .filter(|c| c.kind() == SyntaxKind::PathElement)
                .collect();

            // For multi-segment paths, only extract type args from the LAST element
            if path_elements.len() > 1 {
                if let Some(last_element) = path_elements.last() {
                    for child in last_element.children() {
                        if child.kind() == SyntaxKind::TypeArgumentList {
                            return Some(child);
                        }
                    }
                }
                return None;
            }

            // Single element path
            if let Some(only_element) = path_elements.first() {
                for child in only_element.children() {
                    if child.kind() == SyntaxKind::TypeArgumentList {
                        return Some(child);
                    }
                }
            }
            return None;
        }

        // Also check direct PathElements for simpler paths
        let path_elements: Vec<_> = node
            .children()
            .filter(|c| c.kind() == SyntaxKind::PathElement)
            .collect();

        // For multi-segment paths, only extract type args from the LAST element
        if path_elements.len() > 1 {
            if let Some(last_element) = path_elements.last() {
                for inner in last_element.children() {
                    if inner.kind() == SyntaxKind::TypeArgumentList {
                        return Some(inner);
                    }
                }
            }
            return None;
        }

        if let Some(only_element) = path_elements.first() {
            for inner in only_element.children() {
                if inner.kind() == SyntaxKind::TypeArgumentList {
                    return Some(inner);
                }
            }
        }

        // Only check direct TypeArgumentList if no path structure found
        if path_elements.is_empty() {
            for child in node.children() {
                if child.kind() == SyntaxKind::TypeArgumentList {
                    return Some(child);
                }
            }
        }

        None
    }

    let type_arg_list = find_type_args_on_final_segment(node)?;

    // Resolve each type in the TypeArgumentList
    let mut type_args = Vec::new();

    for child in type_arg_list.children() {
        if child.kind() == SyntaxKind::Ty {
            let mut resolver = TypeResolver::new(
                ctx.model,
                ctx.diagnostics,
                ctx.source,
                ctx.file_id,
                ctx.function_id,
            );
            let ty = resolver.resolve(&child);
            type_args.push(ty);
        }
    }

    // Return Some even if empty - the presence of [] means explicit type args were provided
    Some(type_args)
}

/// Apply type arguments to a function type, returning the instantiated type.
///
/// This validates that:
/// - The symbol is a generic function
/// - The number of type arguments matches the number of type parameters
///
/// Returns None if type arguments can't be applied (with diagnostics emitted).
fn apply_type_args_to_function(
    symbol_id: semantic_tree::symbol::SymbolId,
    _original_ty: &Ty,
    type_args: &[Ty],
    span: &Span,
    ctx: &mut BodyResolutionContext,
) -> Option<Ty> {
    // Get the symbol
    let symbol = ctx.model.query(SymbolFor { id: symbol_id })?;

    // Check if it's a function with type parameters
    let func_sym = symbol.as_any().downcast_ref::<FunctionSymbol>()?;
    let type_params = func_sym.type_parameters();
    let function_name = symbol.metadata().name().value.clone();

    // Validate: function must be generic if type args are provided
    if type_params.is_empty() {
        ctx.diagnostics.add_diagnostic(
            NotGenericError {
                span: span.clone(),
                type_name: function_name,
            }
            .into_diagnostic(),
        );
        return None;
    }

    // Validate: type arg count must match type param count
    if type_args.len() < type_params.len() {
        ctx.diagnostics.add_diagnostic(
            TooFewTypeArgumentsError {
                span: span.clone(),
                type_name: function_name,
                min_expected: type_params.len(),
                got: type_args.len(),
            }
            .into_diagnostic(),
        );
        return None;
    }

    if type_args.len() > type_params.len() {
        ctx.diagnostics.add_diagnostic(
            TooManyTypeArgumentsError {
                span: span.clone(),
                type_name: function_name,
                max_expected: type_params.len(),
                got: type_args.len(),
            }
            .into_diagnostic(),
        );
        return None;
    }

    // Build substitutions from type parameters to provided type arguments
    let mut substitutions = Substitutions::new();
    for (param, arg_ty) in type_params.iter().zip(type_args.iter()) {
        substitutions.insert(param.metadata().id(), arg_ty.clone());
    }

    // Get the callable behavior to get the function type
    let callable = get_callable_behavior(&symbol)?;

    // Build the instantiated function type
    let params: Vec<Ty> = callable
        .parameters()
        .iter()
        .map(|p| p.ty.apply_substitutions(&substitutions))
        .collect();
    let return_type = callable.return_type().apply_substitutions(&substitutions);

    Some(Ty::function(params, return_type, span.clone()))
}

/// Extract the qualified type from an intermediate path segment.
///
/// For `Box[Int].wrap`, this extracts the `Box[Int]` type from the first segment.
/// For `Box.wrap`, this returns `Box` with infer type parameters.
/// For single-segment paths like `wrap`, returns None.
///
/// This is used to capture type arguments on intermediate path segments
/// (before the final segment) so they can be used for type parameter substitution
/// when calling static methods.
fn extract_qualified_type_from_path(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Ty> {
    // Find the ExprPath node
    let expr_path = if node.kind() == SyntaxKind::ExprPath {
        node.clone()
    } else {
        node.children().find(|c| c.kind() == SyntaxKind::ExprPath)?
    };

    let children: Vec<_> = expr_path.children_with_tokens().collect();

    // Find TypeArgumentList anywhere in the path.
    // For paths like `std.memory.Pointer[Int64].nullPointer`, the TypeArgumentList
    // appears after the type name, potentially deep in a multi-segment path.
    let type_arg_pos = children.iter().position(|c| {
        c.as_node()
            .map(|n| n.kind() == SyntaxKind::TypeArgumentList)
            .unwrap_or(false)
    });

    // Verify there's a dot AFTER the TypeArgumentList (meaning there's a member access).
    // If the TypeArgumentList is at the end, this is just a type reference, not qualified member access.
    let has_trailing_member = if let Some(pos) = type_arg_pos {
        children[pos + 1..].iter().any(|c| {
            c.as_token()
                .map(|t| t.kind() == SyntaxKind::Dot)
                .unwrap_or(false)
        })
    } else {
        false
    };

    // If no TypeArgumentList with trailing member, check for generic type without explicit args
    if !has_trailing_member {
        // Find if there's a dot (multi-segment path)
        let has_dot = children.iter().any(|c| {
            c.as_token()
                .map(|t| t.kind() == SyntaxKind::Dot)
                .unwrap_or(false)
        });
        if !has_dot {
            return None;
        }

        // Get the first identifier for single-segment type name fallback
        let first_ident = children
            .iter()
            .filter_map(|c| c.as_token())
            .find(|t| t.kind() == SyntaxKind::Identifier)?;
        let type_name = first_ident.text().to_string();

        let _span = get_node_span(node, ctx.file_id);
        let base_ty = match ctx.model.query(ResolveTypePath {
            path: vec![type_name],
            context: ctx.function_id,
        }) {
            TypePathResolution::Resolved(ty) => ty,
            _ => return None,
        };

        // Only return Some for generic types that need inference
        return match base_ty.kind() {
            TyKind::Struct { symbol, .. } if !symbol.type_parameters().is_empty() => Some(base_ty),
            TyKind::Enum { symbol, .. } if !symbol.type_parameters().is_empty() => Some(base_ty),
            _ => None,
        };
    }

    // We have a TypeArgumentList with trailing member access.
    // Collect all identifiers BEFORE the TypeArgumentList to form the type path.
    let type_arg_pos = type_arg_pos.unwrap();
    let type_path: Vec<String> = children[..type_arg_pos]
        .iter()
        .filter_map(|c| c.as_token())
        .filter(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())
        .collect();

    if type_path.is_empty() {
        return None;
    }

    // Resolve the type path
    let span = get_node_span(node, ctx.file_id);
    let base_ty = match ctx.model.query(ResolveTypePath {
        path: type_path,
        context: ctx.function_id,
    }) {
        TypePathResolution::Resolved(ty) => ty,
        _ => return None,
    };

    // Extract and apply type arguments from the TypeArgumentList
    let type_arg_node = children[type_arg_pos].as_node().unwrap();
    let mut type_args = Vec::new();
    for child in type_arg_node.children() {
        if child.kind() == SyntaxKind::Ty {
            let mut resolver = TypeResolver::new(
                ctx.model,
                ctx.diagnostics,
                ctx.source,
                ctx.file_id,
                ctx.function_id,
            );
            type_args.push(resolver.resolve(&child));
        }
    }

    if !type_args.is_empty() {
        let mut resolver = TypeResolver::new(
            ctx.model,
            ctx.diagnostics,
            ctx.source,
            ctx.file_id,
            ctx.function_id,
        );
        Some(resolver.apply_type_arguments(&base_ty, type_args, span))
    } else {
        // TypeArgumentList present but empty - return base type for generic inference
        match base_ty.kind() {
            TyKind::Struct { symbol, .. } if !symbol.type_parameters().is_empty() => Some(base_ty),
            TyKind::Enum { symbol, .. } if !symbol.type_parameters().is_empty() => Some(base_ty),
            _ => None,
        }
    }
}

/// Parse a lang intrinsic name like "i64_add", "i64_signed_div", "f64_mul", etc.
fn parse_lang_intrinsic(name: &str) -> Option<kestrel_semantic_tree::expr::LangIntrinsic> {
    use kestrel_semantic_tree::expr::LangPrimitive;

    // Try each primitive prefix: i1_, i8_, i16_, i32_, i64_, f32_, f64_
    let primitives = [
        ("i1_", LangPrimitive::I1),
        ("i8_", LangPrimitive::I8),
        ("i16_", LangPrimitive::I16),
        ("i32_", LangPrimitive::I32),
        ("i64_", LangPrimitive::I64),
        ("f32_", LangPrimitive::F32),
        ("f64_", LangPrimitive::F64),
    ];

    for (prefix, primitive) in primitives {
        if let Some(op_name) = name.strip_prefix(prefix) {
            if primitive.is_float() {
                return parse_float_op(primitive, op_name);
            } else {
                return parse_int_op(primitive, op_name);
            }
        }
    }
    None
}

/// Parse an integer operation like "add", "signed_div", "neg", etc.
fn parse_int_op(
    primitive: kestrel_semantic_tree::expr::LangPrimitive,
    op_name: &str,
) -> Option<kestrel_semantic_tree::expr::LangIntrinsic> {
    use kestrel_semantic_tree::expr::{IntBinaryOp, IntUnaryOp, LangIntrinsic, SignedOp};

    // Check for signed_* prefix
    if let Some(rest) = op_name.strip_prefix("signed_") {
        let op = match rest {
            "div" => SignedOp::Div,
            "rem" => SignedOp::Rem,
            "shr" => SignedOp::Shr,
            "lt" => SignedOp::Lt,
            "le" => SignedOp::Le,
            "gt" => SignedOp::Gt,
            "ge" => SignedOp::Ge,
            _ => return None,
        };
        return Some(LangIntrinsic::IntBinarySigned { primitive, op });
    }

    // Check for unsigned_* prefix
    if let Some(rest) = op_name.strip_prefix("unsigned_") {
        let op = match rest {
            "div" => SignedOp::Div,
            "rem" => SignedOp::Rem,
            "shr" => SignedOp::Shr,
            "lt" => SignedOp::Lt,
            "le" => SignedOp::Le,
            "gt" => SignedOp::Gt,
            "ge" => SignedOp::Ge,
            _ => return None,
        };
        return Some(LangIntrinsic::IntBinaryUnsigned { primitive, op });
    }

    // Signedness-agnostic binary ops
    match op_name {
        "add" => Some(LangIntrinsic::IntBinary {
            primitive,
            op: IntBinaryOp::Add,
        }),
        "sub" => Some(LangIntrinsic::IntBinary {
            primitive,
            op: IntBinaryOp::Sub,
        }),
        "mul" => Some(LangIntrinsic::IntBinary {
            primitive,
            op: IntBinaryOp::Mul,
        }),
        "eq" => Some(LangIntrinsic::IntBinary {
            primitive,
            op: IntBinaryOp::Eq,
        }),
        "ne" => Some(LangIntrinsic::IntBinary {
            primitive,
            op: IntBinaryOp::Ne,
        }),
        "and" => Some(LangIntrinsic::IntBinary {
            primitive,
            op: IntBinaryOp::And,
        }),
        "or" => Some(LangIntrinsic::IntBinary {
            primitive,
            op: IntBinaryOp::Or,
        }),
        "xor" => Some(LangIntrinsic::IntBinary {
            primitive,
            op: IntBinaryOp::Xor,
        }),
        "shl" => Some(LangIntrinsic::IntBinary {
            primitive,
            op: IntBinaryOp::Shl,
        }),
        // Unary ops
        "neg" => Some(LangIntrinsic::IntUnary {
            primitive,
            op: IntUnaryOp::Neg,
        }),
        "not" => Some(LangIntrinsic::IntUnary {
            primitive,
            op: IntUnaryOp::Not,
        }),
        // Bit manipulation ops
        "popcount" => Some(LangIntrinsic::IntUnary {
            primitive,
            op: IntUnaryOp::Popcount,
        }),
        "clz" => Some(LangIntrinsic::IntUnary {
            primitive,
            op: IntUnaryOp::Clz,
        }),
        "ctz" => Some(LangIntrinsic::IntUnary {
            primitive,
            op: IntUnaryOp::Ctz,
        }),
        "bswap" => Some(LangIntrinsic::IntUnary {
            primitive,
            op: IntUnaryOp::Bswap,
        }),
        _ => None,
    }
}

/// Parse a float operation like "add", "mul", "neg", etc.
fn parse_float_op(
    primitive: kestrel_semantic_tree::expr::LangPrimitive,
    op_name: &str,
) -> Option<kestrel_semantic_tree::expr::LangIntrinsic> {
    use kestrel_semantic_tree::expr::{
        FloatBinaryOp, FloatConstant, FloatMathOp, FloatPredicate, FloatUnaryOp, LangIntrinsic,
    };

    match op_name {
        // Basic binary operations
        "add" => Some(LangIntrinsic::FloatBinary {
            primitive,
            op: FloatBinaryOp::Add,
        }),
        "sub" => Some(LangIntrinsic::FloatBinary {
            primitive,
            op: FloatBinaryOp::Sub,
        }),
        "mul" => Some(LangIntrinsic::FloatBinary {
            primitive,
            op: FloatBinaryOp::Mul,
        }),
        "div" => Some(LangIntrinsic::FloatBinary {
            primitive,
            op: FloatBinaryOp::Div,
        }),
        "eq" => Some(LangIntrinsic::FloatBinary {
            primitive,
            op: FloatBinaryOp::Eq,
        }),
        "ne" => Some(LangIntrinsic::FloatBinary {
            primitive,
            op: FloatBinaryOp::Ne,
        }),
        "lt" => Some(LangIntrinsic::FloatBinary {
            primitive,
            op: FloatBinaryOp::Lt,
        }),
        "le" => Some(LangIntrinsic::FloatBinary {
            primitive,
            op: FloatBinaryOp::Le,
        }),
        "gt" => Some(LangIntrinsic::FloatBinary {
            primitive,
            op: FloatBinaryOp::Gt,
        }),
        "ge" => Some(LangIntrinsic::FloatBinary {
            primitive,
            op: FloatBinaryOp::Ge,
        }),
        // Unary operations
        "neg" => Some(LangIntrinsic::FloatUnary {
            primitive,
            op: FloatUnaryOp::Neg,
        }),
        // Constants (arity 0)
        "infinity" => Some(LangIntrinsic::FloatConst {
            primitive,
            constant: FloatConstant::Infinity,
        }),
        "nan" => Some(LangIntrinsic::FloatConst {
            primitive,
            constant: FloatConstant::Nan,
        }),
        // Predicates (arity 1, returns bool)
        "is_nan" => Some(LangIntrinsic::FloatPred {
            primitive,
            pred: FloatPredicate::IsNan,
        }),
        "is_infinite" => Some(LangIntrinsic::FloatPred {
            primitive,
            pred: FloatPredicate::IsInfinite,
        }),
        // Math unary operations (arity 1) - only Cranelift-supported ops
        "floor" => Some(LangIntrinsic::FloatMath {
            primitive,
            op: FloatMathOp::Floor,
        }),
        "ceil" => Some(LangIntrinsic::FloatMath {
            primitive,
            op: FloatMathOp::Ceil,
        }),
        "round" => Some(LangIntrinsic::FloatMath {
            primitive,
            op: FloatMathOp::Round,
        }),
        "trunc" => Some(LangIntrinsic::FloatMath {
            primitive,
            op: FloatMathOp::Trunc,
        }),
        "sqrt" => Some(LangIntrinsic::FloatMath {
            primitive,
            op: FloatMathOp::Sqrt,
        }),
        // Fused multiply-add (ternary)
        "fma" => Some(LangIntrinsic::FloatFma { primitive }),
        // Copy sign (binary)
        "copysign" => Some(LangIntrinsic::FloatCopysign { primitive }),
        _ => None,
    }
}
