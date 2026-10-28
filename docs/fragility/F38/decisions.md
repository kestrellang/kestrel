# F38 — decisions

`ServerState.disk_line_indices: HashMap<path, LineIndex>` was a full second copy
of every disk-loaded file's text. `LineIndex` owns a `String`, and
`load_workspace` pushed the entire stdlib through it — 84 files, 1.44 MB, ~41k
lines — plus every workspace source.

All four of its writers were path-derived (`lib.rs`'s workspace load, the
manifest-change reload's `clear()`, and the watched-file reload's insert/remove).
`did_open` / `did_change` / `did_close` touched only `docs` and `sources` and
never went near it. That asymmetry is the bug.

## What it actually broke (reproduced live over stdio, before any edit)

One file, error on 0-based line 2:

| step | published range |
|---|---|
| `initialized` | line 2, char 4-24 |
| `didOpen` | line 2 |
| `didChange` prepending 5 blank lines | line 7 |
| `didClose` (unsaved) | **line 2 char 9 → line 4 char 0** |

`did_close` deliberately keeps the edited buffer in `sources` so other files can
still resolve it, and only drops the `OpenDoc`. The next `refresh` therefore
compiled the *edited* text and rendered its spans through the *disk* index —
every squiggle in the file slid by the size of the edit.

Second failure: a file opened via `didOpen` that the workspace walk never
visited had no disk entry at all. Once its `OpenDoc` went away, `FileMap::lookup`
returned `None`, `from_codespan` bailed at its `?`, and the diagnostic was
**dropped entirely** with nothing logged.

Neither ever panicked. F24's clamp in `position.rs` turns an out-of-range offset
into a plausible-looking position, which is exactly why this was invisible for as
long as it was — the failure had no crash to point at it.

## 1. Delete the field; do not sync it

**Decision:** remove `disk_line_indices` outright rather than adding writes to
it from `did_open` / `did_change` / `did_close`.

A hand-synced second copy of text that `sources` already owns *is* the bug class.
Adding three more writers makes the current symptom go away and leaves the
invariant "every path that mutates text must also mutate the index" for the next
handler to break. `sources` is already documented as the single source of truth
for what we feed the compiler, and it holds correct text for all three cases that
mattered: open files, closed-but-edited buffers, and orphan files the workspace
walk never saw. Deriving the index from it makes the correct thing the only
thing.

The audit's premise of "7-ish read sites" did not survive contact. There was
**exactly one** genuine read — `diagnostics.rs`'s
`doc_indices.get(path).or_else(|| disk_indices.get(path))`. The `s.disk_line_indices.clone()`
one line up was that read's plumbing, not a second consumer.

Every other handler already builds indices on demand — `definition.rs`,
`references.rs`, `workspace_symbols.rs`, `call_hierarchy.rs`, `type_hierarchy.rs`,
`rename.rs`, `code_actions.rs`, `document_highlight.rs` all do
`LineIndex::new(sources.get(&path)?.clone())`. `refresh` was the lone exception.
The fix moves it onto the in-crate precedent instead of inventing a policy.

## 2. Per-refresh construction is *cheaper* than the clone it replaces

This is not a correctness-for-performance trade; the new code does strictly less
work on the hot path.

The old `refresh` ran `s.disk_line_indices.clone()` on **every debounced
keystroke** — a deep clone of ≥1.4 MB of `String` plus ~41k-entry `line_starts`
vectors, under the state lock, for a result that was then consulted for at most a
handful of ids. It also cloned every open doc's `LineIndex` into `doc_indices`.

The new code builds a `HashMap<String, &LineIndex>` borrowing from the open docs
(no text copied), and constructs a fresh `LineIndex` only for the files a
diagnostic actually points at. In the steady state that is 0-5 files. The
server-lifetime memory cost of the map — one extra copy of the whole workspace,
permanently resident — goes away with it.

## 3. Scope the indices by *diagnostic-referenced* file ids

**Decision:** add `referenced_file_ids(codespan, analyze) -> HashSet<usize>`,
walking `diag.labels[].file_id` and `AnalyzeDiagnostic.labels[].span.file_id`, and
build indices only for that set.

This is sound because `FileMap::lookup` is only ever reached from a label's file
id. Verified against `convert.rs` rather than assumed — all four call sites are
label-derived:

* `from_codespan` — `files.lookup(primary.file_id)` and `files.lookup(l.file_id)`
  for the `relatedInformation` labels;
* `from_analyze` → `label_range(primary)` — `files.lookup(label.span.file_id)`;
* `from_analyze` → `span_to_location(&l.span)` — `files.lookup(span.file_id)`.

`relatedInformation` is covered for free precisely because the walk visits *every*
label, not just primaries — a cross-file "declared here" note in a closed stdlib
file still resolves.

The set is computed **inside** the compiler-worker closure, so `id_to_path` is
also narrowed there: it now clones a `String` per referenced file instead of one
per compiled file.

## 4. Resolution order: live buffer, then `sources`

Per id: the open `OpenDoc`'s `LineIndex` if the file is open (that is the text
the editor is showing), otherwise `LineIndex::new(sources.get(path)?.clone())`.

The fallback is the whole fix for the misplacement bug — a closed-but-unsaved
buffer resolves against the edited text still sitting in `sources`, not against
disk.

Open docs are keyed by path via `url_to_path`, matching the old behaviour, rather
than by round-tripping `path_to_url`. `url_to_path` canonicalizes; a client that
opened a symlinked URL would not round-trip back to the same `Url`.

## 5. `FileMap` stays a borrow

`convert.rs` is untouched. `FileMap<'a> { by_id: HashMap<usize, (Url, &'a LineIndex)> }`
is fine as-is; `refresh` builds an owning `HashMap<usize, (Url, LineIndex)>` local
first and borrows into the `FileMap` afterwards. The local outlives `files` on
`refresh`'s own stack, so no lifetime surgery — and no allocation churn — was
needed in the shared conversion layer.

## 6. The silent drop closes structurally, and is made loud anyway

A file can only appear as a diagnostic's `file_id` if it was compiled, which means
it was in `sources`, which means the fallback resolves it. The orphan-file failure
is therefore closed by construction, not by a special case.

A backstop is still worth having, because the *original* failure was invisible
specifically for lack of one. `refresh` collects ids that failed to resolve and
emits a single `client.log_message(MessageType::WARNING, …)` naming them.

**The loud path belongs in `refresh`, not in `convert.rs`.** The `?` chains in
`from_codespan` / `from_analyze` have no idea whether a missing lookup is a bug or
an ordinary "this label points somewhere we don't render"; `refresh` knows exactly
what it needed and what it resolved. Adding logging inside `convert.rs` would mean
threading a client (or an error channel) through a pure conversion module for no
gain.

## Regression coverage

`lib/kestrel-lsp/tests/integration.rs`, both driving the real binary over stdio:

* `diagnostics_stay_put_after_closing_an_edited_buffer` — the misplacement.
  Tempdir with `flock.toml` + `src/main.ks`; initialize with a workspace folder,
  then disk-load → line 2, `didOpen` → line 2, `didChange` prepending 5 blank
  lines → line 7, `didClose` → **still line 7**.
* `diagnostics_survive_closing_a_file_the_workspace_never_saw` — the silent drop.
  `rootUri: null` so `load_workspace` never runs; `didOpen` an orphan file with
  an error, `didClose`, assert the diagnostic is still published and still on the
  same line.

Both were confirmed non-vacuous by restoring the pre-fix sources and re-running:
the first fails `left: [2], right: [7]`, the second fails with the last publish
being an empty array.

The first is the only workspace-rooted test in the file and is correspondingly
slower — `initialize` walks the tree and the first refresh compiles the in-repo
stdlib (~10s vs ~5s for its neighbours). It is *not* exposed to the recorded
worktree duplicate-symbol hazard: that needs two overlapping `flock.toml` trees
visible in one session, and this is a fresh private tempdir with exactly one
manifest.

The harness gained `close(uri)` and `initialize_with_workspace(root_uri)`. The
latter deliberately omits `initializationOptions` so the server falls through to
`default_std_path()`, which for a repo-built binary resolves to the in-repo
`lang/std`.

## Follow-up found in passing — NOT fixed here

`definition.rs:44` does `s.sources.clone()` — a deep clone of the **entire**
source map, stdlib included — into the compiler-worker closure, purely so it can
later run `LineIndex::new(sources.get(&path)?.clone())` for one or two files. The
same pattern appears at:

* `handlers/type_hierarchy.rs` (`prepare`, `supertypes`/`subtypes`)
* `handlers/rename.rs`
* `handlers/call_hierarchy.rs` (`prepare`, `incoming`, `outgoing`)

Same cost class as the `disk_line_indices.clone()` this change removed — ~1.4 MB
of `String` copying for a handful of lookups — but on the **request** path
(goto-definition, rename, call hierarchy) rather than the debounce path, so it is
paid per keystroke-triggered request instead of per debounced edit.

The shape of the fix is the same: pass only the paths that are needed, or hand the
closure a cheap handle instead of an owned map. Out of F38's scope; recorded here
so it is not rediscovered from scratch.
