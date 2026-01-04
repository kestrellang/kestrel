// The Alchemist's Shadow-Lab
// Showcases: Complex protocols, generic reactions, and closure-based transmutation.

module AlchemyLab

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

struct Alchemist[A : Reagent, B : Reagent] {
    let base: A
    let additive: B

    func transmute(formula: (A, B) -> Potion) -> Potion {
        formula(self.base, self.additive)
    }
}

func main() {
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
            "Gaze upon {name}! It is truly potent."
        },
        .Success(name, _) => "Created {name}, but it's a bit weak.",
        .Failed(msg) => "The lab smells like {msg}."
    };

    print(msg: description);
}

func print(msg: String) {
    // Dummy print function
}

