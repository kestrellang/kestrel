// Interstellar navigation for tiny astronauts

module SpaceAdventure

struct Star {
    let brightness: Int
    let distance: Float
}

struct Spaceship {
    var fuel: Int
    var x: Int
    var y: Int

    init(startingFuel: Int) {
        self.fuel = startingFuel;
        self.x = 0;
        self.y = 0;
    }

    mutating func flyTo(destX destX: Int, destY destY: Int) -> Bool {
        let cost = abs(destX - self.x) + abs(destY - self.y);
        if cost <= self.fuel {
            self.x = destX;
            self.y = destY;
            self.fuel = self.fuel - cost;
            true
        } else {
            false
        }
    }

    func canReach(star: Star) -> Bool {
        self.fuel > 0
    }
}

func abs(n: Int) -> Int {
    if n < 0 { 0 - n } else { n }
}

func launchMission() -> (Int, Int) {
    var ship = Spaceship(startingFuel: 100);
    let reached = ship.flyTo(destX: 10, destY: 20);
    (ship.x, ship.y)
}
