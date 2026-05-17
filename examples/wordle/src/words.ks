// Word list and seed-to-answer picker.
//
// One curated list serves as both the answer pool and the dictionary of
// accepted guesses. Real Wordle separates them; we keep things simple.

module wordle.words

public func wordList() -> Array[String] {
    var w = Array[String]();
    w.append("ABOUT"); w.append("ABOVE"); w.append("ALERT"); w.append("ALIEN");
    w.append("ALONE"); w.append("AMBER"); w.append("ANGEL"); w.append("APPLE");
    w.append("ARGUE"); w.append("ARROW"); w.append("AUDIO"); w.append("AVERT");
    w.append("BADGE"); w.append("BAKER"); w.append("BASIC"); w.append("BEACH");
    w.append("BEGIN"); w.append("BERRY"); w.append("BIRTH"); w.append("BLACK");
    w.append("BLAME"); w.append("BLAST"); w.append("BLEND"); w.append("BLIND");
    w.append("BLOCK"); w.append("BLOOM"); w.append("BLUNT"); w.append("BOARD");
    w.append("BRAIN"); w.append("BRAND"); w.append("BRAVE"); w.append("BREAD");
    w.append("BREAK"); w.append("BRICK"); w.append("BRING"); w.append("BROAD");
    w.append("BROOK"); w.append("BROWN"); w.append("BUILD"); w.append("BUNCH");
    w.append("CABIN"); w.append("CABLE"); w.append("CANDY"); w.append("CARGO");
    w.append("CARVE"); w.append("CATCH"); w.append("CAUSE"); w.append("CHAIR");
    w.append("CHALK"); w.append("CHARM"); w.append("CHART"); w.append("CHASE");
    w.append("CHEAP"); w.append("CHECK"); w.append("CHESS"); w.append("CHEST");
    w.append("CHIEF"); w.append("CHILD"); w.append("CHILL"); w.append("CHOIR");
    w.append("CIDER"); w.append("CLAIM"); w.append("CLASS"); w.append("CLEAN");
    w.append("CLEAR"); w.append("CLERK"); w.append("CLICK"); w.append("CLIFF");
    w.append("CLIMB"); w.append("CLOCK"); w.append("CLOSE"); w.append("CLOTH");
    w.append("CLOUD"); w.append("CLOVE"); w.append("COAST"); w.append("COULD");
    w.append("COUNT"); w.append("COURT"); w.append("COVER"); w.append("CRANE");
    w.append("CRAZY"); w.append("CREAM"); w.append("CRISP"); w.append("CROSS");
    w.append("CROWD"); w.append("CROWN"); w.append("CRUMB"); w.append("CURVE");
    w.append("DAILY"); w.append("DANCE"); w.append("DEPTH"); w.append("DOUGH");
    w.append("DOZEN"); w.append("DRAFT"); w.append("DREAM"); w.append("DRINK");
    w.append("DRIVE"); w.append("EAGLE"); w.append("EARLY"); w.append("EARTH");
    w.append("EMBER"); w.append("EMPTY"); w.append("ENJOY"); w.append("ENTER");
    w.append("EQUAL"); w.append("EVERY"); w.append("EXACT"); w.append("EXIST");
    w.append("EXTRA"); w.append("FAITH"); w.append("FANCY"); w.append("FAULT");
    w.append("FEAST"); w.append("FENCE"); w.append("FERRY"); w.append("FIELD");
    w.append("FIGHT"); w.append("FINAL"); w.append("FIRST"); w.append("FLAME");
    w.append("FLASH"); w.append("FLEET"); w.append("FLESH"); w.append("FLOAT");
    w.append("FLOOR"); w.append("FLOUR"); w.append("FLUTE"); w.append("FOCUS");
    w.append("FORCE"); w.append("FOUND"); w.append("FRAME"); w.append("FRESH");
    w.append("FROST"); w.append("FRUIT"); w.append("FUNNY"); w.append("GHOST");
    w.append("GIANT"); w.append("GLASS"); w.append("GLEAM"); w.append("GLOBE");
    w.append("GLOSS"); w.append("GLOVE"); w.append("GRACE"); w.append("GRAIN");
    w.append("GRAND"); w.append("GRANT"); w.append("GRAPE"); w.append("GRASS");
    w.append("GRAVE"); w.append("GREAT"); w.append("GREEN"); w.append("GRIEF");
    w.append("GROUP"); w.append("GROVE"); w.append("GUARD"); w.append("GUEST");
    w.append("GUIDE"); w.append("HABIT"); w.append("HAPPY"); w.append("HEART");
    w.append("HEAVY"); w.append("HONEY"); w.append("HORSE"); w.append("HOTEL");
    w.append("HOUSE"); w.append("HUMAN"); w.append("HUMOR"); w.append("ICILY");
    w.append("IMAGE"); w.append("INDEX"); w.append("INNER"); w.append("INPUT");
    w.append("IVORY"); w.append("JOINT"); w.append("JOLLY"); w.append("JUDGE");
    w.append("JUICE"); w.append("KAYAK"); w.append("KNEEL"); w.append("KNIFE");
    w.append("KNOCK"); w.append("KNOWN"); w.append("LABEL"); w.append("LARGE");
    w.append("LASER"); w.append("LATER"); w.append("LAYER"); w.append("LEARN");
    w.append("LEAST"); w.append("LEAVE"); w.append("LEGAL"); w.append("LEMON");
    w.append("LEVEL"); w.append("LIGHT"); w.append("LIMIT"); w.append("LOCAL");
    w.append("LOGIC"); w.append("LOYAL"); w.append("LUCKY"); w.append("LUNCH");
    w.append("MAGIC"); w.append("MAJOR"); w.append("MAPLE"); w.append("MARCH");
    w.append("MATCH"); w.append("MAYOR"); w.append("MEDAL"); w.append("METAL");
    w.append("METER"); w.append("MIGHT"); w.append("MINOR"); w.append("MIXER");
    w.append("MODEL"); w.append("MONEY"); w.append("MONTH"); w.append("MORAL");
    w.append("MOTOR"); w.append("MOUNT"); w.append("MOUSE"); w.append("MOUTH");
    w.append("MOVIE"); w.append("MUSIC"); w.append("NEEDY"); w.append("NEVER");
    w.append("NIGHT"); w.append("NOBLE"); w.append("NOISE"); w.append("NORTH");
    w.append("NOTCH"); w.append("OCEAN"); w.append("OFFER"); w.append("OLDER");
    w.append("ORDER"); w.append("OTHER"); w.append("OUTER"); w.append("OWNER");
    w.append("PAINT"); w.append("PANEL"); w.append("PAPER"); w.append("PARTY");
    w.append("PATCH"); w.append("PEACE"); w.append("PEARL"); w.append("PHONE");
    w.append("PHOTO"); w.append("PIANO"); w.append("PILOT"); w.append("PIZZA");
    w.append("PLACE"); w.append("PLAIN"); w.append("PLANE"); w.append("PLANT");
    w.append("PLATE"); w.append("PLAZA"); w.append("PLUMB"); w.append("POINT");
    w.append("PORCH"); w.append("POUND"); w.append("POWER"); w.append("PRESS");
    w.append("PRICE"); w.append("PRIDE"); w.append("PRIME"); w.append("PRINT");
    w.append("PRIZE"); w.append("PROOF"); w.append("PROUD"); w.append("PROVE");
    w.append("PULSE"); w.append("QUACK"); w.append("QUAKE"); w.append("QUEEN");
    w.append("QUERY"); w.append("QUEST"); w.append("QUICK"); w.append("QUIET");
    w.append("QUILT"); w.append("QUITE"); w.append("RADIO"); w.append("RAINY");
    w.append("RAISE"); w.append("RANCH"); w.append("RANGE"); w.append("RAPID");
    w.append("REACH"); w.append("REACT"); w.append("READY"); w.append("REALM");
    w.append("REBEL"); w.append("REFER"); w.append("RELAX"); w.append("REPLY");
    w.append("RIDGE"); w.append("RIGHT"); w.append("RIVER"); w.append("ROAST");
    w.append("ROBIN"); w.append("ROBOT"); w.append("ROUGH"); w.append("ROUND");
    w.append("ROUTE"); w.append("ROYAL"); w.append("RURAL"); w.append("SALAD");
    w.append("SCALE"); w.append("SCENE"); w.append("SCOPE"); w.append("SCORE");
    w.append("SEVEN"); w.append("SHADE"); w.append("SHAKE"); w.append("SHALL");
    w.append("SHAPE"); w.append("SHARE"); w.append("SHARP"); w.append("SHEEP");
    w.append("SHEET"); w.append("SHELF"); w.append("SHELL"); w.append("SHIFT");
    w.append("SHINE"); w.append("SHINY"); w.append("SHIRT"); w.append("SHOCK");
    w.append("SHOOT"); w.append("SHORE"); w.append("SHORT"); w.append("SHOUT");
    w.append("SHOWN"); w.append("SIGHT"); w.append("SILLY"); w.append("SINCE");
    w.append("SIXTH"); w.append("SIXTY"); w.append("SKILL"); w.append("SLATE");
    w.append("SLEEP"); w.append("SLEPT"); w.append("SLICE"); w.append("SLIDE");
    w.append("SLOPE"); w.append("SMALL"); w.append("SMART"); w.append("SMILE");
    w.append("SMOKE"); w.append("SNACK"); w.append("SNAKE"); w.append("SNEAK");
    w.append("SNORE"); w.append("SOLID"); w.append("SOLVE"); w.append("SORRY");
    w.append("SOUND"); w.append("SOUTH"); w.append("SPACE"); w.append("SPADE");
    w.append("SPARE"); w.append("SPARK"); w.append("SPEAK"); w.append("SPEED");
    w.append("SPELL"); w.append("SPEND"); w.append("SPICE"); w.append("SPICY");
    w.append("SPINE"); w.append("SPLIT"); w.append("SPOKE"); w.append("SPORT");
    w.append("SPRAY"); w.append("STAFF"); w.append("STAGE"); w.append("STAIR");
    w.append("STAKE"); w.append("STAMP"); w.append("STAND"); w.append("STARE");
    w.append("START"); w.append("STATE"); w.append("STEAM"); w.append("STEEL");
    w.append("STEEP"); w.append("STERN"); w.append("STICK"); w.append("STIFF");
    w.append("STILL"); w.append("STING"); w.append("STOCK"); w.append("STONE");
    w.append("STOOD"); w.append("STORE"); w.append("STORM"); w.append("STORY");
    w.append("STOVE"); w.append("STRAP"); w.append("STRAW"); w.append("STRIP");
    w.append("STUDY"); w.append("STUFF"); w.append("STYLE"); w.append("SUGAR");
    w.append("SUITE"); w.append("SUNNY"); w.append("SUPER"); w.append("SWEET");
    w.append("SWIFT"); w.append("SWING"); w.append("SWORD"); w.append("TABLE");
    w.append("TASTE"); w.append("TEACH"); w.append("THANK"); w.append("THEFT");
    w.append("THEIR"); w.append("THERE"); w.append("THESE"); w.append("THICK");
    w.append("THING"); w.append("THINK"); w.append("THIRD"); w.append("THORN");
    w.append("THOSE"); w.append("THREE"); w.append("THROW"); w.append("TIGHT");
    w.append("TIGER"); w.append("TODAY"); w.append("TOOTH"); w.append("TOPIC");
    w.append("TORCH"); w.append("TOUCH"); w.append("TOUGH"); w.append("TOWER");
    w.append("TRACK"); w.append("TRADE"); w.append("TRAIL"); w.append("TRAIN");
    w.append("TREAT"); w.append("TREND"); w.append("TRIAL"); w.append("TRIBE");
    w.append("TRICK"); w.append("TROUT"); w.append("TRUCK"); w.append("TRULY");
    w.append("TRUNK"); w.append("TRUST"); w.append("TRUTH"); w.append("TWIST");
    w.append("UNDER"); w.append("UNION"); w.append("UNITY"); w.append("UNTIL");
    w.append("UPPER"); w.append("URBAN"); w.append("USAGE"); w.append("USHER");
    w.append("USUAL"); w.append("VAGUE"); w.append("VALID"); w.append("VALUE");
    w.append("VAULT"); w.append("VINYL"); w.append("VIRUS"); w.append("VISIT");
    w.append("VIVID"); w.append("VOCAL"); w.append("VOICE"); w.append("WAIST");
    w.append("WATCH"); w.append("WATER"); w.append("WEARY"); w.append("WEDGE");
    w.append("WHEAT"); w.append("WHEEL"); w.append("WHERE"); w.append("WHICH");
    w.append("WHILE"); w.append("WHITE"); w.append("WHOLE"); w.append("WIDTH");
    w.append("WORLD"); w.append("WORRY"); w.append("WORTH"); w.append("WOUND");
    w.append("WRECK"); w.append("WRIST"); w.append("WRITE"); w.append("WRONG");
    w.append("YEAST"); w.append("YIELD"); w.append("YOUNG"); w.append("YOUTH");
    w.append("ZEBRA");
    w
}

/// Pick an answer for the given seed. A small mixing step keeps adjacent
/// seeds from yielding adjacent words in the list.
public func pickWord(seed: Int64, words: Array[String]) -> String {
    let s = if seed < 0 { 0 - seed } else { seed };
    let mixed = (s * 2654435761) % 2147483647;
    let positive = if mixed < 0 { 0 - mixed } else { mixed };
    let idx = positive % words.count;
    words(unchecked: idx).clone()
}

/// Linear scan; the list is small enough that a hash isn't worth it.
public func isValidWord(w: String, words: Array[String]) -> Bool {
    var i: Int64 = 0;
    while i < words.count {
        if words(unchecked: i) == w { return true };
        i = i + 1
    }
    false
}
