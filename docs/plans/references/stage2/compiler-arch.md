# Stage 2 — Compiler architecture

**Status: BLANK — needs exploration** (gated on the stage-2 commitment).
Known cost centers if started: `MonoTypeKey` lifetime-provenance keying
(the cross-instantiation double-free class), drop/clone elaboration for ref
fields, closure-env capture classification vs. the Rc-closure migration.
