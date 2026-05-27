module datetime

public enum DateError: Formattable {
    case InvalidDate(year: Int64, month: Int64, day: Int64)
    case InvalidTime(hour: Int64, minute: Int64, second: Int64)

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        match self {
            .InvalidDate(year: y, month: m, day: d) => {
                writer.append("invalid date: \(y)-\(m)-\(d)");
            },
            .InvalidTime(hour: h, minute: m, second: s) => {
                writer.append("invalid time: \(h):\(m):\(s)");
            }
        };
    }
}

public enum ParseError: Formattable {
    case InvalidFormat(String)
    case InvalidValue(String)
    case UnexpectedEnd

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        match self {
            .InvalidFormat(msg) => writer.append("invalid format: \(msg)"),
            .InvalidValue(msg) => writer.append("invalid value: \(msg)"),
            .UnexpectedEnd => writer.append("unexpected end of input")
        };
    }
}
