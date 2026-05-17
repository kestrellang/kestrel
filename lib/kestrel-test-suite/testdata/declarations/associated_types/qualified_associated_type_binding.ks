// test: diagnostics
// stdlib: false
module Test

protocol Iterator {
    type Item;
}
protocol Container {
    type Item;
}
struct Foo: Iterator, Container {
    type Iterator.Item = lang.i64;
    type Container.Item = lang.str;
}
