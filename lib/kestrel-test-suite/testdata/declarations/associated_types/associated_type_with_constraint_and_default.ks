// test: diagnostics
// stdlib: false

module Test
            protocol Equatable { }
            struct MyInt: Equatable { }
            protocol Container {
                type Item: Equatable = MyInt;
            }
