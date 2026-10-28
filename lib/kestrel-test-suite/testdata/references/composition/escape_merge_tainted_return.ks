// test: diagnostics
// stdlib: true

// References 2b: taint joins through a merge — one local-tainted arm
// poisons the merged result even though the other arm is .None
// (conservative over-rejection, the sound direction).
module Test

func maybeDangle(c: Bool) -> Optional[&Int64] {
    var x = 1;
    let r = &x;
    let o: Optional[&Int64] = if c {
        .Some(r)
    } else {
        .None
    };
    o // ERROR(E494)
}
