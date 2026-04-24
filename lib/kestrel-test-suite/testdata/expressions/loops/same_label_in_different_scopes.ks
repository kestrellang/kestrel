// test: diagnostics
// stdlib: false

module Main

func test() {
    myloop: while true {
        break myloop;
    }
    myloop: loop {
        break myloop;
    }
}
