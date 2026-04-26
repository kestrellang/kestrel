// test: diagnostics
// stdlib: false

module Main

func identity[T](x: T) -> T { x }

struct Pair[A, B] {
    let first: A
    let second: B

    func getFirst() -> A { self.first }
    func getSecond() -> B { self.second }
}

func main() -> lang.i64 {
    let x = identity[lang.i64](42);
    let s = identity[lang.i1](true);

    let p = Pair[lang.i64, lang.i64](first: 10, second: 20);
    let a = p.getFirst();
    let b = p.getSecond();

    lang.i64_add(lang.i64_add(x, a), b)
}
