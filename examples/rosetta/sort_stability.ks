// Rosetta Code: Sort stability
// https://rosettacode.org/wiki/Sort_stability
module Main

@main
func main() {
    let cities = [("UK", "London"), ("US", "New York"), ("US", "Birmingham"), ("UK", "Birmingham")];
    // Array.sort(byKey:) is stable: equal keys keep their original order.
    var byCountry = cities;
    byCountry.sort(byKey: { it.0 });
    for (country, city) in byCountry { println("\(country) \(city)"); }
    println();
    var byCity = cities;
    byCity.sort(byKey: { it.1 });
    for (country, city) in byCity { println("\(country) \(city)"); }
}
