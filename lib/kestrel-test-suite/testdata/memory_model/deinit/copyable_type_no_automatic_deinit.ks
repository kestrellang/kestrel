// test: diagnostics
// stdlib: false

module Test
struct Counter {
    var count: lang.i64
}

func example() {
    let c = Counter(count: 0);
}
