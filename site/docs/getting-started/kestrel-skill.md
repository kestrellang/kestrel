# Kestrel Skill

The Kestrel skill is a Claude Code agent that knows the language well — its syntax, idioms, common gotchas, and the standard library. Drop it into your project and it acts as a pair-programmer that reads your code and writes more of it.

## Installing

```sh
claude --add-skill kestrel
```

This registers the skill globally. From inside any Kestrel project, ask Claude to write code, refactor a function, or explain a diagnostic, and the skill will route the work through Kestrel-specific knowledge.

## What it's good at

- **Writing idiomatic code.** Knows the labels, access modes, and `where`-clause conventions that catch newcomers.
- **Explaining diagnostics.** Maps E-codes to plain-English explanations and likely fixes.
- **Scaffolding common shapes.** "Add a protocol with these requirements," "write the boilerplate for this enum's `Hashable` conformance."
- **Editing across files.** Knows the module layout and how `import` resolves.

## What to use it for

Day-to-day: same things you'd ask a senior teammate. "Why won't this compile?" "Refactor this to use protocols instead of overloading." "Add tests for this function."

When you're learning the language, the skill pairs well with this guide — read a chapter, then ask Claude to write a small example using what you just learned. The skill will produce something idiomatic that you can tear apart.

## What to skip

Don't use the skill for understanding the *compiler* (the Rust code in `lib/`). Different agent, different context. The Kestrel skill is about the language; the compiler internals have their own dedicated docs.

---

[← Flock](flock.md) · [↑ Getting Started](index.md) · [Install the LSP Extension →](lsp-extension.md)
