// Minimal reproduction case for subscript call issue

module subscript_test

// Type with subscript
public struct Container[T] {
    private var data: T

    public init(data: T) {
        self.data = data
    }

    // Subscript with labeled parameter
    public subscript(unchecked n: Int64) -> T {
        get { self.data }
        set { self.data = newValue }
    }

    // Subscript without label
    public subscript(n: Int64) -> T {
        get { self.data }
        set { self.data = newValue }
    }
}

// Test function - should work but fails
public func testLabeled() -> Int64 {
    let c = Container(data: 42);
    c(unchecked: 0)  // Should call subscript(unchecked:)
}

// Test function - should this work?
public func testUnlabeled() -> Int64 {
    let c = Container(data: 42);
    c(0)  // Should call subscript(_:)
}
