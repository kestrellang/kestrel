// test: diagnostics
// stdlib: false

module Test
            struct NotAProtocol { }
            protocol Container {
                type Item: NotAProtocol;
            }
