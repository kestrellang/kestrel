// The Galactic Philharmonic
// Showcases: Higher-order functions, trailing closures, implicit `it` parameter, and pipeline-like composition.

module GalacticOrchestra

// Extern C bindings for printing
@extern(.C, mangleName: "kestrel_print_string")
func c_print_string(ptr: lang.ptr[I8], len: Int) -> Int

@extern(.C, mangleName: "kestrel_print_newline")
func c_print_newline(dummy: Int) -> Int

func printLine(s: String) {
    let _ = c_print_string(s.unsafePtr(), s.length());
    let _ = c_print_newline(0);
}

struct Note {
    let frequency: Float
    let duration: Float
}

protocol Instrument {
    func play(note note: Note) -> String
}

struct StarFlute : Instrument {
    func play(note note: Note) -> String {
        "A shimmering whistle from the StarFlute"
    }
}

struct NebulaCello : Instrument {
    func play(note note: Note) -> String {
        "A deep cosmic resonance from the NebulaCello"
    }
}

func conductor(perform: (String) -> String) -> String {
    let base = "The performance begins: ";
    perform(base)
}

func main() -> Int {
    let flute = StarFlute();
    let cello = NebulaCello();

    // Trailing closure with implicit `it`
    let symphony = conductor {
        it + "The stars are singing!"
    };
    printLine(symphony);

    // Nested closures and "pipelining" via function calls
    let finale = conductor { (intro) in
        intro + " and the void echoes back."
    };
    printLine(finale);
    
    0
}
