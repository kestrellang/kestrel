```mermaid
flowchart TD
  %% ========= PROPOSED: shift-to-query + shared helpers =========

  subgraph Binder
    SB2[SemanticBinder::bind] --> BS2[bind_symbol] --> REG2[DeclarationBinderRegistry::get]

    REG2 --> FB2[FunctionBinder]
    REG2 --> STB2[StructBinder]
    REG2 --> PB2[ProtocolBinder]
    REG2 --> TAB2[TypeAliasBinder]
    REG2 --> EXB2[ExtensionBinder]
    REG2 --> IB2[InitializerBinder]
  end

  subgraph Model_NewOrExpandedQueries
    Q_MIS[MethodsInSymbol (new query)<br/>replaces StructMethods/ExtensionMethods/FunctionsInSymbol]
    Q_PW[ProtocolWalker / ProtocolAncestors (new query/helper)]
    Q_PathRes[PathResolver (shared helper)<br/>used by ResolveTypePath + ResolveValuePath]
    Q_AOK[AncestorOfKind (existing)]
    Q_IsIn[IsInsideAny -> calls AncestorOfKind or walk_ancestors helper]

    Q_APEX[ApplicableExtensionsFor{ty} (new query)]
    Q_RPBP[ResolveProtocolBoundPath{segments,context,span} (new query)]
    Q_RAT[ResolveAssociatedTypeFromTypeParam{type_param, segment, context} (new query)]
  end

  subgraph Model_ExistingQueries
    Q_RTP2[ResolveTypePath] --> Q_PathRes
    Q_RVP2[ResolveValuePath] --> Q_PathRes
    Q_RN2[ResolveName]
  end

  subgraph Binder_NewSharedFunctions
    F_Gen[binders/utils/generics.rs<br/>resolve_generics/where/type_bound (single impl)]
    F_Params[binders/utils/parameters.rs<br/>resolve_parameters(..., implicit_labels)]
    F_Body[binders/utils/body.rs<br/>setup_and_resolve_body + get_self_type]
    F_Scope[resolution/local_scope.rs<br/>LocalScope::new_from_dyn? or helper]
  end

  %% binder uses shared functions
  FB2 --> F_Gen
  STB2 --> F_Gen
  PB2 --> F_Gen
  TAB2 --> F_Gen
  EXB2 --> F_Gen

  FB2 --> F_Params
  IB2 --> F_Params

  FB2 --> F_Body --> F_Scope
  IB2 --> F_Body
  CTX2[body_resolver/context.rs] --> F_Body

  %% key “move to query” edges
  F_Gen --> Q_RPBP
  F_Gen --> Q_RAT
  F_Gen --> Q_RTP2

  %% protocol traversal consolidation
  Q_PRM2[ProtocolRequiredMethods] --> Q_PW
  Q_PMWD2[ProtocolMethodsWithDefiner] --> Q_PW
  Q_IPM2[InheritedProtocolMember] --> Q_PW

  %% extension applicability consolidation
  Q_RVP2 --> Q_APEX
  BRM2[body_resolver/members.rs] --> Q_APEX

  %% methods consolidation
  Q_SM2[StructMethods] -.-> Q_MIS
  Q_EM2[ExtensionMethods] -.-> Q_MIS
  Q_FIS2[FunctionsInSymbol] -.-> Q_MIS
```

