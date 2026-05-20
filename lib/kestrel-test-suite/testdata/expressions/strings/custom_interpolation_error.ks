// test: diagnostics
// stdlib: true

module Test

import std.text.(StringBuilder, Interpolatable, ExpressibleByStringInterpolation, Formattable, FormatOptions)

protocol Bindable {
    func bindValue() -> String
}

extend Int64: Bindable {
    func bindValue() -> String { self.formatted() }
}

// Bool does NOT conform to Bindable

struct SqlAccumulator: Interpolatable, Cloneable {
    var template: String

    init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64) {
        self.template = "";
    }

    public func clone() -> SqlAccumulator {
        var c = SqlAccumulator(literalCapacity: 0, interpolationCount: 0);
        c.template = self.template.clone();
        c
    }

    mutating func appendLiteral(literal: String) {
        self.template.append(literal);
    }

    mutating func appendInterpolation[T](value: T) where T: Bindable {
        self.template.append("?");
    }

    mutating func build() -> SqlQuery {
        SqlQuery(template: self.template)
    }
}

struct SqlQuery: ExpressibleByStringInterpolation, Cloneable {
    type Interpolation = SqlAccumulator

    var template: String

    init(template template: String) {
        self.template = template;
    }

    init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.template = String(stringLiteral: ptr, length);
    }

    init(interpolation: SqlAccumulator) {
        var acc = interpolation;
        self.template = acc.build().template;
    }

    public func clone() -> SqlQuery {
        SqlQuery(template: self.template.clone())
    }
}

func main() -> lang.i64 {
    let flag = true;
    let q: SqlQuery = "SELECT * FROM users WHERE active = \(flag)"; // ERROR: Bool !: Bindable
    0
}
