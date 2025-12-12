```mermaid
flowchart TD
  %% ========= CURRENT: what calls what =========

  subgraph Compiler
    COMP[Compilation::from_sources<br/>lib/kestrel-compiler/src/compilation.rs] --> SB[SemanticBinder::bind<br/>lib/kestrel-semantic-tree-binder/src/resolution/binder.rs]
  end

  subgraph BinderOrchestrator
    SB --> BS[bind_symbol (walk tree)]
    BS --> REG[DeclarationBinderRegistry::get]
  end

  subgraph Binders
    REG --> FB[FunctionBinder::bind_declaration<br/>binders/function.rs]
    REG --> IB[InitializerBinder::bind_declaration<br/>binders/initializer.rs]
    REG --> STB[StructBinder::bind_declaration<br/>binders/struct.rs]
    REG --> PB[ProtocolBinder::bind_declaration<br/>binders/protocol.rs]
    REG --> TAB[TypeAliasBinder::bind_declaration<br/>binders/type_alias.rs]
    REG --> EXB[ExtensionBinder::bind_declaration<br/>binders/extension.rs]
  end

  subgraph BinderHelpers_Duplicated
    FB --> RG_F[resolve_generics] --> RWC_F[resolve_where_clause] --> RTB_F[resolve_type_bound]
    STB --> RG_S[resolve_generics] --> RWC_S[resolve_where_clause] --> RTB_S[resolve_type_bound]
    PB --> RG_P[resolve_generics] --> RWC_P[resolve_where_clause] --> RTB_P[resolve_type_bound]
    TAB --> RG_TA[resolve_generics] --> RWC_TA[resolve_where_clause] --> RTB_TA[resolve_type_bound]
    EXB --> RWC_EX[resolve_extension_where_clause] --> RTB_EX[resolve_extension_type_bound]

    FB --> RFB[resolve_function_body] --> LSN[LocalScope::new(Arc<FunctionSymbol>)]
    IB --> RIB[resolve_initializer_body] --> LSN
    CTX[body_resolver/context.rs<br/>resolve_and_attach_body] --> LSN

    FB --> RPF[resolve_parameters_from_syntax] --> RSPF[resolve_single_parameter]
    IB --> RPI[resolve_parameters_from_syntax] --> RSPI[resolve_single_parameter]
    FB --> GSTF[get_self_type]
    IB --> GSTI[get_self_type]
  end

  subgraph SemanticModelQueries
    RTB_F --> Q_RTP[model.query(ResolveTypePath)]
    RTB_S --> Q_RTP
    RTB_P --> Q_RTP
    RTB_TA --> Q_RTP
    RTB_EX --> Q_RTP

    BRP[body_resolver/paths.rs] --> Q_RVP[model.query(ResolveValuePath)]

    Q_RTP --> Q_RN[ResolveName] --> Q_SF[ScopeFor]
    Q_RTP --> Q_VCBN[VisibleChildrenByName]
    Q_RTP --> Q_IPM[InheritedProtocolMember]
    Q_RTP --> Q_SymFor[SymbolFor]

    Q_RVP --> Q_RN
    Q_RVP --> Q_VCBN
    Q_RVP --> Q_Ext[ExtensionsFor]
    Q_RVP --> Q_Vis[IsVisibleFrom]

    Q_RN --> Q_IPM
    Q_RN --> Q_SymFor

    Q_PRM[ProtocolRequiredMethods] --> Q_SymFor
    Q_PMWD[ProtocolMethodsWithDefiner] --> Q_CFS[ConformancesForSymbol]
    Q_IPM --> Q_IPM
  end

  subgraph Analyzers
    AN_C[conformance analyzer] --> Q_PRM
    AN_C --> Q_PMWD
    AN_PM[protocol_method analyzer] --> Q_FIS[FunctionsInSymbol]
    AN_EC[extension_conflict analyzer] --> Q_EM[ExtensionMethods]
    AN_EC --> Q_SM[StructMethods]
  end
```

