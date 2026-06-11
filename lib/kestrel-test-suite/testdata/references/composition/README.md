# references/composition — stage 2b (refs in enums/tuples/structs)

KNOWN GAP (2026-06-11, targeted for the bare-ref-as-T follow-up): a
protocol-dispatched OPERATOR on a ref-instantiated container —
`a == b` at `Optional[&Int64]` — bypasses the ConformsOrigin gate AND
the bound-aware `type_satisfies` (the extension's `where T: Equatable`
clause is not consulted at member dispatch), so it surfaces as a
post-mono "Callee::Witness not resolved" ICE instead of a clean
DoesNotConform. The program is still rejected — wrong message, right
verdict. Explicit `where T: P`-bounded generic calls DO gate cleanly.
