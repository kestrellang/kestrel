// Minimal test case for Verifier error

public enum Optional[T] {
    case Some(T)
    case None
}

public func test(x: lang.i64) -> Optional[lang.i64] {
    if x >= 10 {
        return .None
    }
    return .Some(x)
}
