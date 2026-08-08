// test: diagnostics
// stdlib: false

// PINS A KNOWN v1 LIMITATION — not the desired end state.
//
// Plan D4 (docs/plans/closure-kinds/closure-kinds-plan.md): "v1 limitation
// (pinned by test): nested-closure captures still collapse to whole-local
// (`force_whole` TODO in captures.rs) — transitive narrowest-place capture
// through nested closures is a NAMED FOLLOW-UP." The collapse lives in
// `Recorder::record` in lib/kestrel-type-infer/src/captures.rs, which rewrites
// the captured `PlaceKey` to `PlaceKey::whole(key.root)` whenever `force_whole`
// is set (i.e. anywhere inside a nested closure), because sub-places are not
// threaded through the inner closure's env struct.
//
// Consequence pinned here: the freeze (plan D8) is place-granular and would
// normally cover only `p.a`, leaving the unrelated field `p.b` free to be
// consumed. Because the read happens inside a NESTED closure, the capture is
// widened to the whole local `p`, so the freeze covers `p.b` too and the
// consume is rejected. That E507 is OVER-STRICT: when the follow-up lands and
// nested closures capture the narrowest place, this line becomes legal and
// this test must be re-baselined (the `sink(p.b)` annotation is deleted).
//
// Contrast: memory_model/closure_kinds/view/view_capture_of_self_field_reads_
// through.ks shows the non-nested case, where `{ self.n }` really does capture
// only the place `self.n`.
module Test

struct Res: not Copyable {
    var v: lang.i64
}

struct Pair: not Copyable {
    var a: lang.i64
    var b: Res
}

func sink(consuming r: Res) -> lang.i64 { r.v }

func test() -> lang.i64 {
    let p = Pair(a: 1, b: Res(v: 2));
    let outer = { () in
        let inner = { p.a };   // nested: `force_whole` collapses `p.a` -> whole `p`
        inner()
    };
    let taken = sink(p.b);     // ERROR(E507)
    lang.i64_add(taken, outer())
}
