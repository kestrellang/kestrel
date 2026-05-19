// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.text.(StringBuilder, Interpolatable, ExpressibleByStringInterpolation, Formattable, FormatOptions)

// A custom protocol that only Int64 and String conform to
protocol Bindable {
    func bindValue() -> String
}

extend Int64: Bindable {
    func bindValue() -> String { self.formatted() }
}

extend String: Bindable {
    func bindValue() -> String { self }
}

// Accumulator that only accepts Bindable values (not Formattable)
struct SqlAccumulator: Interpolatable, Cloneable {
    var template: String
    var paramCount: Int64

    init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64) {
        self.template = "";
        self.paramCount = 0;
    }

    public func clone() -> SqlAccumulator {
        var c = SqlAccumulator(literalCapacity: 0, interpolationCount: 0);
        c.template = self.template.clone();
        c.paramCount = self.paramCount;
        c
    }

    mutating func appendLiteral(literal: String) {
        self.template.append(literal);
    }

    // Different constraint: some Bindable instead of some Formattable
    mutating func appendInterpolation[T](value: T) where T: Bindable {
        self.paramCount = self.paramCount + 1;
        self.template.append("?");
    }

    mutating func build() -> SqlQuery {
        SqlQuery(template: self.template, paramCount: self.paramCount)
    }
}

struct SqlQuery: ExpressibleByStringInterpolation, Cloneable {
    type Interpolation = SqlAccumulator

    var template: String
    var paramCount: Int64

    init(template template: String, paramCount paramCount: Int64) {
        self.template = template;
        self.paramCount = paramCount;
    }

    init(stringLiteral value: lang.str) {
        self.template = "";
        self.paramCount = 0;
    }

    init(interpolation interpolation: SqlAccumulator) {
        var acc = interpolation;
        let built = acc.build();
        self.template = built.template;
        self.paramCount = built.paramCount;
    }

    public func clone() -> SqlQuery {
        SqlQuery(template: self.template.clone(), paramCount: self.paramCount)
    }
}

func main() -> lang.i64 {
    let name = "Alice";
    let age: Int64 = 30;

    // Bindable constraint: Int64 and String both conform
    let q: SqlQuery = "SELECT * FROM users WHERE name = \(name) AND age = \(age)";
    if q.template != "SELECT * FROM users WHERE name = ? AND age = ?" { return 1 }
    if q.paramCount != 2 { return 2 }

    // Single interpolation
    let q2: SqlQuery = "SELECT \(1)";
    if q2.template != "SELECT ?" { return 3 }
    if q2.paramCount != 1 { return 4 }

    0
}
