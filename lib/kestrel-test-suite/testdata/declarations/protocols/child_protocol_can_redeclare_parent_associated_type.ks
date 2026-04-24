// test: diagnostics
// stdlib: false
module Test
public protocol _ExpressibleByArrayLiteral {
    type Element
}

public protocol ExpressibleByArrayLiteral: _ExpressibleByArrayLiteral {
    type Element
}
