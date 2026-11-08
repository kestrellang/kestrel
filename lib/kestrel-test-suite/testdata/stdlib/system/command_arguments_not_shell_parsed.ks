// test: execution
// stdlib: true

// Arguments reach the program exactly as given: spaces, quotes, `$` and `;`
// are not interpreted by any shell.

module Test

@main
func main() -> lang.i64 {
    let tricky = "a b 'c' \"d\" $HOME; echo injected";
    match Command("printf").arguments(["%s|%s", tricky, ""]).output() {
        .Ok(o) => {
            let text = o.standardOutputText() ?? "";
            if not text.isEqual(to: "a b 'c' \"d\" $HOME; echo injected|") { return 1 }
        },
        .Err(_) => { return 2 }
    }
    0
}
