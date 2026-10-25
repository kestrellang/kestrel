// test: execution
// stdlib: true

module Test

@main
func main() -> lang.i64 {
    // SystemRandom draws stay in requested ranges
    var i: Int64 = 0;
    while i < 50 {
        let v = Int64.random(in: 0..<100);
        if v < 0 { return 1 }
        if v >= 100 { return 2 }
        i = i + 1
    }

    // three consecutive OS-entropy draws are not all identical
    var sys = SystemRandom();
    let a = sys.nextUInt64();
    let b = sys.nextUInt64();
    let c = sys.nextUInt64();
    if a == b and b == c { return 3 }

    // Bool.random / Float64.random conveniences execute and stay in range
    let coin = Bool.random();
    let unit = Float64.random();
    if unit < 0.0 { return 4 }
    if unit >= 1.0 { return 5 }
    let ranged = Float64.random(from: 10.0, to: 20.0);
    if ranged < 10.0 { return 6 }
    if ranged >= 20.0 { return 7 }

    // randomElement: None on empty, member of the collection otherwise,
    // deterministic with a seeded generator
    var arr = Array[Int64]();
    match arr.randomElement() {
        .Some(x) => { return 8 },
        .None => { let _ = 0; }
    }
    arr.append(10); arr.append(20); arr.append(30);
    var rng = Lcg64(seed: 42);
    match arr.randomElement(using: rng) {
        .Some(x) => {
            if arr.contains(x) == false { return 9 }
        },
        .None => { return 10 }
    }
    var rng2 = Lcg64(seed: 42);
    let p1 = arr.randomElement(using: rng2);
    var rng3 = Lcg64(seed: 42);
    let p2 = arr.randomElement(using: rng3);
    if p1 != p2 { return 11 }

    // shuffle(using:) advances the caller's generator state
    var s1 = Lcg64(seed: 11);
    var deck = Array[Int64]();
    var d: Int64 = 0;
    while d < 8 { deck.append(d); d = d + 1 }
    deck.shuffle(using: s1);
    var s2 = Lcg64(seed: 11);
    if s1.nextUInt64() == s2.nextUInt64() { return 12 }

    // no-arg shuffle keeps the same elements
    var deck2 = Array[Int64]();
    deck2.append(1); deck2.append(2); deck2.append(3);
    deck2.shuffle();
    if deck2.count != 3 { return 13 }
    if deck2.contains(1) == false { return 14 }
    if deck2.contains(2) == false { return 15 }
    if deck2.contains(3) == false { return 16 }

    0
}
