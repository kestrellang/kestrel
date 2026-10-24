module Http

@nonExhaustive
public enum Method {
    case Get
    case Post
}

// Inside its own module the enum is closed: listing every case is enough.
public func code(m: Method) -> lang.i64 {
    match m {
        .Get => 1,
        .Post => 2,
    }
}
