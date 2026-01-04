// The Galactic Philharmonic
// Showcases: Higher-order functions, trailing closures, implicit `it` parameter, and pipeline-like composition.

module GalacticOrchestra

struct Note {
    let frequency: Float
    let duration: Float
}

protocol Instrument {
    func play(note: Note) -> String
}

struct StarFlute : Instrument {
    func play(note: Note) -> String {
        "A shimmering whistle at {note.frequency}Hz"
    }
}

struct NebulaCello : Instrument {
    func play(note: Note) -> String {
        "A deep cosmic resonance at {note.frequency}Hz"
    }
}

func conductor(perform: (String) -> String) -> String {
    let base = "The performance begins: ";
    perform(base)
}

func compose(notes: [Note], transform: (Note) -> String) -> [String] {
    // In a real implementation, we'd iterate and map
    // For this example, we'll simulate the behavior
    let n1 = Note(frequency: 440.0, duration: 1.0);
    let n2 = Note(frequency: 880.0, duration: 0.5);
    
    [transform(n1), transform(n2)]
}

func print(msg: String) {
    // Dummy print function
}

func main() {
    let flute = StarFlute();
    let cello = NebulaCello();

    // Trailing closure with implicit `it`
    let symphony = conductor {
        it + "The stars are singing!"
    };
    print(msg: symphony);

    // Explicit parameters in closure
    let arrangement = compose(notes: []) { note in
        if note.frequency > 500.0 {
            flute.play(note: note)
        } else {
            cello.play(note: note)
        }
    };

    // Nested closures and "pipelining" via function calls
    let finale = conductor { intro in
        let layers = ["Stardust", "Solar Wind", "Void"];
        // In reality, we'd use a real map here
        intro + " and the void echoes back."
    };
    print(msg: finale);
}

