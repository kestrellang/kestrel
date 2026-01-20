// The Alchemist's Shadow-Lab
// Showcases: Complex protocols, generic reactions, and closure-based transmutation.

module AlchemyLab

// Extern C bindings for printing
@extern(.C, mangleName: "kestrel_print_string")
func c_print_string(ptr: lang.ptr[I8], len: Int) -> Int

@extern(.C, mangleName: "kestrel_print_newline")
func c_print_newline(dummy: Int) -> Int

func printLine(s: String) {
    let _ = c_print_string(s.unsafePtr(), s.length());
    let _ = c_print_newline(0);
}

protocol Reagent {
    func essence() -> String
}

struct Mercury : Reagent {
    func essence() -> String { "Fluid Silver" }
}

struct Sulfur : Reagent {
    func essence() -> String { "Burning Stone" }
}

enum Potion {
    case Failed(residue: String)
    case Success(name: String, potency: Int)
}

struct Alchemist[A, B] where A: Reagent, B: Reagent {
    let base: A
    let additive: B

    func transmute(formula: (A, B) -> Potion) -> Potion {
        formula(self.base, self.additive)
    }
}

func main() -> Int {
    let mercury = Mercury();
    let sulfur = Sulfur();

    let lab = Alchemist(base: mercury, additive: sulfur);

    // Using trailing closure for the transmutation formula
    let result = lab.transmute { (a, b) in 
        match (a.essence(), b.essence()) {
            ("Fluid Silver", "Burning Stone") => .Success(name: "Philosopher's Dew", potency: 100),
            _ => .Failed(residue: "A puddle of sad grey slime")
        }
    };

    let description = match result {
        .Success(name, potency) if potency > 50 => {
            "Gaze upon the potion! It is truly potent."
        },
        .Success(name, _) => "Created a potion, but it's a bit weak.",
        .Failed(msg) => "The lab smells terrible."
    };

    printLine(description);
    0
}
