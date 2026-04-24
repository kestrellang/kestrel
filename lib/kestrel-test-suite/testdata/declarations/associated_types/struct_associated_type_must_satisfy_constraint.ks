// test: diagnostics
// stdlib: false

module Test
            protocol Equatable { }
            struct NotEquatable { }
            protocol Container {
                type Item: Equatable;
            }
            struct BadContainer: Container {
                type Item = NotEquatable;
            }
