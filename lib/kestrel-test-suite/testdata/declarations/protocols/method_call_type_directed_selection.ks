// test: diagnostics
// stdlib: false

module Test

public struct SmallValue {
    var x: lang.i8
    public init() { self.x = lang.cast_i64_i8(5) }
}

public struct LargeValue {
    var x: lang.i32
    public init() { self.x = lang.cast_i64_i32(100) }
}

public struct Processor {
    var result: lang.i64

    public init() { self.result = 0 }

    public func processSmall(value: SmallValue) -> lang.i64 {
        lang.cast_i8_i64(value.x)
    }

    public func processLarge(value: LargeValue) -> lang.i64 {
        lang.cast_i32_i64(value.x)
    }
}

public func test() {
    let p = Processor();
    let s = SmallValue();
    let l = LargeValue();

    // Method calls - function params don't have external labels by default
    let r1 = p.processSmall(s);
    let r2 = p.processLarge(l);
}
