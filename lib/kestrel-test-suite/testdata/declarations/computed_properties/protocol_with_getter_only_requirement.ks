// test: diagnostics
// stdlib: false

module Test

protocol Named {
    var name: lang.str { get }
}

struct Person: Named {
    var name: lang.str
}
