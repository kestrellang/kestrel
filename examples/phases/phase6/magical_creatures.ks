// A magical menagerie with type-safe creature breeding

module MagicalCreatures

// Creatures can be compared by power level
protocol Powerful {
    func powerLevel() -> Int
}

// Some creatures can be combined to create hybrids
protocol Combinable {
    func essence() -> String
}

// A magical container that only holds powerful things
struct Sanctuary[C: Powerful] {
    var resident: C
    var magicAura: Int

    func isLegendary() -> Bool {
        self.resident.powerLevel() > 9000
    }
}

// Dragons are powerful fire-breathers
struct Dragon : Powerful, Combinable {
    let name: String
    let fireBreath: Int
    let wingSpan: Int

    func powerLevel() -> Int {
        self.fireBreath * self.wingSpan
    }

    func essence() -> String { "fire" }

    func roar() -> String {
        if self.powerLevel() > 100 {
            "ROOOAAAAR!"
        } else {
            "rawr"
        }
    }
}

// Phoenix rises from ashes
struct Phoenix : Powerful, Combinable {
    var rebornCount: Int
    let brightness: Int

    func powerLevel() -> Int {
        self.brightness * (self.rebornCount + 1)
    }

    func essence() -> String { "rebirth" }

    mutating func rise() {
        self.rebornCount = self.rebornCount + 1;
    }
}

// Unicorns are majestic and pure
struct Unicorn : Powerful {
    let hornLength: Int
    let sparkleLevel: Int

    func powerLevel() -> Int {
        self.hornLength + self.sparkleLevel * 10
    }

    func blessing() -> Int {
        self.sparkleLevel * 7
    }
}

// A breeding pair requires both creatures to be combinable
struct BreedingPair[A: Combinable, B: Combinable] {
    let parent1: A
    let parent2: B

    func combinedEssence() -> (String, String) {
        (self.parent1.essence(), self.parent2.essence())
    }
}

// Hybrid creature born from two combinable parents
struct Hybrid[P1: Powerful + Combinable, P2: Powerful + Combinable] {
    let origin1: P1
    let origin2: P2
    let uniqueTrait: String

    func inheritedPower() -> Int {
        let base = self.origin1.powerLevel() + self.origin2.powerLevel();
        base + base / 4
    }
}

// Transform any powerful creature into sanctuary guardian
func enthrone[C: Powerful](creature: C) -> Sanctuary[C] {
    Sanctuary[C](resident: creature, magicAura: creature.powerLevel() * 2)
}

// Breed two combinable creatures
func breed[A: Powerful + Combinable, B: Powerful + Combinable](
    first: A,
    second: B,
    trait: String
) -> Hybrid[A, B] {
    Hybrid[A, B](origin1: first, origin2: second, uniqueTrait: trait)
}

// Find the mightiest among two powerful creatures
func mightier[T: Powerful](a: T, b: T) -> T {
    if a.powerLevel() > b.powerLevel() { a } else { b }
}

func summonCreatures() -> Int {
    let smaug = Dragon(name: "Smaug", fireBreath: 95, wingSpan: 120);
    var fawkes = Phoenix(rebornCount: 3, brightness: 80);
    let starlight = Unicorn(hornLength: 30, sparkleLevel: 50);

    // Dragons and Phoenix can breed (both Combinable)
    let hybrid = breed[Dragon, Phoenix](
        first: smaug,
        second: fawkes,
        trait: "flaming wings"
    );

    // Any powerful creature can be enthroned
    let dragonSanctuary = enthrone[Dragon](creature: smaug);
    let unicornSanctuary = enthrone[Unicorn](creature: starlight);

    // Phoenix rises again
    fawkes.rise();

    // Compare creatures of same type
    let dragon2 = Dragon(name: "Ancalagon", fireBreath: 200, wingSpan: 300);
    let strongest = mightier[Dragon](a: smaug, b: dragon2);

    hybrid.inheritedPower() + strongest.powerLevel()
}
