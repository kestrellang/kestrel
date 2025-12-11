// Robot pets with programmable personalities

module RobotPets

protocol Pet {
    func speak() -> String
    func happiness() -> Int
}

struct RoboDog : Pet {
    let name: String
    var batteryLevel: Int
    var tailWags: Int

    init(name: String) {
        self.name = name;
        self.batteryLevel = 100;
        self.tailWags = 0;
    }

    func speak() -> String { "Boop bork!" }

    func happiness() -> Int { self.tailWags * 10 }

    mutating func pet() {
        self.tailWags = self.tailWags + 1;
        self.batteryLevel = self.batteryLevel - 1;
    }
}

struct RoboCat : Pet {
    var mood: Int

    init() { self.mood = 50; }

    func speak() -> String { "Mrrp-beep" }

    func happiness() -> Int { self.mood }

    mutating func ignore() {
        if self.mood < 100 {
            self.mood = self.mood + 5;
        }
    }
}

struct PetHotel[P] {
    var guest: P
}

func adoptPet() -> RoboDog {
    var dog = RoboDog(name: "Sparky-3000");
    dog.pet();
    dog.pet();
    dog
}
