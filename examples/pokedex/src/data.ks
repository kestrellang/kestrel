// Data layer: Kanto pokedex list, JSON helpers, type theming

module pokedex.data

import quill.value.(Value)

// ============================================================================
// JSON HELPERS
// ============================================================================

public func getFloat(v: Value) -> Float64 {
    match v.asFloat() {
        .Some(f) => f,
        .None => {
            match v.asInt() {
                .Some(n) => Float64(from: n),
                .None => 0.0
            }
        }
    }
}

public func getString(v: Value) -> String {
    match v.asString() {
        .Some(s) => s,
        .None => ""
    }
}

public func getInt(v: Value) -> Int64 {
    match v.asInt() {
        .Some(n) => n,
        .None => {
            match v.asFloat() {
                .Some(f) => {
                    match f.toInt64() {
                        .Some(n) => n,
                        .None => 0
                    }
                },
                .None => 0
            }
        }
    }
}

public func getField(obj: Value, key: String) -> Value {
    match obj.value(forKey: key) {
        .Some(v) => v,
        .None => Value.Null
    }
}

public func getArrayField(obj: Value, key: String) -> Array[Value] {
    match obj.value(forKey: key) {
        .Some(v) => {
            match v.asArray() {
                .Some(arr) => arr,
                .None => Array[Value]()
            }
        },
        .None => Array[Value]()
    }
}


// ============================================================================
// KANTO POKEDEX (Gen 1, 151 entries) — single source of truth
// ============================================================================

public struct PokemonEntry {
    public var id: Int64
    public var apiName: String
    public var displayName: String
    public var primaryType: String
}

/// Build the full Kanto pokedex. All 151 entries inline so id/apiName/
/// displayName/primaryType all live in one place.
public func kantoPokedex() -> Array[PokemonEntry] {
    var p = Array[PokemonEntry]();
    p.append(PokemonEntry(id: 1, apiName: "bulbasaur", displayName: "Bulbasaur", primaryType: "grass"));
    p.append(PokemonEntry(id: 2, apiName: "ivysaur", displayName: "Ivysaur", primaryType: "grass"));
    p.append(PokemonEntry(id: 3, apiName: "venusaur", displayName: "Venusaur", primaryType: "grass"));
    p.append(PokemonEntry(id: 4, apiName: "charmander", displayName: "Charmander", primaryType: "fire"));
    p.append(PokemonEntry(id: 5, apiName: "charmeleon", displayName: "Charmeleon", primaryType: "fire"));
    p.append(PokemonEntry(id: 6, apiName: "charizard", displayName: "Charizard", primaryType: "fire"));
    p.append(PokemonEntry(id: 7, apiName: "squirtle", displayName: "Squirtle", primaryType: "water"));
    p.append(PokemonEntry(id: 8, apiName: "wartortle", displayName: "Wartortle", primaryType: "water"));
    p.append(PokemonEntry(id: 9, apiName: "blastoise", displayName: "Blastoise", primaryType: "water"));
    p.append(PokemonEntry(id: 10, apiName: "caterpie", displayName: "Caterpie", primaryType: "bug"));
    p.append(PokemonEntry(id: 11, apiName: "metapod", displayName: "Metapod", primaryType: "bug"));
    p.append(PokemonEntry(id: 12, apiName: "butterfree", displayName: "Butterfree", primaryType: "bug"));
    p.append(PokemonEntry(id: 13, apiName: "weedle", displayName: "Weedle", primaryType: "bug"));
    p.append(PokemonEntry(id: 14, apiName: "kakuna", displayName: "Kakuna", primaryType: "bug"));
    p.append(PokemonEntry(id: 15, apiName: "beedrill", displayName: "Beedrill", primaryType: "bug"));
    p.append(PokemonEntry(id: 16, apiName: "pidgey", displayName: "Pidgey", primaryType: "normal"));
    p.append(PokemonEntry(id: 17, apiName: "pidgeotto", displayName: "Pidgeotto", primaryType: "normal"));
    p.append(PokemonEntry(id: 18, apiName: "pidgeot", displayName: "Pidgeot", primaryType: "normal"));
    p.append(PokemonEntry(id: 19, apiName: "rattata", displayName: "Rattata", primaryType: "normal"));
    p.append(PokemonEntry(id: 20, apiName: "raticate", displayName: "Raticate", primaryType: "normal"));
    p.append(PokemonEntry(id: 21, apiName: "spearow", displayName: "Spearow", primaryType: "normal"));
    p.append(PokemonEntry(id: 22, apiName: "fearow", displayName: "Fearow", primaryType: "normal"));
    p.append(PokemonEntry(id: 23, apiName: "ekans", displayName: "Ekans", primaryType: "poison"));
    p.append(PokemonEntry(id: 24, apiName: "arbok", displayName: "Arbok", primaryType: "poison"));
    p.append(PokemonEntry(id: 25, apiName: "pikachu", displayName: "Pikachu", primaryType: "electric"));
    p.append(PokemonEntry(id: 26, apiName: "raichu", displayName: "Raichu", primaryType: "electric"));
    p.append(PokemonEntry(id: 27, apiName: "sandshrew", displayName: "Sandshrew", primaryType: "ground"));
    p.append(PokemonEntry(id: 28, apiName: "sandslash", displayName: "Sandslash", primaryType: "ground"));
    p.append(PokemonEntry(id: 29, apiName: "nidoran-f", displayName: "Nidoran \u{2640}", primaryType: "poison"));
    p.append(PokemonEntry(id: 30, apiName: "nidorina", displayName: "Nidorina", primaryType: "poison"));
    p.append(PokemonEntry(id: 31, apiName: "nidoqueen", displayName: "Nidoqueen", primaryType: "poison"));
    p.append(PokemonEntry(id: 32, apiName: "nidoran-m", displayName: "Nidoran \u{2642}", primaryType: "poison"));
    p.append(PokemonEntry(id: 33, apiName: "nidorino", displayName: "Nidorino", primaryType: "poison"));
    p.append(PokemonEntry(id: 34, apiName: "nidoking", displayName: "Nidoking", primaryType: "poison"));
    p.append(PokemonEntry(id: 35, apiName: "clefairy", displayName: "Clefairy", primaryType: "fairy"));
    p.append(PokemonEntry(id: 36, apiName: "clefable", displayName: "Clefable", primaryType: "fairy"));
    p.append(PokemonEntry(id: 37, apiName: "vulpix", displayName: "Vulpix", primaryType: "fire"));
    p.append(PokemonEntry(id: 38, apiName: "ninetales", displayName: "Ninetales", primaryType: "fire"));
    p.append(PokemonEntry(id: 39, apiName: "jigglypuff", displayName: "Jigglypuff", primaryType: "normal"));
    p.append(PokemonEntry(id: 40, apiName: "wigglytuff", displayName: "Wigglytuff", primaryType: "normal"));
    p.append(PokemonEntry(id: 41, apiName: "zubat", displayName: "Zubat", primaryType: "poison"));
    p.append(PokemonEntry(id: 42, apiName: "golbat", displayName: "Golbat", primaryType: "poison"));
    p.append(PokemonEntry(id: 43, apiName: "oddish", displayName: "Oddish", primaryType: "grass"));
    p.append(PokemonEntry(id: 44, apiName: "gloom", displayName: "Gloom", primaryType: "grass"));
    p.append(PokemonEntry(id: 45, apiName: "vileplume", displayName: "Vileplume", primaryType: "grass"));
    p.append(PokemonEntry(id: 46, apiName: "paras", displayName: "Paras", primaryType: "bug"));
    p.append(PokemonEntry(id: 47, apiName: "parasect", displayName: "Parasect", primaryType: "bug"));
    p.append(PokemonEntry(id: 48, apiName: "venonat", displayName: "Venonat", primaryType: "bug"));
    p.append(PokemonEntry(id: 49, apiName: "venomoth", displayName: "Venomoth", primaryType: "bug"));
    p.append(PokemonEntry(id: 50, apiName: "diglett", displayName: "Diglett", primaryType: "ground"));
    p.append(PokemonEntry(id: 51, apiName: "dugtrio", displayName: "Dugtrio", primaryType: "ground"));
    p.append(PokemonEntry(id: 52, apiName: "meowth", displayName: "Meowth", primaryType: "normal"));
    p.append(PokemonEntry(id: 53, apiName: "persian", displayName: "Persian", primaryType: "normal"));
    p.append(PokemonEntry(id: 54, apiName: "psyduck", displayName: "Psyduck", primaryType: "water"));
    p.append(PokemonEntry(id: 55, apiName: "golduck", displayName: "Golduck", primaryType: "water"));
    p.append(PokemonEntry(id: 56, apiName: "mankey", displayName: "Mankey", primaryType: "fighting"));
    p.append(PokemonEntry(id: 57, apiName: "primeape", displayName: "Primeape", primaryType: "fighting"));
    p.append(PokemonEntry(id: 58, apiName: "growlithe", displayName: "Growlithe", primaryType: "fire"));
    p.append(PokemonEntry(id: 59, apiName: "arcanine", displayName: "Arcanine", primaryType: "fire"));
    p.append(PokemonEntry(id: 60, apiName: "poliwag", displayName: "Poliwag", primaryType: "water"));
    p.append(PokemonEntry(id: 61, apiName: "poliwhirl", displayName: "Poliwhirl", primaryType: "water"));
    p.append(PokemonEntry(id: 62, apiName: "poliwrath", displayName: "Poliwrath", primaryType: "water"));
    p.append(PokemonEntry(id: 63, apiName: "abra", displayName: "Abra", primaryType: "psychic"));
    p.append(PokemonEntry(id: 64, apiName: "kadabra", displayName: "Kadabra", primaryType: "psychic"));
    p.append(PokemonEntry(id: 65, apiName: "alakazam", displayName: "Alakazam", primaryType: "psychic"));
    p.append(PokemonEntry(id: 66, apiName: "machop", displayName: "Machop", primaryType: "fighting"));
    p.append(PokemonEntry(id: 67, apiName: "machoke", displayName: "Machoke", primaryType: "fighting"));
    p.append(PokemonEntry(id: 68, apiName: "machamp", displayName: "Machamp", primaryType: "fighting"));
    p.append(PokemonEntry(id: 69, apiName: "bellsprout", displayName: "Bellsprout", primaryType: "grass"));
    p.append(PokemonEntry(id: 70, apiName: "weepinbell", displayName: "Weepinbell", primaryType: "grass"));
    p.append(PokemonEntry(id: 71, apiName: "victreebel", displayName: "Victreebel", primaryType: "grass"));
    p.append(PokemonEntry(id: 72, apiName: "tentacool", displayName: "Tentacool", primaryType: "water"));
    p.append(PokemonEntry(id: 73, apiName: "tentacruel", displayName: "Tentacruel", primaryType: "water"));
    p.append(PokemonEntry(id: 74, apiName: "geodude", displayName: "Geodude", primaryType: "rock"));
    p.append(PokemonEntry(id: 75, apiName: "graveler", displayName: "Graveler", primaryType: "rock"));
    p.append(PokemonEntry(id: 76, apiName: "golem", displayName: "Golem", primaryType: "rock"));
    p.append(PokemonEntry(id: 77, apiName: "ponyta", displayName: "Ponyta", primaryType: "fire"));
    p.append(PokemonEntry(id: 78, apiName: "rapidash", displayName: "Rapidash", primaryType: "fire"));
    p.append(PokemonEntry(id: 79, apiName: "slowpoke", displayName: "Slowpoke", primaryType: "water"));
    p.append(PokemonEntry(id: 80, apiName: "slowbro", displayName: "Slowbro", primaryType: "water"));
    p.append(PokemonEntry(id: 81, apiName: "magnemite", displayName: "Magnemite", primaryType: "electric"));
    p.append(PokemonEntry(id: 82, apiName: "magneton", displayName: "Magneton", primaryType: "electric"));
    p.append(PokemonEntry(id: 83, apiName: "farfetchd", displayName: "Farfetch'd", primaryType: "normal"));
    p.append(PokemonEntry(id: 84, apiName: "doduo", displayName: "Doduo", primaryType: "normal"));
    p.append(PokemonEntry(id: 85, apiName: "dodrio", displayName: "Dodrio", primaryType: "normal"));
    p.append(PokemonEntry(id: 86, apiName: "seel", displayName: "Seel", primaryType: "water"));
    p.append(PokemonEntry(id: 87, apiName: "dewgong", displayName: "Dewgong", primaryType: "water"));
    p.append(PokemonEntry(id: 88, apiName: "grimer", displayName: "Grimer", primaryType: "poison"));
    p.append(PokemonEntry(id: 89, apiName: "muk", displayName: "Muk", primaryType: "poison"));
    p.append(PokemonEntry(id: 90, apiName: "shellder", displayName: "Shellder", primaryType: "water"));
    p.append(PokemonEntry(id: 91, apiName: "cloyster", displayName: "Cloyster", primaryType: "water"));
    p.append(PokemonEntry(id: 92, apiName: "gastly", displayName: "Gastly", primaryType: "ghost"));
    p.append(PokemonEntry(id: 93, apiName: "haunter", displayName: "Haunter", primaryType: "ghost"));
    p.append(PokemonEntry(id: 94, apiName: "gengar", displayName: "Gengar", primaryType: "ghost"));
    p.append(PokemonEntry(id: 95, apiName: "onix", displayName: "Onix", primaryType: "rock"));
    p.append(PokemonEntry(id: 96, apiName: "drowzee", displayName: "Drowzee", primaryType: "psychic"));
    p.append(PokemonEntry(id: 97, apiName: "hypno", displayName: "Hypno", primaryType: "psychic"));
    p.append(PokemonEntry(id: 98, apiName: "krabby", displayName: "Krabby", primaryType: "water"));
    p.append(PokemonEntry(id: 99, apiName: "kingler", displayName: "Kingler", primaryType: "water"));
    p.append(PokemonEntry(id: 100, apiName: "voltorb", displayName: "Voltorb", primaryType: "electric"));
    p.append(PokemonEntry(id: 101, apiName: "electrode", displayName: "Electrode", primaryType: "electric"));
    p.append(PokemonEntry(id: 102, apiName: "exeggcute", displayName: "Exeggcute", primaryType: "grass"));
    p.append(PokemonEntry(id: 103, apiName: "exeggutor", displayName: "Exeggutor", primaryType: "grass"));
    p.append(PokemonEntry(id: 104, apiName: "cubone", displayName: "Cubone", primaryType: "ground"));
    p.append(PokemonEntry(id: 105, apiName: "marowak", displayName: "Marowak", primaryType: "ground"));
    p.append(PokemonEntry(id: 106, apiName: "hitmonlee", displayName: "Hitmonlee", primaryType: "fighting"));
    p.append(PokemonEntry(id: 107, apiName: "hitmonchan", displayName: "Hitmonchan", primaryType: "fighting"));
    p.append(PokemonEntry(id: 108, apiName: "lickitung", displayName: "Lickitung", primaryType: "normal"));
    p.append(PokemonEntry(id: 109, apiName: "koffing", displayName: "Koffing", primaryType: "poison"));
    p.append(PokemonEntry(id: 110, apiName: "weezing", displayName: "Weezing", primaryType: "poison"));
    p.append(PokemonEntry(id: 111, apiName: "rhyhorn", displayName: "Rhyhorn", primaryType: "ground"));
    p.append(PokemonEntry(id: 112, apiName: "rhydon", displayName: "Rhydon", primaryType: "ground"));
    p.append(PokemonEntry(id: 113, apiName: "chansey", displayName: "Chansey", primaryType: "normal"));
    p.append(PokemonEntry(id: 114, apiName: "tangela", displayName: "Tangela", primaryType: "grass"));
    p.append(PokemonEntry(id: 115, apiName: "kangaskhan", displayName: "Kangaskhan", primaryType: "normal"));
    p.append(PokemonEntry(id: 116, apiName: "horsea", displayName: "Horsea", primaryType: "water"));
    p.append(PokemonEntry(id: 117, apiName: "seadra", displayName: "Seadra", primaryType: "water"));
    p.append(PokemonEntry(id: 118, apiName: "goldeen", displayName: "Goldeen", primaryType: "water"));
    p.append(PokemonEntry(id: 119, apiName: "seaking", displayName: "Seaking", primaryType: "water"));
    p.append(PokemonEntry(id: 120, apiName: "staryu", displayName: "Staryu", primaryType: "water"));
    p.append(PokemonEntry(id: 121, apiName: "starmie", displayName: "Starmie", primaryType: "water"));
    p.append(PokemonEntry(id: 122, apiName: "mr-mime", displayName: "Mr. Mime", primaryType: "psychic"));
    p.append(PokemonEntry(id: 123, apiName: "scyther", displayName: "Scyther", primaryType: "bug"));
    p.append(PokemonEntry(id: 124, apiName: "jynx", displayName: "Jynx", primaryType: "ice"));
    p.append(PokemonEntry(id: 125, apiName: "electabuzz", displayName: "Electabuzz", primaryType: "electric"));
    p.append(PokemonEntry(id: 126, apiName: "magmar", displayName: "Magmar", primaryType: "fire"));
    p.append(PokemonEntry(id: 127, apiName: "pinsir", displayName: "Pinsir", primaryType: "bug"));
    p.append(PokemonEntry(id: 128, apiName: "tauros", displayName: "Tauros", primaryType: "normal"));
    p.append(PokemonEntry(id: 129, apiName: "magikarp", displayName: "Magikarp", primaryType: "water"));
    p.append(PokemonEntry(id: 130, apiName: "gyarados", displayName: "Gyarados", primaryType: "water"));
    p.append(PokemonEntry(id: 131, apiName: "lapras", displayName: "Lapras", primaryType: "water"));
    p.append(PokemonEntry(id: 132, apiName: "ditto", displayName: "Ditto", primaryType: "normal"));
    p.append(PokemonEntry(id: 133, apiName: "eevee", displayName: "Eevee", primaryType: "normal"));
    p.append(PokemonEntry(id: 134, apiName: "vaporeon", displayName: "Vaporeon", primaryType: "water"));
    p.append(PokemonEntry(id: 135, apiName: "jolteon", displayName: "Jolteon", primaryType: "electric"));
    p.append(PokemonEntry(id: 136, apiName: "flareon", displayName: "Flareon", primaryType: "fire"));
    p.append(PokemonEntry(id: 137, apiName: "porygon", displayName: "Porygon", primaryType: "normal"));
    p.append(PokemonEntry(id: 138, apiName: "omanyte", displayName: "Omanyte", primaryType: "rock"));
    p.append(PokemonEntry(id: 139, apiName: "omastar", displayName: "Omastar", primaryType: "rock"));
    p.append(PokemonEntry(id: 140, apiName: "kabuto", displayName: "Kabuto", primaryType: "rock"));
    p.append(PokemonEntry(id: 141, apiName: "kabutops", displayName: "Kabutops", primaryType: "rock"));
    p.append(PokemonEntry(id: 142, apiName: "aerodactyl", displayName: "Aerodactyl", primaryType: "rock"));
    p.append(PokemonEntry(id: 143, apiName: "snorlax", displayName: "Snorlax", primaryType: "normal"));
    p.append(PokemonEntry(id: 144, apiName: "articuno", displayName: "Articuno", primaryType: "ice"));
    p.append(PokemonEntry(id: 145, apiName: "zapdos", displayName: "Zapdos", primaryType: "electric"));
    p.append(PokemonEntry(id: 146, apiName: "moltres", displayName: "Moltres", primaryType: "fire"));
    p.append(PokemonEntry(id: 147, apiName: "dratini", displayName: "Dratini", primaryType: "dragon"));
    p.append(PokemonEntry(id: 148, apiName: "dragonair", displayName: "Dragonair", primaryType: "dragon"));
    p.append(PokemonEntry(id: 149, apiName: "dragonite", displayName: "Dragonite", primaryType: "dragon"));
    p.append(PokemonEntry(id: 150, apiName: "mewtwo", displayName: "Mewtwo", primaryType: "psychic"));
    p.append(PokemonEntry(id: 151, apiName: "mew", displayName: "Mew", primaryType: "psychic"));
    p
}

/// O(1) lookup by id (1..151). Returns a Normal-typed sentinel if out of range.
public func kantoEntryById(id: Int64) -> PokemonEntry {
    let all = kantoPokedex();
    if id >= 1 and id <= all.count {
        return all(unchecked: id - 1)
    }
    PokemonEntry(id: 0, apiName: "", displayName: "Unknown", primaryType: "normal")
}

/// All 18 type names in canonical order. Used to render the type-filter row.
public func allTypes() -> Array[String] {
    var t = Array[String]();
    t.append("normal");
    t.append("fire");
    t.append("water");
    t.append("electric");
    t.append("grass");
    t.append("ice");
    t.append("fighting");
    t.append("poison");
    t.append("ground");
    t.append("flying");
    t.append("psychic");
    t.append("bug");
    t.append("rock");
    t.append("ghost");
    t.append("dragon");
    t.append("dark");
    t.append("steel");
    t.append("fairy");
    t
}

// ============================================================================
// TYPE THEMING (background color, accent color, emoji)
// ============================================================================

public func typeColor(t: String) -> String {
    if t == "normal"   { return "#a8a878" }
    if t == "fire"     { return "#f08030" }
    if t == "water"    { return "#6890f0" }
    if t == "electric" { return "#f8d030" }
    if t == "grass"    { return "#78c850" }
    if t == "ice"      { return "#98d8d8" }
    if t == "fighting" { return "#c03028" }
    if t == "poison"   { return "#a040a0" }
    if t == "ground"   { return "#e0c068" }
    if t == "flying"   { return "#a890f0" }
    if t == "psychic"  { return "#f85888" }
    if t == "bug"      { return "#a8b820" }
    if t == "rock"     { return "#b8a038" }
    if t == "ghost"    { return "#705898" }
    if t == "dragon"   { return "#7038f8" }
    if t == "dark"     { return "#705848" }
    if t == "steel"    { return "#b8b8d0" }
    if t == "fairy"    { return "#ee99ac" }
    "#777"
}

/// A darker variant of the type color, used for card-frame gradients.
public func typeColorDark(t: String) -> String {
    if t == "normal"   { return "#6d6d4a" }
    if t == "fire"     { return "#a04a10" }
    if t == "water"    { return "#3858a8" }
    if t == "electric" { return "#a89018" }
    if t == "grass"    { return "#447a30" }
    if t == "ice"      { return "#5a8a8a" }
    if t == "fighting" { return "#7a1c18" }
    if t == "poison"   { return "#5e2660" }
    if t == "ground"   { return "#90703c" }
    if t == "flying"   { return "#5a4a90" }
    if t == "psychic"  { return "#a82a50" }
    if t == "bug"      { return "#5e6810" }
    if t == "rock"     { return "#705820" }
    if t == "ghost"    { return "#3d2a58" }
    if t == "dragon"   { return "#3a1a90" }
    if t == "dark"     { return "#3a2818" }
    if t == "steel"    { return "#74748a" }
    if t == "fairy"    { return "#a85878" }
    "#444"
}

/// A translucent rgba glow for body backgrounds (alpha ~0.18).
public func typeGlow(t: String) -> String {
    if t == "normal"   { return "rgba(168,168,120,0.18)" }
    if t == "fire"     { return "rgba(240,128,48,0.22)" }
    if t == "water"    { return "rgba(104,144,240,0.22)" }
    if t == "electric" { return "rgba(248,208,48,0.20)" }
    if t == "grass"    { return "rgba(120,200,80,0.20)" }
    if t == "ice"      { return "rgba(152,216,216,0.22)" }
    if t == "fighting" { return "rgba(192,48,40,0.22)" }
    if t == "poison"   { return "rgba(160,64,160,0.22)" }
    if t == "ground"   { return "rgba(224,192,104,0.20)" }
    if t == "flying"   { return "rgba(168,144,240,0.22)" }
    if t == "psychic"  { return "rgba(248,88,136,0.22)" }
    if t == "bug"      { return "rgba(168,184,32,0.20)" }
    if t == "rock"     { return "rgba(184,160,56,0.20)" }
    if t == "ghost"    { return "rgba(112,88,152,0.22)" }
    if t == "dragon"   { return "rgba(112,56,248,0.24)" }
    if t == "dark"     { return "rgba(112,88,72,0.22)" }
    if t == "steel"    { return "rgba(184,184,208,0.18)" }
    if t == "fairy"    { return "rgba(238,153,172,0.22)" }
    "rgba(120,120,120,0.18)"
}

public func typeEmoji(t: String) -> String {
    if t == "normal"   { return "\u{2B50}" }      // ⭐
    if t == "fire"     { return "\u{1F525}" }     // 🔥
    if t == "water"    { return "\u{1F4A7}" }     // 💧
    if t == "electric" { return "\u{26A1}" }      // ⚡
    if t == "grass"    { return "\u{1F33F}" }     // 🌿
    if t == "ice"      { return "\u{2744}\u{FE0F}" } // ❄️
    if t == "fighting" { return "\u{1F44A}" }     // 👊
    if t == "poison"   { return "\u{2620}\u{FE0F}" } // ☠️
    if t == "ground"   { return "\u{1F30D}" }     // 🌍
    if t == "flying"   { return "\u{1FAB6}" }     // 🪶
    if t == "psychic"  { return "\u{1F52E}" }     // 🔮
    if t == "bug"      { return "\u{1F41B}" }     // 🐛
    if t == "rock"     { return "\u{1FAA8}" }     // 🪨
    if t == "ghost"    { return "\u{1F47B}" }     // 👻
    if t == "dragon"   { return "\u{1F409}" }     // 🐉
    if t == "dark"     { return "\u{1F319}" }     // 🌙
    if t == "steel"    { return "\u{2699}\u{FE0F}" } // ⚙️
    if t == "fairy"    { return "\u{1F9DA}" }     // 🧚
    "\u{2728}"
}

// ============================================================================
// STAT THEMING
// ============================================================================

public func statLabel(name: String) -> String {
    if name == "hp"              { return "HP" }
    if name == "attack"          { return "Atk" }
    if name == "defense"         { return "Def" }
    if name == "special-attack"  { return "SpA" }
    if name == "special-defense" { return "SpD" }
    if name == "speed"           { return "Spe" }
    name
}

/// Map a stat's base value (0..255 in practice) to a 0..100 percentage for the bar.
public func statPercent(value: Int64) -> Int64 {
    let max: Int64 = 200;
    let v = if value > max { max } else { value };
    (v * 100) / max
}

public func statColor(value: Int64) -> String {
    if value < 60   { return "#f87171" }
    if value < 90   { return "#fbbf24" }
    if value < 120  { return "#6ee7b7" }
    "#67e8f9"
}

// ============================================================================
// RADAR GEOMETRY
//
// Six-vertex hexagon, 12-o'clock and clockwise. Stat indices from PokeAPI
// (hp, atk, def, spA, spD, spe) are mapped to angle slots so the radar
// reads naturally: HP top, Atk top-right, Def bottom-right, Spe bottom,
// SpD bottom-left, SpA top-left.
// ============================================================================

public func unitX(angleIdx: Int64) -> Float64 {
    if angleIdx == 0 { return 0.0 }
    if angleIdx == 1 { return 0.866 }
    if angleIdx == 2 { return 0.866 }
    if angleIdx == 3 { return 0.0 }
    if angleIdx == 4 { return -0.866 }
    return -0.866
}

public func unitY(angleIdx: Int64) -> Float64 {
    if angleIdx == 0 { return -1.0 }
    if angleIdx == 1 { return -0.5 }
    if angleIdx == 2 { return 0.5 }
    if angleIdx == 3 { return 1.0 }
    if angleIdx == 4 { return 0.5 }
    return -0.5
}

/// Map PokeAPI stat index (hp=0, atk=1, def=2, spA=3, spD=4, spe=5) to its
/// position on the hexagon (0=top, 1=top-right, ... 5=top-left).
public func statAngleIdx(statIdx: Int64) -> Int64 {
    if statIdx == 3 { return 5 }
    if statIdx == 4 { return 4 }
    if statIdx == 5 { return 3 }
    statIdx
}

/// Inverse of `statAngleIdx`: which PokeAPI stat lives at each angle slot
/// (0=top, 1=top-right, ... 5=top-left). Required to walk the radar polygon
/// in clockwise order so its edges don't cross.
public func angleToStatIdx(angleIdx: Int64) -> Int64 {
    if angleIdx == 3 { return 5 }
    if angleIdx == 5 { return 3 }
    angleIdx
}

public func statShortLabel(statIdx: Int64) -> String {
    if statIdx == 0 { return "HP" }
    if statIdx == 1 { return "ATK" }
    if statIdx == 2 { return "DEF" }
    if statIdx == 3 { return "SpA" }
    if statIdx == 4 { return "SpD" }
    if statIdx == 5 { return "SPE" }
    ""
}

/// Round a Float64 to the nearest Int64 (used for SVG coords).
public func roundInt(f: Float64) -> Int64 {
    let r = if f >= 0.0 { f + 0.5 } else { f - 0.5 };
    match r.toInt64() {
        .Some(n) => n,
        .None => 0
    }
}
