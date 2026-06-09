// test: diagnostics
// stdlib: false

// Stage 0.5: reference returns parse but are not supported yet — this is
// the one position stage 1 carves out of the rejection walk.
module Test

func giveRef(x: lang.i64) -> &lang.i64 { x } // ERROR(E481)
