#!/usr/bin/env bash
# Bulk-create issues from ROADMAP.md unchecked items, one issue per
# checkbox, assigned to the right milestone with a type label and the
# `triage` label.
#
# Idempotent: looks up existing issues by title before creating.
#
# Usage: bash scripts/setup/migrate-roadmap.sh [--dry-run]
#
# After this runs, the issues sit in the Backlog with the `triage` label
# until the next triage day, when Area + Size are assigned.

set -euo pipefail

OWNER="${OWNER:-kestrellang}"
REPO="${REPO:-kestrel}"

DRY_RUN=0
if [ "${1:-}" = "--dry-run" ]; then
  DRY_RUN=1
fi

# Build a set of existing issue titles (open + closed) once, for the
# idempotency check. Open issues only — we don't want to recreate things
# that were closed manually. If you want to ignore closed too, change
# --state=open below.
echo "==> Loading existing open issue titles"
EXISTING_TITLES=$(gh issue list --repo "$OWNER/$REPO" --state open --limit 1000 --json title --jq '.[].title')

create_issue() {
  local milestone="$1"
  local type_label="$2"
  local title="$3"
  local body="$4"

  if echo "$EXISTING_TITLES" | grep -qxF "$title"; then
    echo "  skip (exists): $title"
    return
  fi

  if [ "$DRY_RUN" = "1" ]; then
    printf "  [dry-run] %-12s %-10s %s\n" "$milestone" "$type_label" "$title"
    return
  fi

  gh issue create \
    --repo "$OWNER/$REPO" \
    --title "$title" \
    --body "$body" \
    --label "$type_label,triage" \
    --milestone "$milestone" \
    >/dev/null
  echo "  created: [$milestone] $title"
}

# Format: each row is `milestone|type|title|body`. Body is short — the
# real design lives on whatever PR/branch implements it.

ROWS=$(cat <<'EOF'
0.16|feature|Opaque types (`some Protocol`)|Add `some Protocol` opaque return types. See ROADMAP §0.16.
0.16|feature|Allow computed properties in protocol extensions|Lift the current restriction. See ROADMAP §0.16.
0.16|feature|Name collisions between methods and computed properties|Define rules for resolving the conflict. See ROADMAP §0.16.
0.16|feature|Keywords usable as labels|Allow keywords like `if`, `for` as parameter labels. See ROADMAP §0.16.
0.16|feature|`some` patterns|`if let some(x) = optional` style binding. See ROADMAP §0.16.
0.16|feature|Null patterns|Pattern-match against null. See ROADMAP §0.16.
0.16|feature|Chained guards|Guards that chain multiple conditions. See ROADMAP §0.16.
0.16|feature|Normal guard|Standalone `guard` statement. See ROADMAP §0.16.
0.16|feature|`Self` constructors|Allow `Self()` in protocol/extension contexts. See ROADMAP §0.16.
0.16|feature|Half-open ranges (`..n`, `n..`)|Prefix and suffix range syntax. See ROADMAP §0.16.
0.16|feature|Optional and throwing constructors|`init?` and `init throws` variants. See ROADMAP §0.16.

0.17|feature|Existential types (`any Protocol`)|Boxed via GlobalAllocator, vtable with drop/size/align + protocol methods. `any P` is non-Copyable; `Cloneable` requires `P: Cloneable`. See ROADMAP §0.17.
0.17|feature|Escaping closures|Box captures when a closure outlives its frame. See ROADMAP §0.17.
0.17|feature|Indirect enum heap-boxing|`indirect case` variant payloads heap-boxed via GlobalAllocator. See ROADMAP §0.17.

0.18|feature|Attribute system infrastructure|Parsed and propagated through AST/HIR/MIR. See ROADMAP §0.18.
0.18|feature|Auto-derived protocols|`@derive(Equatable, Hashable, Cloneable, Comparable)`. Depends on attribute infra. See ROADMAP §0.18.
0.18|feature|Built-in attributes (`@inline`, `@deprecated`)|See ROADMAP §0.18.

0.19|feature|Optional chaining|`a?.b?.c` syntax. See ROADMAP §0.19.
0.19|feature|Pipe operator (`\|>`)|Function pipelining. See ROADMAP §0.19.
0.19|feature|Placeholder arguments (`_`)|For partial application. See ROADMAP §0.19.

0.20|feature|Lazy properties|`lazy let expensive = compute()`. See ROADMAP §0.20.
0.20|feature|Property observers (`willSet` / `didSet`)|See ROADMAP §0.20.
0.20|feature|`mutating get` on computed properties and subscripts|Unblocks insert-on-read APIs like `Dictionary.subscript(key:inserting:)`. See ROADMAP §0.20.
0.20|feature|Conditional conformance|`Box[T]: Copyable where T: Copyable`. See ROADMAP §0.20.

0.21|chore|Stabilize stdlib by 0.21|Standard library expansion and depth pass. See ROADMAP §0.21.
0.21|chore|Improve compiler speed|Profiling pass and targeted improvements. See ROADMAP §0.21.
0.21|chore|Speed up stdlib|Targeted hot-path improvements. See ROADMAP §0.21.
0.21|feature|Language refinements informed by 0.16–0.20 usage|Catch-all for small adjustments. See ROADMAP §0.21.

0.22|feature|Class declarations with reference semantics|See ROADMAP §0.22.
0.22|feature|Reference counting with control blocks|Runtime support for class instances. See ROADMAP §0.22.
0.22|feature|Identity (`===` reference equality)|See ROADMAP §0.22.
0.22|feature|RTTI via extended vtables|See ROADMAP §0.22.
0.22|feature|`@weak` / `@unowned` reference attributes|See ROADMAP §0.22.
0.22|feature|`@final` classes|Disallow further subclassing. See ROADMAP §0.22.

0.23|chore|Bug fixes from 0.16–0.22 usage|Catch-all for issues surfaced after each cycle. See ROADMAP §0.23.
0.23|chore|Class runtime hardening|Informed by real-world adoption. See ROADMAP §0.23.
0.23|chore|Documentation and stdlib polish|See ROADMAP §0.23.
0.23|chore|Stabilization of Preview 2 surface area|API freeze pass. See ROADMAP §0.23.

Preview 3|feature|`generator` / `yield` syntax|Generator function declarations. See ROADMAP Preview 3.
Preview 3|feature|CPS / state-machine lowering|Generator and async share infrastructure. See ROADMAP Preview 3.
Preview 3|feature|Lazy sequences via generator functions|Stdlib API on top of generators. See ROADMAP Preview 3.
Preview 3|feature|`async` / `await` syntax|Built on generator state machines. See ROADMAP Preview 3.
Preview 3|feature|Async executor and runtime|See ROADMAP Preview 3.
Preview 3|feature|`Future` type|Stdlib type for async results. See ROADMAP Preview 3.
Preview 3|feature|Async standard library APIs|Async I/O, channels, etc. See ROADMAP Preview 3.
Preview 3|feature|Atomic operations and ordering semantics|See ROADMAP Preview 3.
Preview 3|feature|Memory model for concurrent access|See ROADMAP Preview 3.
Preview 3|feature|`send` / `sync` capabilities for thread safety|Type-level thread-safety. See ROADMAP Preview 3.
Preview 3|feature|Task groups and cancellation|Structured concurrency. See ROADMAP Preview 3.
Preview 3|feature|Async generators and async sequences|See ROADMAP Preview 3.
Preview 3|feature|Actors or concurrency model refinement|Decision point informed by usage. See ROADMAP Preview 3.
Preview 3|feature|Multithreading primitives|Thread spawning, joining. See ROADMAP Preview 3.
Preview 3|chore|Concurrency testing and debugging tools|Race detector, deterministic scheduler, etc. See ROADMAP Preview 3.

Preview 4|feature|`using` / `given` implicit parameters|Implicit parameter passing. See ROADMAP Preview 4.
Preview 4|refactor|Migrate `GlobalAllocator` to `given Allocator`|Implicits subsume the global allocator. See ROADMAP Preview 4.
Preview 4|refactor|Migrate async context to `using`|Executors and cancellation tokens via implicits. See ROADMAP Preview 4.
Preview 4|feature|Effect-lite generalization|Implicit propagation, handler blocks, effect inference. Stepping-stone to 3.0. See ROADMAP Preview 4.
Preview 4|feature|Language features informed by real-world usage|Catch-all. See ROADMAP Preview 4.
Preview 4|chore|Standard library shape and depth|See ROADMAP Preview 4.
Preview 4|chore|Ecosystem tooling and refinements|See ROADMAP Preview 4.

RC|feature|LLVM backend|Production-quality codegen. See ROADMAP RC.
RC|feature|WebAssembly target|See ROADMAP RC.
RC|feature|`const` compile-time evaluation|See ROADMAP RC.
RC|feature|`unsafe` blocks and escape hatches|See ROADMAP RC.
RC|chore|Standard library stabilization|API freeze for 1.0. See ROADMAP RC.

2.0|feature|User-defined procedural macros|Extends `@derive` from 0.18. See ROADMAP 2.0.
2.0|feature|Compile-time reflection|Inspect types, fields, conformances. See ROADMAP 2.0.
2.0|feature|`comptime` blocks|Compile-time evaluation. See ROADMAP 2.0.
2.0|feature|Custom attributes that generate code|See ROADMAP 2.0.

3.0|feature|User-defined `effect` declarations|See ROADMAP 3.0.
3.0|feature|`handle` blocks (effect handlers)|See ROADMAP 3.0.
3.0|feature|Effect polymorphism|`func map(f: (A) -> B / E) -> Array[B] / E`. See ROADMAP 3.0.
3.0|refactor|Reframe async / generators / throw as built-in effects|Existing machinery becomes effects. See ROADMAP 3.0.
3.0|feature|Built-in effects (`async`, `throws`, `yield`, `alloc`, `unsafe`, `const`)|See ROADMAP 3.0.
3.0|chore|Unified control flow under one composable model|Documentation and stdlib pass. See ROADMAP 3.0.
EOF
)

echo "==> Migrating ROADMAP items to issues"
echo "$ROWS" | while IFS='|' read -r milestone type_label title body; do
  [ -z "${milestone:-}" ] && continue
  case "$milestone" in
    \#*) continue ;;  # comment line
  esac
  create_issue "$milestone" "$type_label" "$title" "$body"
done

echo "==> Done."
