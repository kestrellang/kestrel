// The Vault of Eternal Silence
// Showcases: Non-copyable structs (`not Copyable`), implicit moves, and pattern matching on resource states.

module VoidVault

// Extern C bindings for printing
@extern(.C, mangleName: "kestrel_print_string")
func c_print_string(ptr: lang.ptr[I8], len: Int) -> Int

@extern(.C, mangleName: "kestrel_print_int")
func c_print_int(value: Int) -> Int

@extern(.C, mangleName: "kestrel_print_newline")
func c_print_newline(dummy: Int) -> Int

func printLine(s: String) {
    let _ = c_print_string(s.unsafePtr(), s.length());
    let _ = c_print_newline(0);
}

func printInt(i: Int) {
    let _ = c_print_int(i);
}

@builtin(.Copyable)
protocol Copyable {}

// A unique artifact that cannot be duplicated
struct Artifact : not Copyable {
    let name: String
    let powerLevel: Int
}

enum VaultSlot {
    case Occupied(artifact: Artifact)
    case Empty
    case Collapsed(reason: String)
}

struct Vault {
    var slot: VaultSlot

    mutating func deposit(item: Artifact) {
        match self.slot {
            .Empty => {
                // item is implicitly moved here
                self.slot = .Occupied(artifact: item);
            },
            .Occupied(existing) => {
                self.slot = .Collapsed(reason: "Tried to double-stuff the vault");
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
        .Occupied(item) => "The vault holds a powerful artifact!",
        .Empty => "The vault is hauntingly empty.",
        .Collapsed(reason) => "The vault has imploded!",
    }
}

func main() -> Int {
    printLine("=== The Vault of Eternal Silence ===");
    
    var myVault = Vault(slot: .Empty);
    printLine("Created an empty vault");
    
    let crown = Artifact(name: "Crown of the Void", powerLevel: 9001);
    printLine("Created artifact: Crown of the Void");
    printLine("Power level:");
    printInt(crown.powerLevel);
    let _ = c_print_newline(0);

    // crown is moved into the vault
    printLine("Depositing artifact into vault...");
    myVault.deposit(crown);

    // let illegalCopy = crown; // Error: use of moved value

    printLine("Withdrawing from vault...");
    let retrieved = myVault.withdraw();
    let report = inspect(retrieved);
    printLine(report);
    
    printLine("Checking vault again...");
    let empty = myVault.withdraw();
    let report2 = inspect(empty);
    printLine(report2);
    
    0
}
