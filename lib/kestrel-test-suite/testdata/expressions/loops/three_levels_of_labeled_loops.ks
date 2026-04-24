// test: diagnostics
// stdlib: false

module Main

func test() {
    a: while true {
        b: while true {
            c: while true {
                break a;
            }
            break b;
        }
        break;
    }
}
