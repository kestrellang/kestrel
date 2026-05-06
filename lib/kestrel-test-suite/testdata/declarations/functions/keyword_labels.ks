// test: diagnostics
// stdlib: false

module Test

func insert(in list: lang.str, at index: lang.i64) { }
func search(for needle: lang.str, in haystack: lang.str) -> lang.i64 { 0 }
func convert(as format: lang.i64) -> lang.i64 { format }
func check(if condition: lang.i64) -> lang.i64 { condition }
func repeat(while count: lang.i64) { }
func attempt(try action: lang.i64) -> lang.i64 { action }
func handle(match pattern: lang.i64) -> lang.i64 { pattern }
func validate(guard condition: lang.i64) -> lang.i64 { condition }
func configure(set value: lang.i64) { }
func retrieve(get key: lang.str) -> lang.str { key }
func process(or fallback: lang.i64) -> lang.i64 { fallback }
func combine(and other: lang.i64) -> lang.i64 { other }
func toggle(not flag: lang.i64) -> lang.i64 { flag }

// Mixed keyword and identifier labels
func transfer(from source: lang.str, in target: lang.str) { }

// Overloaded by keyword label
func send(to recipient: lang.str) { }
func send(from sender: lang.str) { }
func send(in channel: lang.str) { }
