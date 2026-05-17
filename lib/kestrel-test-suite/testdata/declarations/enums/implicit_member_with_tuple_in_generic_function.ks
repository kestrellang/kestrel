// test: diagnostics
// stdlib: false

module Test

public enum Option[T] {
    case Some(T)
    case None
}

// Generic function that takes Option[T]
public func process[T](opt: Option[T]) -> lang.i64 {
    match opt {
        .Some(_) => 1,
        .None => 0
    }
}

// Test: inline tuple in .Some() with generic function
public func test1() -> lang.i64 {
    process(.Some((5, 10)))
}

// Test: identity function with implicit member
public func identity[T](x: Option[T]) -> Option[T] {
    x
}

public func test2() -> Option[(lang.i64, lang.i64)] {
    identity(.Some((5, 10)))
}

// Test: with more complex tuple types
public func test3() -> lang.i64 {
    process(.Some((1, 2, 3)))
}
