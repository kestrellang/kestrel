// test: diagnostics
// stdlib: false

module Main

func doSomething() -> () { () }

func test() -> () {
    doSomething()
}
