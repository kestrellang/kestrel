/// The SQL type — safe string interpolation that produces parameterized queries.
///
/// `"\(expr)"` targeting SQL emits `?` placeholders and collects bindings,
/// preventing SQL injection at compile time. Only `Bindable` types can be
/// interpolated; everything else is a type error.
///
/// ```
/// let name = "Alice";
/// let q: SQL = "select * from users where name = \(name)";
/// // q.template  == "select * from users where name = ?"
/// // q.bindings  == [.Text("Alice")]
/// ```

module talon.sqlite.sql

import talon.sqlite.value.(SqliteValue, Bindable)

/// Accumulator that builds parameterized SQL. Appends `?` for each
/// interpolation and collects the bound value.
public struct SqlAccumulator: Interpolatable, Cloneable {
    var template: StringBuilder
    var bindings: Array[SqliteValue]

    public init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64) {
        self.template = StringBuilder(capacity: literalCapacity + interpolationCount * 2);
        self.bindings = Array[SqliteValue]();
    }

    public func clone() -> SqlAccumulator {
        var c = SqlAccumulator(literalCapacity: 0, interpolationCount: 0);
        c.template = self.template.clone();
        c.bindings = self.bindings.clone();
        c
    }

    public mutating func appendLiteral(literal: String) {
        self.template.append(literal);
    }

    public mutating func appendInterpolation[T](value: T) where T: Bindable {
        self.template.append("?");
        self.bindings.append(value.toSqliteValue());
    }

    public mutating func build() -> SQL {
        SQL(template: self.template.build(), bindings: self.bindings)
    }
}

/// A parameterized SQL query with `?` placeholders and bound values.
public struct SQL: ExpressibleByStringInterpolation, Cloneable {
    public type Interpolation = SqlAccumulator

    public var template: String
    public var bindings: Array[SqliteValue]

    public init(template template: String, bindings bindings: Array[SqliteValue]) {
        self.template = template;
        self.bindings = bindings;
    }

    public init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.bindings = [];
        self.template = String(stringLiteral: ptr, length);
    }

    public init(interpolation: SqlAccumulator) {
        var acc = interpolation;
        self.template = acc.template.build();
        self.bindings = acc.bindings;
    }

    public func clone() -> SQL {
        SQL(template: self.template.clone(), bindings: self.bindings.clone())
    }
}
