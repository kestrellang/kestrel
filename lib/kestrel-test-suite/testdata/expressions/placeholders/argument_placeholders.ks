// test: execution
// stdlib: true
// expect-exit: 0

// `_` inside a larger argument makes that argument a one-parameter closure;
// every `_` in it is the same parameter.

module Test

struct Address {
    let city: String
}

struct User {
    let first: String
    let last: String
    let age: Int64
    let address: Address?
}

func double(x: Int64) -> Int64 { x * 2 }

@main
func main() -> lang.i64 {
    let users = [
        User(first: "ada", last: "lovelace", age: 36, address: .Some(Address(city: "london"))),
        User(first: "kid", last: "x", age: 12, address: .None),
    ];
    // member access
    let names = users.map(_.first);
    if names(0) != "ada" or names(1) != "kid" { return 1 }
    // a whole comparison is the closure body
    let adults = users.filter(where: _.age >= 18);
    if adults.count != 1 { return 2 }
    // chained members
    let lens = users.map(_.first.bytes.count);
    if lens(0) != 3 { return 3 }
    // operators
    let xs = [1, 2, 3];
    if xs.map(_ * 2)(2) != 6 { return 4 }
    if xs.map(-_)(0) != -1 { return 5 }
    // one parameter: `_ + _` doubles
    if xs.map(_ + _)(1) != 4 { return 6 }
    // `and` keeps one parameter across both sides
    let between = [0, 5, 12].filter(where: _ > 0 and _ < 10);
    if between.count != 1 or between(0) != 5 { return 7 }
    // several members joined
    let full = users.map(_.first + " " + _.last);
    if full(0) != "ada lovelace" { return 8 }
    // string interpolation passes through
    let tags = users.map("\(_.first)!");
    if tags(1) != "kid!" { return 9 }
    // `??` passes through
    let ages = [Optional.Some(1), .None].map(_ ?? 0);
    if ages(1) != 0 { return 10 }
    // calling each element
    let fs: Array[(Int64) -> Int64] = [double, { it + 1 }];
    let applied = fs.map(_(5));
    if applied(0) != 10 or applied(1) != 6 { return 11 }
    // sort key
    let byAge = users.sorted(byKey: _.age);
    if byAge(0).first != "kid" { return 12 }
    0
}
