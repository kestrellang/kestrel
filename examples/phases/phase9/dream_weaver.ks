// The Dream-Weaver's Garden
// Showcases: Recursive enums, generics, and trailing closure syntax.

module DreamWeaver

indirect enum Dream[T] {
    case Spark(idea: T)
    case Flow(current: T, next: Dream[T])
    case Nightmare(terror: String)
}

struct Garden[D] {
    var dreams: [Dream[D]]
    var lucidity: Int

    init() {
        self.dreams = [];
        self.lucidity = 100;
    }

    mutating func plant(dream: Dream[D]) {
        self.dreams.append(dream);
    }

    func harvest(processor: (Dream[D]) -> Int) -> Int {
        var total = 0;
        // In a real implementation, we would iterate here
        // For now, let's just process the first dream if it exists
        if let [first, ..] = self.dreams {
            total = processor(first);
        }
        total
    }
}

func cultivate() -> Int {
    var myGarden = Garden[String]();
    
    let sweetDream = Dream.Flow(
        current: "Flying over a city of gold",
        next: .Spark(idea: "A key made of starlight")
    );
    
    myGarden.plant(dream: sweetDream);
    
    // Using trailing closure syntax
    myGarden.harvest { dream in
        match dream {
            .Spark(_) => 10,
            .Flow(_, next: _) => 25,
            .Nightmare(terror: t) if t == "Spiders" => -50,
            .Nightmare(_) => -10
        }
    }
}

