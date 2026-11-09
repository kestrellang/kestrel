// Rosetta Code: 100 doors
// https://rosettacode.org/wiki/100_doors
module Main

@main
func main() {
    var open = Array[Bool](repeating: false, count: 101);
    for pass in 1..=100 {
        var door = pass;
        while door <= 100 {
            open(door) = not open(door);
            door = door + pass;
        }
    }
    let openDoors = (1..=100).iter().filter { open(it) == true }.collect();
    println("Open doors: \(openDoors)");
}
