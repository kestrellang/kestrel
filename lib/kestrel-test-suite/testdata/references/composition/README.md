# references/composition — stage 2b (refs in enums/tuples/structs)

~~KNOWN GAP~~ FIXED 2026-06-11 (stage 2d, C1): a protocol-dispatched
OPERATOR on a ref-instantiated container — `a == b` at `Optional[&Int64]`
— used to bypass the ConformsOrigin gate AND the bound-aware
`type_satisfies` and ICE post-mono ("Callee::Witness not resolved"). The
bypass was `nominal_satisfies` trusting INDIRECT conformance sources
unconditionally: operator protocols arrive via the blanket
`extend Equatable: Equal[Self]` (an extension targeting a PROTOCOL) and
via refinement parents (`protocol Comparable: Equatable`), and the early
non-Extension/empty-clause accept skipped the genuine requirement. Now
both source shapes gate on `type_satisfies(recv, parent_protocol)`, so
the where clause of `extend Optional[T]: Equatable` is evaluated at
`T = &Int64`. As of 2026-06-12 that evaluation SUCCEEDS: the stdlib
forwarding extension `extend &T: Equatable where T: Equatable`
(core/ref.ks, via the generic synthetic `lang.&` entity) supplies the
conformance and the operators dispatch the pointee's witnesses end to
end (`op_dispatch_ref_payload_works.ks`; positive refinement direction
pinned by `op_dispatch_refinement_still_conforms.ks`; the reject side —
protocols no ref extension declares — by
`references/extend_ref/stdlib_ref_conformance_surface.ks`).
