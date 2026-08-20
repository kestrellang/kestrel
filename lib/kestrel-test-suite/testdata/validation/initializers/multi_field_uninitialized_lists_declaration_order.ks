// test: diagnostics
// stdlib: false

// E005's message interpolates the uninitialized field names, so their order is
// user-visible text. It used to come from a `HashSet` — 25 builds of this exact
// file produced 17 different orderings (F43c). The set is an `IndexSet` fed by
// `children_of_kind`, so the list is now declaration order, always.
//
// The names are deliberately NOT in alphabetical order: `width, height, alpha,
// depth` distinguishes declaration order from a `BTreeSet`'s alphabetical order
// (`alpha, depth, height, width`) as well as from any hash order. Four fields
// makes an accidental match vanishingly unlikely.

module Main

struct Config {
    var width: lang.i64
    var height: lang.i64
    var alpha: lang.i64
    var depth: lang.i64

    init() {
    } // ERROR: initializer does not initialize all fields: 'width', 'height', 'alpha', 'depth'
}
