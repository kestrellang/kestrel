# Let/Var, Static, and Computed Properties by Context

This document summarizes how `let`, `var`, `static`, and computed properties behave in the current Kestrel language/compiler implementation. Each context has its own 2D table:

- Columns: `static`, `non-static`
- Rows: `computed`, `non-computed`

Each cell summarizes `let`/`var` behavior.

Notes:
- “Computed” means a field with a computed body (`{ ... }`, `{ get ... }`, `{ get set ... }`) or a protocol requirement (`{ get }`, `{ get set }`).
- “Non-computed” means a stored field (no computed body).
- When a behavior is not currently supported beyond parsing, it is called out explicitly.

## Global (Module Scope)

Module-level field declarations parse, but they are not currently resolvable as values in expressions (no `ValueBehavior` attached). In practice, treat global fields as **not usable yet**, regardless of `let/var/static/computed`.

|                         | static | non-static |
|-------------------------|--------|------------|
| **computed**            | Parsed, but not resolvable as a value; `let/var` effectively unusable. | Parsed, but not resolvable as a value; `let/var` effectively unusable. |
| **non-computed**        | Parsed, but not resolvable as a value; `let/var` effectively unusable. | Parsed, but not resolvable as a value; `let/var` effectively unusable. |

## Struct

|                         | static | non-static |
|-------------------------|--------|------------|
| **computed**            | Only `var` is intended (spec); parser currently allows `let`. Access lowers to getter/setter calls; `let` with computed body is accepted syntactically but not enforced. | Only `var` is intended (spec); parser currently allows `let`. Access lowers to getter/setter calls; assignment uses setter. |
| **non-computed**        | Parsed as static stored field. Access resolves via type reference; access is treated as non-mutable, so `var` does not enable assignment currently. | Stored instance field. `let` = immutable after init; `var` = mutable if the root binding is `var`. Included in memberwise init; must be initialized in init (unless computed). |

## Enum

|                         | static | non-static |
|-------------------------|--------|------------|
| **computed**            | Only `var` is intended (spec); parser currently allows `let`. Access lowers to getter/setter calls (static getter/setter have no receiver). | Only `var` is intended (spec); parser currently allows `let`. Access lowers to getter/setter calls. |
| **non-computed**        | Parsed, but enum lowering only emits cases; enum stored fields are not represented in layout/codegen. Treat as unsupported beyond parsing/binding. | Parsed, but enum lowering only emits cases; enum stored fields are not represented in layout/codegen. Treat as unsupported beyond parsing/binding. |

## Protocol

|                         | static | non-static |
|-------------------------|--------|------------|
| **computed**            | Protocol property requirements must be computed (`{ get }` or `{ get set }`). `var` is intended; `let` is not supported by spec (parser currently accepts). Requirements are derived only when there is no body. | Same as static: computed-only requirements. `var` intended, `let` not supported by spec; getter-only or getter+setter requirement forms. |
| **non-computed**        | Stored fields are not protocol requirements; ignored for conformance. `let/var` have no requirement effect. | Stored fields are not protocol requirements; ignored for conformance. `let/var` have no requirement effect. |

