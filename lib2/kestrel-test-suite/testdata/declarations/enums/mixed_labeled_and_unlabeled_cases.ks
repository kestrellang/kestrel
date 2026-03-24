// test: diagnostics
// stdlib: false

module Test

enum Message {
    case Text(lang.str)
    case Number(value: lang.i64)
    case Pair(lang.str, lang.i64)
}
