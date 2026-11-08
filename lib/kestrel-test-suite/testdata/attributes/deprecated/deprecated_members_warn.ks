// test: diagnostics
// stdlib: false

module Test

struct Counter {
    @deprecated("read 'count' instead")
    var value: lang.i64

    var count: lang.i64

    // The type's own code may set and read its deprecated members.
    init(value: lang.i64) {
        self.value = value;
        self.count = value;
        self.reset();
    }

    @deprecated
    func reset() {}

    @deprecated
    var doubled: lang.i64 { self.count }
}

func use() -> lang.i64 {
    let c = Counter(1);
    c.reset(); // WARN: 'reset' is deprecated
    let d = c.doubled; // WARN: 'doubled' is deprecated
    c.value // WARN: 'value' is deprecated: read 'count' instead
}
