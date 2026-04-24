// test: diagnostics
// stdlib: false

module Test

struct Person {
    var name: lang.str
    var age: lang.i64
    var email: lang.str

    init(name: lang.str, age: lang.i64, email: lang.str) {
        self.name = name;
        self.age = age;
        self.email = email
    }

    init(name: lang.str) {
        self.init(name, 0, "")
    }
}

func test() {
    let p = Person("Alice");
    let _n = p.name;
    let _a = p.age;
    let _e = p.email;
}
