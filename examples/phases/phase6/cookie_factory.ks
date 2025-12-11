// An automated cookie production line

module CookieFactory

protocol Bakeable {
    func bakeTime() -> Int
    func isGoldenBrown(minutes: Int) -> Bool
}

struct Cookie : Bakeable {
    let flavor: String
    let chips: Int

    func bakeTime() -> Int { 12 }

    func isGoldenBrown(minutes: Int) -> Bool {
        minutes >= self.bakeTime()
    }

    func deliciousness() -> Int {
        self.chips * 7
    }
}

struct Oven {
    var temperature: Int
    var timer: Int

    init() {
        self.temperature = 0;
        self.timer = 0;
    }

    mutating func preheat(degrees degrees: Int) {
        self.temperature = degrees;
    }

    mutating func tick() -> Bool {
        if self.timer > 0 {
            self.timer = self.timer - 1;
            self.timer == 0
        } else {
            false
        }
    }

    mutating func startBaking(minutes minutes: Int) {
        self.timer = minutes;
    }
}

struct ConveyorBelt[Item] {
    var items: [Item]
    var speed: Int

    init(speed: Int) {
        self.items = [];
        self.speed = speed;
    }
}

func bakeBatch() -> Int {
    var oven = Oven();
    oven.preheat(degrees: 350);
    let cookie = Cookie(flavor: "chocolate", chips: 20);
    oven.startBaking(minutes: cookie.bakeTime());
    cookie.deliciousness()
}
