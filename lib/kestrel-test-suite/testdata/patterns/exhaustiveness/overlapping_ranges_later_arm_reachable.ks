// test: diagnostics
// stdlib: false
//
// F1 — a later arm made unreachable by the UNION of earlier overlapping ranges
// is redundant; one that still owns some values is only an overlap warning.

module Main

func covered(x: lang.i64) -> lang.i64 {
    match x {
        0..=10 => 1,
        5..=15 => 2, // WARN: overlap
        3..=12 => 3, // WARN: unreachable
        _ => 0
    }
}

func notCovered(x: lang.i64) -> lang.i64 {
    match x {
        0..=10 => 1,
        12..=15 => 2,
        3..=13 => 3, // WARN: overlap
        _ => 0
    }
}
