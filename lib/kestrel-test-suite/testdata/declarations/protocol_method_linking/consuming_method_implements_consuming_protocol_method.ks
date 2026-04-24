// test: diagnostics
// stdlib: false
module Test

protocol Disposable {
    consuming func dispose()
}
struct Resource: Disposable {
    consuming func dispose() { }
}
