// Helper: minimal Prelude module with literal protocol definitions.
// Include this in tests that reference Prelude.ExpressibleBy* without stdlib.
module Prelude

@builtin(.ExpressibleByIntLiteral)
protocol ExpressibleByIntegerLiteral {
    init(intLiteral value: lang.i64)
}

@builtin(.ExpressibleByFloatLiteral)
protocol ExpressibleByFloatLiteral {
    init(floatLiteral value: lang.f64)
}

@builtin(.ExpressibleByStringLiteral)
protocol ExpressibleByStringLiteral {
    init(stringLiteral value: lang.str)
}

@builtin(.ExpressibleByBoolLiteral)
protocol ExpressibleByBoolLiteral {
    init(boolLiteral value: lang.i1)
}

@builtin(.ExpressibleByNullLiteral)
protocol ExpressibleByNullLiteral {
    init()
}
