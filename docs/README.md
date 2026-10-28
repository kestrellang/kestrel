# Kestrel Documentation

## Living Documentation

Kept current with the compiler. Start here.

### For Language Users

- **[Language Guide](language/)** — syntax, features, and how to write Kestrel code
  - Recent feature pages: [References (`&T`)](language/references.md), [Error Handling](language/error-handling.md), [Entry Points (`@main`)](language/entry-points.md), [Opaque Types (`some P`)](language/opaque-types.md), [String Interpolation](language/string-interpolation.md)
- **[Memory Model](memory-model/)** — value semantics, copy/move, access modes, drops, ABI
- **[Standard Library Reference](stdlib/)** — generated API docs for the stdlib
- **[Tooling](tooling.md)** — the `kestrel` CLI, backends, and optimization levels
- **[Error Codes](error-codes.md)** — diagnostic code reference
- **[Flock](../lang/flock/README.md)** — package manager (build, run, install, publish)
- **[Jessup](../lang/jessup/README.md)** — toolchain version manager

### For Compiler Developers

- **[Contributing](contributing/)** — architecture, workflows, patterns, quick reference
- **[Naming Conventions](NAMING_CONVENTIONS.md)** — naming rules across the codebase

## Historical & Design Material

Point-in-time documents — design explorations, implementation plans, and
snapshots that informed the compiler as it was built. They are **not**
current-behavior references and may contradict what the compiler does today.

- **[Plans](plans/)** — per-feature implementation plans (see the [index](plans/index.md) for shipped/unshipped status)
- **[Design](design/)** — API design sketches (e.g. datetime)
- **[Refactor](refactor/)** — MIR rewrite report
- **[References Prototype](references-prototype/)** — pre-implementation exploration of `&T` (the shipped design lives in [language/references.md](language/references.md))
- **[0.16 Scope](scope_0.16.md)** — scoping notes for the 0.16 cycle (0.16 has shipped)
- **[Blog Drafts](BLOG.md)**
- **[Future Ideas](FUTURE_IDEAS.md)** / **[Future Architecture Ideas](FUTURE_ARCH_IDEAS.md)** / **[Future Effects](future_effects.md)** — speculative direction notes
- **[Bug Hunt](bughunt.md)** — bug-hunt session notes
- **[New Collections](new-collections.md)** — collection-design exploration
