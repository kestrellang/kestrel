# kestrel-target

`TargetConfig`: the triple, pointer width and `@platform` OS. See `docs/architecture.md`.

- **Every target fact lives here.** MIR layout and codegen once had separate
  pointer widths (MIR's was hard-coded to 8). Read `pointer_size()`; never
  write 8 or 4 for a pointer in a stage.
- **Add a new fact as a method on `TargetConfig`**, not as a stage-local
  constant or a second `--target` parse.
- An OS with no `@platform` case gives `os() == None`, which keeps every
  declaration. Adding an `Os` case changes filtering for that triple.
