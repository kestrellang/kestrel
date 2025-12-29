// The Chrono-Cutter's Workshop
// Showcases: Enums with associated values, structs, and complex pattern matching with guards.

module TemporalBlacksmith

enum TemporalState {
    case Pristine
    case Weathered(centuries: Int)
    case Echoing(fragments: Int)
    case Paradoxical(warning: String)
}

struct ChronoSword {
    let name: String
    var state: TemporalState
    var power: Int

    init(name: String, power: Int) {
        self.name = name;
        self.state = .Pristine;
        self.power = power;
    }

    mutating func age(by years: Int) {
        match self.state {
            .Pristine => {
                self.state = .Weathered(centuries: years / 100);
            },
            .Weathered(centuries: c) => {
                self.state = .Weathered(centuries: c + (years / 100));
            },
            _ => {
                // Paradoxical items don't age normally
            }
        }
    }

    func describe() -> String {
        match self.state {
            .Pristine => "A shining blade from the future.",
            .Weathered(centuries: c) if c > 10 => "A rusted relic, centuries old.",
            .Weathered(centuries: _) => "A slightly worn blade.",
            .Echoing(fragments: f) => "A blade that exists in {f} places at once.",
            .Paradoxical(warning: w) => "A dangerous rift: {w}",
        }
    }
}

struct Anvil {
    var instability: Int

    mutating func forge(item: ChronoSword) -> TemporalState {
        if self.instability > 50 {
            .Paradoxical(warning: "The fabric of reality is tearing!")
        } else {
            .Echoing(fragments: item.power / 10)
        }
    }
}

func main() {
    var sword = ChronoSword(name: "Time-Eater", power: 120);
    var anvil = Anvil(instability: 20);
    
    sword.age(by: 500);
    let newState = anvil.forge(item: sword);
    sword.state = newState;
    
    let description = sword.describe();
}

