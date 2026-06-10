# Stage 2 — Syntax

> **2a SHIPPED 2026-06-11** — zero new syntax, by design:
> `struct Foo: not Static` (nominal negation) and `where T: not Static`
> (param relaxation = need-not, the `T: not Copyable` convention) both
> ride the existing `NegativeBound`/`ConformanceItem::Negative` paths.
> `Static` is a plain `@builtin(.Static)` marker protocol in std.core.

**Remaining (2b/2c/2d): needs exploration** — ref-payload declaration
surface (likely none: `Optional[&T]` is just a type argument once E485
carves), closure capture cue (if any), `refs()`/for-in surface, the
escaping-annotation spelling for `F: Static` params. No lifetime
spelling, ever (ratified).
