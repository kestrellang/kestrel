// test: diagnostics
// stdlib: false
// skip: stage1 — blocked: try_consume gate surfaces as an uncoded verify ICE; needs a coded diagnostic or a dedicated harness mode before this can be annotated

// Inherited-free pin (tests.md `intra_block_consume_while_borrowed`):
// consuming the owner while a reference into it is live in the same block
// is rejected by the existing OSSA `try_consume` gate. The gate exists and
// fires today, but as an ICE-class verify error the diagnostics harness
// cannot match — revisit once escape diagnostics (T3) settle on how
// uncoded verify errors surface.
module Test

struct Res: not Copyable {
    var v: lang.i64
}

struct Box {
    var r: Res
    func peek() -> &Res { self.r }
    consuming func destroy() {}
}

func use(b: Box) {
    // hold the ref open as a call argument while consuming the owner
    observe(b.peek(), consume(b)); // ERROR
}

func observe(r: Res, u: ()) {}
func consume(b: Box) -> () { b.destroy() }
