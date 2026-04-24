// test: diagnostics
// stdlib: false

module Test

struct Config {
    var debug: lang.i1
    var verbose: lang.i1

    init(debug: lang.i1, verbose: lang.i1) {
        self.debug = debug;
        self.verbose = verbose
    }

    init(debug: lang.i1) {
        self.init(debug, false);
        if debug {
            self.verbose = true
        }
    }
}
