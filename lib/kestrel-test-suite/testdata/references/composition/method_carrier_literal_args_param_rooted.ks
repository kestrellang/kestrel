// test: execution
// backends: cranelift,llvm
// stdlib: true
// expect-stdout: 2 2\n

// A method that builds a carrier from its receiver, called with literal
// arguments: an `Int64` argument can't hold the `Array` the carrier points
// at, so the carrier roots at the receiver only and returning it from a
// function whose param is the receiver is fine (used to be a false E494
// rooted at the literal temporaries).
module Test

struct View {
    let base: &Array[Int64]
    let lo: Int64
    func elem(i: Int64) -> Int64 { let a = self.base; a(self.lo + i) }
}

extend Array[Int64] {
    func view(lo lo: Int64) -> View {
        let b = &self;
        View(base: b, lo: lo)
    }
}

func viaMethod(xs: [Int64]) -> View { xs.view(lo: 0) }

@main
func main() {
    let xs: [Int64] = [1, 2, 3];
    let s = xs.view(lo: 0);
    println("\(s.elem(1)) \(viaMethod(xs).elem(1))");
}
