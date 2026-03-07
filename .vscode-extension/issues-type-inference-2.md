 1. For-loop ProtocolCall (desugar.rs): Changed for-loop desugaring from plain MethodCall to ProtocolCall with
  Builtin::IterableProtocol/Builtin::IteratorProtocol
  2. TyKind::Param member resolution (resolve.rs): Added resolve_param_member that searches protocol bounds from parent's WhereClause when receiver is a type
  parameter
  3. Init-specific search (resolve.rs): Added search for NodeKind::Initializer entities (which have no Name component) when looking for "init" members
  4. Parameter label extraction fix (params.rs): Handle Name node wrapping of labels (Name > Identifier) in addition to bare Identifier tokens — eliminated all
  81 AmbiguousMember errors
  5. Init overload disambiguation (generate.rs): When multiple inits match by labels (e.g., Int64.init(from: Int8/UInt8/...)), skip arg constraints to avoid
  picking the wrong overload
  6. Type alias resolution (ty.rs): Resolve type aliases like type Fd = Int32 during AST→HIR lowering so they produce the same HirTy entity
  7. Tuple index resolution (solver.rs): Handle TyKind::Tuple in solve_member for numeric index names ("0", "1")
  8. Associated type scaffold (solver.rs): Added detection of TypeAlias entities in lower_hir_ty_sub with Associated constraint emission for concrete non-self
  receivers (currently zero-effect but infrastructure is in place)

   1. Call constraint (constraint.rs, ctx.rs, generate.rs, solver.rs, resolve.rs) — 43 errors fixed                                                                                          
    - New constraint dispatches based on callee type: Function → param/return unification, Named → subscript resolution
    - Subscript candidate search in resolve_member via "(subscript)" sentinel
  2. HirTy::Never (hir/ty.rs, hir-lower/ty.rs, generate.rs, solver.rs) — 4 errors fixed
    - New variant so AstType::Never lowers properly through the pipeline
    - lower_hir_ty_with_subs and lower_hir_ty_sub both handle it → ctx.never()
  3. lang.panic → Never (lang_module.rs) — 4 errors fixed
    - Was returning (), now returns Never (diverging function)
    - Fixes Optional.unwrap, Optional.expect, Result.unwrap, etc.
  4. Block divergence detection (generate.rs) — 4 errors fixed
    - { return .None } blocks now typed as Never instead of ()
    - Checked via block_diverges() — looks at last statement


 Fix 1: Associated Type Resolution on Abstract Types (-8 NoAssociatedType → 0)                                                                                                             
                                                                                                                                                                                          
  Problem: resolve_associated_type only handled concrete TyKind::Named entities. When the container was a TypeAlias (e.g., Iter from Iterable protocol) or TypeParameter, it returned None
  immediately. So Iter.Item couldn't be resolved.

  Fix in resolve.rs: Extended resolve_associated_type to handle:
  - TyKind::Named { entity: TypeAlias } → search protocol bounds for the associated type
  - TyKind::Named { entity: TypeParameter } → search param protocol bounds
  - TyKind::Param → same as TypeParameter
  - Abstract types without a TypeAnnotation return the entity as HirTy::Named

  Fix 2: Closure vs Block Expression Ambiguity (-8 ImplicitMemberNotFound → 0, -11 TypeMismatch)

  Problem: In match arm bodies, { stmts; expr } was parsed as a parameterless closure instead of a block expression. This gave .None a Function type instead of propagating from the match
  result. Also, statement-like expressions (if/match) at end of function bodies weren't recognized as tail expressions.

  Fixes across 4 files:
  1. lower.rs (AST builder): lower_block promotes the last ExpressionStatement without semicolon to tail_expr
  2. lower.rs (AST builder): lower_match_arm detects parameterless closures and uses lower_closure_as_block
  3. ast_body.rs: Added AstExpr::Block variant; body.rs: Added HirExpr::Block
  4. generate.rs: Handles HirExpr::Block via gen_block

  1. AST builder fix (helpers.rs): SyntaxKind::AssociatedTypeTarget was missing from the filter in set_where_clause's TypeEquality handler, causing Item.Output = Item constraints to be
  silently dropped
  2. Extension where clause emission (lib.rs): Added emit_extension_where_clauses + get_or_create_subject_tv to emit associated, conforms, and equal constraints from extension where
  clauses during method body inference
  3. Where clause assoc type substitutions (ctx.rs, solver.rs): Added where_clause_assoc_subs field to InferCtx. When lower_hir_ty_sub encounters a TypeAlias entity (like Output), it
  checks this map first — critical because each call creates new TyVars that otherwise wouldn't connect to the equality constraint
  4. Protocol type param substitution (solver.rs): In solve_member, map owning protocol's type params (e.g., Rhs in Addable[Rhs = Self]) to the receiver TyVar
  5. Static member resolution for TypeAlias (resolve.rs): Fallback to static member search when instance member search fails for TypeAlias receivers
  6. Named entity substitution (generate.rs): lower_hir_ty_with_subs now also substitutes Named entities (not just Param), needed for where clause RHS lowering
  7. Stdlib (iterator.ks): Added Item.Output = Item to sum/product extension where clauses
