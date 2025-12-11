// A miniature kingdom with feudal economics

module TinyKingdom

struct Villager {
    let name: String
    var coins: Int
    var happiness: Int

    init(name: String) {
        self.name = name;
        self.coins = 10;
        self.happiness = 50;
    }

    mutating func payTax(amount: Int) -> Bool {
        if self.coins >= amount {
            self.coins = self.coins - amount;
            self.happiness = self.happiness - 5;
            true
        } else {
            self.happiness = self.happiness - 20;
            false
        }
    }

    mutating func findCoin() {
        self.coins = self.coins + 1;
        self.happiness = self.happiness + 10;
    }
}

struct Castle {
    var treasury: Int
    var walls: Int

    init() {
        self.treasury = 0;
        self.walls = 100;
    }

    mutating func collectFrom(villager: Villager) -> Int {
        let tax = villager.coins / 10;
        self.treasury = self.treasury + tax;
        tax
    }

    func isStanding() -> Bool { self.walls > 0 }
}

struct Kingdom[V] {
    var ruler: String
    var population: [V]
}

func foundKingdom() -> Castle {
    var castle = Castle();
    var bob = Villager(name: "Bob");
    bob.findCoin();
    bob.findCoin();
    castle
}
