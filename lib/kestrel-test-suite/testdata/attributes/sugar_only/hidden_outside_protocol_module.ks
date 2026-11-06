// test: diagnostics
// stdlib: false
// include: _counter_lib.ks

module Test

// The requirement and the conformer's witness are both hidden outside `Counter`.
func direct(c: Counter.C) -> lang.i64 { c.bumpHook() } // ERROR: 'bumpHook' can only be used through its syntax
func generic[T](t: T) -> lang.i64 where T: Counter.Bump { t.bumpHook() } // ERROR: 'bumpHook' can only be used through its syntax

// Unmarked requirements and extension methods stay callable.
func allowed(c: Counter.C) -> lang.i64 { lang.i64_add(c.plain(), c.viaDefault()) }

// A conformer's own module may call its own implementation by name...
struct Mine: Counter.Bump {
    func bumpHook() -> lang.i64 { 2 }
    func plain() -> lang.i64 { self.bumpHook() }
}

// ...but not another module's.
func otherModules(c: Counter.C, m: Mine) -> lang.i64 { lang.i64_add(m.bumpHook(), c.bumpHook()) } // ERROR: 'bumpHook' can only be used through its syntax
