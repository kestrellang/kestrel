// The Vault of Eternal Silence
// Showcases: Non-copyable structs (`not Copyable`), implicit moves, and pattern matching on resource states.

module VoidVault

// A unique artifact that cannot be duplicated
struct Artifact : not Copyable {
    let name: String
    let powerLevel: Int
}

enum VaultSlot {
    case Occupied(Artifact)
    case Empty
    case Collapsed(reason: String)
}

struct Vault {
    var slot: VaultSlot

    mutating func deposit(item: Artifact) {
        match self.slot {
            .Empty => {
                // item is implicitly moved here
                self.slot = .Occupied(item);
            },
            .Occupied(existing) => {
                self.slot = .Collapsed(reason: "Tried to double-stuff the vault with {item.name}");
                // both existing and item would be dropped here
            },
            _ => {}
        }
    }

    mutating func withdraw() -> VaultSlot {
        let current = self.slot;
        self.slot = .Empty;
        current // Move the slot contents out
    }
}

func inspect(slot: VaultSlot) -> String {
    match slot {
        .Occupied(item) => "The vault holds the {item.name}, glowing with power {item.powerLevel}.",
        .Empty => "The vault is hauntingly empty.",
        .Collapsed(reason) => "The vault has imploded: {reason}",
    }
}

func print(msg: String) {
    // Dummy print function
}

func main() {
    var myVault = Vault(slot: .Empty);
    let crown = Artifact(name: "Crown of the Void", powerLevel: 9001);

    // crown is moved into the vault
    myVault.deposit(item: crown);

    // let illegalCopy = crown; // Error: use of moved value

    let retrieved = myVault.withdraw();
    let report = inspect(slot: retrieved);
    print(msg: report);
}

