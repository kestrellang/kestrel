# archmap

Generates an interactive map of the compiler from the source tree.

```sh
python3 scripts/archmap/build.py . -o archmap.html            # map the current checkout
git worktree add ../kn nightly && python3 scripts/archmap/build.py ../kn -o archmap.html   # map nightly
```

- `extract.py` scans the workspace (via `cargo metadata` and regex + brace matching
  over the Rust sources) and prints JSON: crates and their deps, modules, `QueryFn`
  impls and which queries call which, component reads/writes, analyzers and their
  diagnostic codes, the MIR `Stage` pipeline, and `ktrace!` categories.
- `template.html` is the viewer. `build.py` inlines the JSON into it to make one
  self-contained page.

The only hand-maintained input is `STAGES` in `extract.py`, which places each crate in a
pipeline stage. New crates show up under "Unassigned" (and on the Doc drift tab) until
they are added there.

Query edges marked "indirect" follow same-crate helper functions by name, so they are a
best guess. Direct edges are calls written inside the `impl QueryFn` block.
