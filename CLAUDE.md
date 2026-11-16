Project structure, workflows, quick references and patterns are in [`docs/contributing/`](docs/contributing/index.md).

## Agent rules

- **Don't edit a test to make it pass**, unless I tell you to or it uses invalid Kestrel syntax. Don't add `#[ignore]`.
- **Tests document the compiler's current state.** They don't all need to pass. If something doesn't work yet, add a test so it gets fixed.
- **When you hit a roadblock, stop and ask.** Don't revert or throw away changes. After 3 failed attempts at the same kind of fix, stop, list what you tried and what you ruled out, and ask for guidance.
- **You share the branch with other agents** working at the same time.
- **Say what you measured.** Report `pwd` and `git rev-parse HEAD` with every measurement, and build fresh (`cargo build --release --bin kestrel`) first, because a stale `target/` measures an old compiler. A worktree under `.claude/worktrees/` is on `main`, which is far behind the working branch, so a measurement there is about a different compiler. See [`verifying-claims.md`](docs/contributing/verifying-claims.md).
- **Don't relay another agent's measurement** you haven't reproduced. Re-run the ones that matter, or attribute them and mark them unverified.
- **When you get stuck and work it out,** save a memory about it so you don't get stuck the same way again.

## Debugging

- Trace through compiler internals with `ktrace!` from the `kestrel-debug` crate. Never use `eprintln!`, `println!` or ad-hoc flags. Add `ktrace!("<category>", …)` at the site you're investigating.
- Turn categories on with `KESTREL_DEBUG`: `KESTREL_DEBUG=infer,solver <cmd>`, or `KESTREL_DEBUG=all`.
- Categories are free-form strings picked at each call site. List the ones in use with `rg -oUN --no-filename -r '$1' 'ktrace!\(\s*"([^"]+)"' lib | sort -u`. Common ones: `infer`, `solver`, `hir-lower`, `entail`, `lit-default`, `existential`, `copy-prop`, `clone-shim`, `drop-fix`, `last-use-move`, `vtable`, `llvm-verify` (LLVM functions that fail verification), `lsp-verify` (persistent vs fresh LSP diagnostics).

## Testing

- **Run `kestrel-test-suite` only through the [`/triage`](.claude/skills/triage/SKILL.md) skill**, whether it's the full suite, a subset or one test. Never run `cargo test -p kestrel-test-suite` or a `file_tests-*` binary directly. Triage records results in `.triage/triage.db`, runs in the background, and is safe alongside other agents.
- The `triage` CLI is local tooling in `external/triage` (gitignored), so a fresh clone or cloud session may not have it. If `triage` isn't on your `PATH`, say so and ask before running tests any other way.
- **Read the results** when a run finishes: the pass/fail counts and the failure messages. A run whose output you didn't read is not a test run.
- Run the full suite before commits, not after every edit. While iterating, pass triage a pattern.

## Code quality

- Prefer early returns. Avoid deep nesting and long functions.
- Reuse existing code instead of writing new functions.
- Keep code terse, with comments that say what it does and why.
- Keep solutions clean and complete, and follow hECS patterns.
- Keep a single source of truth. Don't spread one fact over several places.
- Consider incremental compilation.
- Avoid fragile fixes. Prefer holistic ones that won't break later.
- When you fix a mistake, think about how to stop it being made again.
- **No name lookups for compiler-known things.** Never identify a lang item, stdlib declaration, lang type, enum case, protocol method or module by its spelling (`name == "Some"`, `"std"`, `"lang"`, `"Copyable"`, `"next"`), and never fall back to a name when a tag is missing. Tag it at the source (`@builtin(.X)` + `ResolveBuiltin`, a marker or `LangType` component set at seeding or load time, or a resolution recorded by type-infer) and read the tag. Compiler-generated code that must name a lang item uses `AstType::Builtin`, not a path. A missing tag is an error, not a reason to guess. User-written paths still resolve through normal scoping; this rule is only about the compiler recognizing its own items.

## Directory guidance (`AGENTS.md`)

Before editing a file, read any `AGENTS.md` in its directory or a parent directory up to the repo root. These record the conventions for that subtree: design rules, diagnostic-code ranges, analyzer structure, HIR invariants and so on. The nearest file wins, and this file is the fallback.

**Capture patterns as you find them.** When you notice a reusable rule ("we always do X here because Y", "never do Z in this subtree", "adding a W means updating these three places"), ask the user in one line whether to add it to the nearest `AGENTS.md`, or to a new one in the smallest subtree it covers. Don't add entries silently, and don't save them up for the end of the session.
