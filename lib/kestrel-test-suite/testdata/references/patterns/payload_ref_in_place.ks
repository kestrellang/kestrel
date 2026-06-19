// test: execution
// stdlib: false
// backends: cranelift,llvm

// The `&` pattern flagship (stage 1.5 item 2): a NotCopyable enum payload
// read IN PLACE — `&v` borrows the payload where it sits (the match proves
// the tag before the projection), so no copy/move of the payload ever
// happens. Works on borrowed-param scrutinees and on named-binding
// scrutinees (`match r`).
module Test

struct Res: not Copyable {
    var v: lang.i64
}

enum Bucket {
    case Occupied(lang.i64, Res, lang.i64)
    case Empty
}

func readThrough(b: Bucket) -> lang.i64 {
    match b {
        .Occupied(_, &v, _) => v.v,
        .Empty => 0
    }
}

@main
func main() -> lang.i64 {
    let b = Bucket.Occupied(7, Res(v: 42), 9);
    let direct = readThrough(b);
    let r = &b;
    let viaBinding = match r {
        .Occupied(_, &v, _) => v.v,
        .Empty => 1
    };
    let empty = readThrough(Bucket.Empty);
    if lang.i64_eq(direct, 42) { } else { return 1; }
    if lang.i64_eq(viaBinding, 42) { } else { return 2; }
    if lang.i64_eq(empty, 0) { } else { return 3; }
    0
}
