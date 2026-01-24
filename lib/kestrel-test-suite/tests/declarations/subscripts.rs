use kestrel_test_suite::*;

mod basic {
    use super::*;

    #[test]
    fn subscript_with_parameter() {
        Test::new(
            r#"module Test
            struct Int {}
            struct Container[T] {
                private var data: T

                public init(data: T) {
                    self.data = data
                }

                public subscript(index: lang.i64) -> T {
                    get {
                        self.data
                    }
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn subscript_with_setter() {
        Test::new(
            r#"module Test
            struct Int {}
            struct Container[T] {
                private var data: T

                public init(data: T) {
                    self.data = data
                }

                public subscript(index: lang.i64) -> T {
                    get {
                        self.data
                    }
                    set {
                        self.data = newValue
                    }
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}

mod execution {
    use super::*;

    /// Test that subscripts with their own type parameters work end-to-end.
    /// This tests several fixes:
    /// - Subscript getters are lowered (KestrelSymbolKind::Subscript in item.rs)
    /// - Subscript getter parameters are mapped (not just self)
    /// - Subscript's own type parameters are registered
    /// - Subscript call type arguments are extracted from argument types
    #[test]
    fn generic_subscript_executes() {
        Test::new(
            r#"module Test
            import std.core.Formattable
            import std.text.String

            struct Formatter {
                public init() {}

                public subscript[F](value: F) -> String where F: Formattable {
                    get {
                        value.format()
                    }
                }
            }

            func main() -> lang.i64 {
                let f = Formatter();
                let s = f(42);
                // Return byte count of "42" which is 2
                s.byteCount().raw
            }
        "#,
        )
        .with_stdlib()
        .expect(ExitCode(2));
    }
}

mod regression {
    use super::*;

    /// Regression test for: Subscript parameters not bound in getter/setter body
    /// Issue: Using a subscript parameter name like `index` in the getter or setter body
    /// caused an "undefined name 'index'" error because the SubscriptBinder was correctly
    /// adding parameters to the local scope, but the GetterBinder was also being invoked
    /// and creating a new LocalScope without the parameters, overwriting the correct one.
    ///
    /// Root cause: Both SubscriptBinder and GetterBinder were binding the getter/setter
    /// bodies. The GetterBinder ran second and overwrote the ExecutableBehavior that the
    /// SubscriptBinder had correctly created with parameters.
    ///
    /// Fix: GetterBinder and SetterBinder now check if their parent is a Subscript and
    /// skip body binding in that case, leaving it to SubscriptBinder.
    #[test]
    fn subscript_parameter_accessible_in_getter() {
        Test::new(
            r#"module Test
            struct Int {}
            struct Container[T] {
                private var data: T

                public init(data: T) {
                    self.data = data
                }

                public subscript(dummy: lang.i64) -> T {
                    get {
                        // Use dummy to ensure it's accessible
                        let _unused = dummy;
                        self.data
                    }
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    /// Regression test for subscript parameters in setter body
    #[test]
    fn subscript_parameter_accessible_in_setter() {
        Test::new(
            r#"module Test
            struct Int {}
            struct Container[T] {
                private var data1: T
                private var data2: T

                public init(data: T) {
                    self.data1 = data;
                    self.data2 = data;
                }

                public subscript(dummy: lang.i64) -> T {
                    get {
                        // Use dummy to ensure it's accessible
                        let _unused = dummy;
                        self.data1
                    }
                    set {
                        // Use dummy to ensure it's accessible
                        let _unused = dummy;
                        self.data1 = newValue
                    }
                }
            }
        "#,
        )
        .expect(Compiles);
    }

    /// Test multiple parameters in subscript
    #[test]
    fn subscript_multiple_parameters() {
        Test::new(
            r#"module Test
            struct Int {}
            struct Matrix[T] {
                private var data: T

                public init(data: T) {
                    self.data = data
                }

                public subscript(row: lang.i64, col: lang.i64) -> T {
                    get {
                        // Use row and col to ensure they're accessible
                        let _unused1 = row;
                        let _unused2 = col;
                        self.data
                    }
                }
            }
        "#,
        )
        .expect(Compiles);
    }
}
