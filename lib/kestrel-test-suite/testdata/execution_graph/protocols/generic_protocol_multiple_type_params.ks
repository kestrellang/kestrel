// test: diagnostics
// stdlib: false

module Test

protocol Mapping[K, V] {
    func read(key: K) -> V
}
