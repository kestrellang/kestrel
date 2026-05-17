# kestrel-pattern-matching — Agent Guide

Patterns for extending or modifying the Maranget-style usefulness and
exhaustiveness algorithms.

## Fix the algorithm, not the consumer

When a diagnostic in the analyzer looks wrong, don't filter it after the
fact. Fix the matrix-level computation so `redundant_arms`,
`overlapping_arms`, and `missing_patterns` are semantically correct for
all consumers (diagnostics AND decision-tree codegen).

Precedent: `Constructor::matches` treats overlapping integer ranges as
fully compatible, which makes `specialize` report partial overlaps as
redundant. The fix lives in `usefulness::check_match` (union-coverage
over `prior_int_ranges` / `prior_char_ranges`), not in the analyzer that
reports E306. The analyzer just reports what the algorithm says.

If an analyzer post-hoc filter is the only option, document *why* the
matrix can't be fixed. Otherwise move the logic into this crate.

## Overlap vs redundancy are mutually exclusive

An arm is **redundant** (`redundant_arms`) when all its values are
covered by earlier arms. An arm is **overlapping** (`overlapping_arms`)
when it shares *some* values with earlier arms but owns *some new ones*.
These are disjoint — an arm cannot be in both lists.

Consumers rely on this: the diagnostic layer picks E307 (overlap) for
`overlapping_arms` and E306 (redundant) for `redundant_arms` without
needing to dedup. If you change the classification, preserve this
invariant.

## Range handling

Range arms (`IntRange`, `CharRange`) and integer/char literals all
participate in interval-coverage tracking. `extract_int_range` /
`extract_char_range` normalize them to `(start, end)` tuples that feed
`range_covered_by_union_*`.

Rules:
- Open bounds (`None`) widen to the type's extremum (`i64::MIN`/`MAX`,
  `0`/`char::MAX`), not skipped — an unbounded range IS a range.
- Empty ranges (`start > end`, e.g. `10..=0`) are skipped: they are
  syntactically valid but semantically empty, and should be flagged by a
  future bounds-validation pass, not silently coerced to "vacuously
  covered."
- Guarded arms don't contribute to `prior_*_ranges` — a guard can fail
  at runtime so the arm doesn't actually cover its interval.

## Deduplication invariants

Each piece of logic lives in exactly one place. Do not duplicate:

- **Pattern decomposition** → `FlatPat::decompose()` (used by both
  `matrix::specialize` and the decision tree compiler).
- **Constructor field types** → `Constructor::field_types()`.
- **Constructor matching** → `Constructor::matches()`.
- **Type classification** → `TypeShape::classify()`.

If you need similar logic elsewhere, extend the canonical function
rather than reimplementing.
