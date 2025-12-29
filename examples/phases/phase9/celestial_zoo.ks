// The Celestial Menagerie
// Showcases: Protocols, nested pattern matching, or-patterns, and @-bindings.

module CelestialZoo

protocol Celestial {
    func luminosity() -> Int
    func drift() -> Bool
}

struct Star : Celestial {
    let mass: Int
    func luminosity() -> Int { self.mass * 100 }
    func drift() -> Bool { false }
}

struct Comet : Celestial {
    var speed: Int
    func luminosity() -> Int { 10 }
    func drift() -> Bool { true }
}

enum Creature {
    case VoidWhale(age: Int, belly: [String])
    case NebulaCat(colors: [String])
    case SolarPhoenix(heat: Int)
}

struct Sanctuary[C] {
    var resident: C
    var name: String
}

func inspect(creature: Creature) -> String {
    match creature {
        .VoidWhale(age, belly: ["Space Plankton", ..]) if age > 1000 => {
            "An ancient whale full of plankton."
        },
        whale @ .VoidWhale(age: _, belly: _) => {
            // whale @ binding used here
            "A mysterious whale of unknown contents."
        },
        .NebulaCat(colors: ["Purple" or "Violet", "Pink", ..]) => {
            "A very fashionable space cat."
        },
        .SolarPhoenix(heat: h) if h > 9000 => {
            "IT'S OVER NINE THOUSAND!"
        },
        _ => "Just another celestial wanderer."
    }
}

func setupZoo() -> Int {
    let star = Star(mass: 10);
    let cat = Creature.NebulaCat(colors: ["Purple", "Pink", "Blue"]);
    
    let msg = inspect(creature: cat);
    star.luminosity()
}

