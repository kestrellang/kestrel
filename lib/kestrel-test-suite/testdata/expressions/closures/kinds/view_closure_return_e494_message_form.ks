// test: diagnostics
// stdlib: false

// E494's MESSAGE FORM for a closure carrier, pinned by text rather than by
// code. The other 23 closure tests that assert E494 match on the code alone,
// so nothing else notices if the wording drifts.
//
// docs/design/closures.md, Diagnostics table: E494 is "unchanged for view
// kinds; message gains a fix-it suggesting an `escaping` or `consuming` owning
// type". The fix-it itself is a diagnostic NOTE (plan, decision 7):
//
//   a normal or `mutating` closure holds VIEWS into this frame, so it cannot
//   leave it
//   write an owning kind in the expected/return type — `escaping (…) -> …`
//   (shared environment, callable many times) or `consuming (…) -> …` (unique
//   environment, called once) — and the literal is rebuilt with owned captures
//
// The `.ks` harness matches a diagnostic's message + primary label only; a
// diagnostic's notes are not visible to inline annotations at all. The note
// text is therefore asserted directly, in
// `kestrel_mir::verify::tests::escape_view_closure_return_note_suggests_owning_kind`.
// This file pins the half the harness CAN see, and pins that the view kind is
// still rejected at all (the positive `escaping` sibling is
// memory_model/closure_kinds/escaping/).
module Test

func makeAdder(n: lang.i64) -> () -> lang.i64 {
    let base = lang.i64_add(n, 1);
    { base } // ERROR: cannot return this closure: it captures
}
