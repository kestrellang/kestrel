// test: diagnostics
// stdlib: false

// E210: `&mutating` needs a MUTABLE place — let locals, let-rooted field
// chains, and shared-`&` reaches reject; shared `&` of the same places is
// fine (no annotations on those lines).
module Test

struct Pair {
    var a: lang.i64
    var b: lang.i64
}

struct Box {
    var v: lang.i64
    func peek() -> &lang.i64 { self.v }
}

func f() {
    let x: lang.i64 = 1;
    let m1 = &mutating x; // ERROR(E210)
    let s1 = &x;

    let p = Pair(a: 1, b: 2);
    let m2 = &mutating p.a; // ERROR(E210)
    let s2 = &p.a;

    let bx = Box(v: 3);
    let m3 = &mutating bx.peek(); // ERROR(E210)
    let s3 = &bx.peek();
}
