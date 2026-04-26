// test: diagnostics
// stdlib: false

module Test

public struct Small {
    var x: lang.i8
    public init() { self.x = lang.cast_i64_i8(0) }
}

public struct Large {
    var x: lang.i32
    public init() { self.x = lang.cast_i64_i32(0) }
}

public struct Target {
    var value: lang.i64

    public init(fromSmall other: Small) {
        self.value = lang.cast_i8_i64(other.x)
    }

    public init(fromLarge other: Large) {
        self.value = lang.cast_i32_i64(other.x)
    }
}

public func test() {
    let s = Small();
    let l = Large();

    // Different labels - type-directed selection validates correct init is called
    let t1 = Target(fromSmall: s);
    let t2 = Target(fromLarge: l);
}
