// test: diagnostics
// stdlib: false

module Test

protocol Combinable {
    func combine(a: Self, b: Self) -> Self
}
func combineThree[T](x: T, y: T, z: T) -> T where T: Combinable {
    let partial: T = x.combine(y, z);
    return partial
}
func main() {}
