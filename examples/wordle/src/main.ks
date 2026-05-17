// Wordle clone — all state lives in the URL (?s=<seed>&g=<comma-list>),
// so any guess is shareable and bookmarkable. Submitting a guess does a
// plain GET form submission and the server 302s back to a clean URL.

module wordle.main

import perch.app.(App)
import perch.request.(Request)
import perch.response.(Response)
import perch.middleware.(logger)
import wordle.words.(wordList, pickWord, isValidWord)
import wordle.game.(MAX_GUESSES, WORD_LEN, outcome, Outcome)
import wordle.ui.(pageHtml)
import http.url.(percentEncode)
import wordle.util.(splitGuesses, joinGuesses, parseSeed, normalizeGuess)

// ============================================================================
// CONTEXT — the word list is loaded once and reused across requests.
// ============================================================================

struct Ctx: Cloneable {
    var words: Array[String]

    func clone() -> Ctx {
        var copy = Array[String]();
        var i: Int64 = 0;
        while i < self.words.count {
            copy.append(self.words(unchecked: i).clone());
            i = i + 1
        };
        Ctx(words: copy)
    }
}

// ============================================================================
// ROUTE HANDLER
// ============================================================================

func handleRoot(req: Request, ctx: Ctx) -> Response {
    let sParam = match req.query("s") {
        .Some(v) => v,
        .None => ""
    };
    if sParam.byteCount == 0 {
        // First visit — pick a default seed and redirect so the URL is shareable.
        return Response.redirect(to: "/?s=42")
    };

    let seed = parseSeed(sParam);
    let answer = pickWord(seed, ctx.words);
    var guesses = splitGuesses(match req.query("g") {
        .Some(v) => v,
        .None => ""
    });

    let errFromUrl = match req.query("err") {
        .Some(v) => decodeError(v),
        .None => ""
    };

    let raw = match req.query("w") {
        .Some(v) => v,
        .None => ""
    };

    if raw.byteCount > 0 {
        match validateAndAppend(raw, guesses, answer, ctx.words) {
            .Ok(newGuesses) => {
                return Response.redirect(to: cleanUrl(seed, newGuesses, ""))
            },
            .Err(msg) => {
                return Response.redirect(to: cleanUrl(seed, guesses, msg))
            }
        }
    };

    Response.ok(html: pageHtml(seed, guesses, answer, errFromUrl))
}

// ============================================================================
// VALIDATION
// ============================================================================

enum GuessResult: Cloneable {
    case Ok(Array[String])
    case Err(String)

    func clone() -> GuessResult {
        match self {
            .Ok(arr) => GuessResult.Ok(arr.clone()),
            .Err(s) => GuessResult.Err(s.clone())
        }
    }
}

func validateAndAppend(raw: String, guesses: Array[String], answer: String, words: Array[String]) -> GuessResult {
    // Game already over?
    match outcome(guesses, answer) {
        .InProgress => {},
        _ => return GuessResult.Err("game-over")
    };

    let normalized = normalizeGuess(raw);
    if normalized.byteCount == 0 {
        return GuessResult.Err("bad-format")
    };

    var newList = Array[String]();
    var i: Int64 = 0;
    while i < guesses.count {
        newList.append(guesses(unchecked: i).clone());
        i = i + 1
    };
    newList.append(normalized);
    GuessResult.Ok(newList)
}

// ============================================================================
// URL HELPERS
// ============================================================================

func cleanUrl(seed: Int64, guesses: Array[String], err: String) -> String {
    var u = String();
    u.append("/?s=");
    u.append(seed.formatted());
    if guesses.count > 0 {
        u.append("&g=");
        u.append(joinGuesses(guesses))
    };
    if err.byteCount > 0 {
        u.append("&err=");
        u.append(percentEncode(err))
    };
    u
}

func decodeError(code: String) -> String {
    if code == "bad-format" { return "Type 5 letters." };
    if code == "not-a-word" { return "Not in word list." };
    if code == "game-over" { return "Game over — start a new one." };
    ""
}

// ============================================================================
// MAIN
// ============================================================================

func main() {
    let ctx = Ctx(words: wordList());
    var app = App[Ctx](ctx);
    app.use(logger[Ctx]());

    app.onGet("/", { (req: Request, ctx: Ctx) in
        handleRoot(req, ctx)
    });

    let port: UInt16 = 8090;
    let _ = println("Wordle running on http://localhost:8090");
    match app.listen(port) {
        .Ok(_) => {},
        .Err(e) => {
            let _ = println("Error: " + e.description());
        }
    }
}
