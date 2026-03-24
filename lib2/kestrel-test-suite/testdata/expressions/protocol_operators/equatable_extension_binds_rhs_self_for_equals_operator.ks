// test: diagnostics
// stdlib: true

module Test

public enum LocalOrdering: std.core.Equatable {
    case Less
    case Equal

    public func equals(other: LocalOrdering) -> std.core.Bool {
        true
    }
}

public func test() -> std.core.Bool {
    LocalOrdering.Less == LocalOrdering.Equal
}
