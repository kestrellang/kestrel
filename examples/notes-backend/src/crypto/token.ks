module notes.crypto

import std.numeric.(SystemRandom)

// Generates a session token: 256 bits of OS entropy. Every login must get a
// fresh, unguessable token — `tokens.token` is UNIQUE, so a token derived
// from the user (as it once was) made every login after the first fail.
public func generateToken() -> String {
    var rng = SystemRandom();
    var token = "tok-";
    for _ in 0..<4 {
        token = token + rng.nextUInt64().formatted();
    }
    token
}
