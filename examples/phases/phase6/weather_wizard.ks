// A weather wizard who controls the skies

module WeatherWizard

protocol Spell {
    func cast() -> String
    func manaCost() -> Int
}

struct RainSpell : Spell {
    let intensity: Int

    func cast() -> String { "Pitter patter!" }
    func manaCost() -> Int { self.intensity * 2 }
}

struct SunshineSpell : Spell {
    let warmth: Int

    func cast() -> String { "Let there be light!" }
    func manaCost() -> Int { self.warmth }
}

struct Wizard {
    let name: String
    var mana: Int
    var spellsCast: Int

    init(name: String) {
        self.name = name;
        self.mana = 100;
        self.spellsCast = 0;
    }

    mutating func rest() {
        self.mana = self.mana + 25;
        if self.mana > 100 {
            self.mana = 100;
        }
    }

    mutating func castRain(power power: Int) -> Bool {
        let spell = RainSpell(intensity: power);
        if self.mana >= spell.manaCost() {
            self.mana = self.mana - spell.manaCost();
            self.spellsCast = self.spellsCast + 1;
            true
        } else {
            false
        }
    }

    func isTired() -> Bool { self.mana < 20 }
}

struct SpellBook[S] {
    var pages: [S]
}

func summonStorm() -> Int {
    var merlin = Wizard(name: "Merlin");
    var storms = 0;
    while not merlin.isTired() {
        let success = merlin.castRain(power: 15);
        if success { storms = storms + 1; }
    }
    storms
}
