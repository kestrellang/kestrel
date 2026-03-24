// test: diagnostics
// stdlib: false
module Test

protocol Container {
    type Item;
}
struct Box[T]: Container {
    type Item = T;
}
