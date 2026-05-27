# kestrel-mir-3

OSSA MIR data structures, monomorphizer, and passes. This file documents
known rules and invariants. It is not exhaustive — when you discover a new
rule, add it here.

## Verifier

`verify::verify_ossa` runs after lowering and after mono passes. It checks:
- Value uniqueness (no ValueId defined twice)
- Linear ownership (@owned consumed exactly once)
- Borrow scoping (EndBorrow before block exit)
- Block arg count, type, and ownership match
- Address init/uninit tracking (Uninit → StoreInit → Take)

It also checks that every operand has a definition (block param or
instruction result). A value used but never defined crashes codegen with
"ValueId not in value_map" — the verifier catches this earlier.

## Witness resolution

`mono/witness.rs::find_witness_with_method` matches witnesses by
`(protocol, self_type, method_key)`. For generic protocols like
`Convertible[From]`, multiple witnesses exist for the same
`(protocol, self_type)` pair (one per source type). The lookup filters
by `proto_type_args` using `witness_proto_args_match` — this compares the
witness's protocol type args against the expected args from the call site's
`method_type_args`. Without this filter, the first matching witness wins
regardless of which generic instantiation it represents.

`witness_proto_args_match` treats `TypeParam` entries as wildcards (they
come from `extend T: Proto[FreeParam]` where the free param has no
concrete value at witness-construction time).
