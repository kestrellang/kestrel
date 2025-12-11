// A tiny potion shop with magical brews

module PotionShop

protocol Drinkable {
    func drink() -> String
}

struct Potion : Drinkable {
    let name: String
    let potency: Int

    func drink() -> String {
        if self.potency > 50 {
            "Woah! Powerful stuff!"
        } else {
            "A mild tingle..."
        }
    }
}

struct Cauldron[T] {
    var contents: [T]
    var bubbling: Bool

    init() {
        self.contents = [];
        self.bubbling = false;
    }

    mutating func addIngredient(item: T) {
        self.bubbling = true;
    }

    mutating func stir(times: Int) {
        var i = 0;
        while i < times {
            self.bubbling = not self.bubbling;
            i = i + 1;
        }
    }
}

func brew(strength: Int) -> Potion {
    Potion(name: "Mystery Elixir", potency: strength * 10)
}
