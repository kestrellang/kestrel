// Test case with nested early returns - should trigger verifier error

public enum Optional[T] {
    case Some(T)
    case None
}

public func test(a: lang.i64, b: lang.i64) -> Optional[lang.i64] {
    // First early return
    if a >= 10 {
        return .None
    }
    
    // Second early return  
    if b >= 20 {
        return .None
    }
    
    // Third early return
    if a == b {
        return .None
    }
    
    // Finally a success case
    return .Some(a + b)
}
