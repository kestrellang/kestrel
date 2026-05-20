module wall.filter

public func containsProfanity(text: String, blocklist: Set[String]) -> Bool {
    let lower = text.lowercased();
    for word in lower.split(" ") {
        let trimmed = word.trimmed().toOwned();
        if trimmed.byteCount > 0 {
            if blocklist.contains(trimmed) {
                return true
            }
        }
    };
    false
}
