// HTML rendering: full page, board, keyboard, banner, toast.

module wordle.ui

import plume.(Template)
import wordle.game.(LetterState, Outcome, MAX_GUESSES, WORD_LEN, scoreGuess, keyboardState, outcome)
import wordle.util.(joinGuesses, urlEncode)

// ============================================================================
// CSS
// ============================================================================

func pageCss() -> String {
    var s = String(capacity: 4096);
    s.append(##"*{box-sizing:border-box;margin:0;padding:0}body{font-family:'Inter',system-ui,-apple-system,sans-serif;background:#0e0e10;color:#d7dadc;min-height:100vh;-webkit-font-smoothing:antialiased;display:flex;flex-direction:column;align-items:center;padding:32px 16px 48px}a{color:inherit;text-decoration:none}.header{display:flex;align-items:baseline;justify-content:space-between;width:100%;max-width:480px;margin-bottom:28px;padding-bottom:14px;border-bottom:1px solid #2a2a2c}.title{font-family:'Newsreader','Times New Roman',serif;font-size:1.85rem;font-weight:600;font-style:italic;letter-spacing:-0.01em;color:#e8e8ea}.new-game{font-size:0.72rem;color:#818384;letter-spacing:0.12em;text-transform:uppercase;font-weight:600;padding:4px 0;border-bottom:1px solid transparent;transition:color 0.15s,border-color 0.15s}.new-game:hover{color:#d7dadc;border-bottom-color:#d7dadc}.board{display:grid;grid-template-rows:repeat(6,1fr);gap:5px;margin-bottom:28px}.row{display:grid;grid-template-columns:repeat(5,1fr);gap:5px}.tile{width:60px;height:60px;display:flex;align-items:center;justify-content:center;font-size:1.7rem;font-weight:700;text-transform:uppercase;color:#d7dadc;letter-spacing:0;border-radius:2px}.tile.empty{background:transparent;border:1.5px solid #3a3a3c}.tile.absent{background:#3a3a3c;color:#d7dadc}.tile.present{background:#b59f3b;color:#fff}.tile.correct{background:#538d4e;color:#fff}.banner{width:100%;max-width:336px;margin-bottom:18px;padding:18px 20px;text-align:center;border-top:1px solid #2a2a2c;border-bottom:1px solid #2a2a2c;font-size:0.88rem;color:#a0a0a2;letter-spacing:0.02em}.banner.won{color:#6aaa64}.banner.lost{color:#818384}.banner .answer{display:block;margin-top:8px;font-family:'Newsreader','Times New Roman',serif;font-style:italic;font-weight:600;font-size:1.5rem;letter-spacing:0.04em;color:#e8e8ea}.toast{position:fixed;top:24px;left:50%;transform:translate(-50%,0);background:#e8e8ea;color:#0e0e10;padding:10px 20px;border-radius:2px;font-size:0.78rem;font-weight:600;letter-spacing:0.04em;text-transform:uppercase;box-shadow:0 4px 20px rgba(0,0,0,0.5);animation:toastIn 0.2s ease both,toastOut 0.3s ease 2.5s both;z-index:1000}.input-row{display:flex;gap:6px;width:100%;max-width:336px;margin-bottom:32px}.input-row input{flex:1;padding:13px 16px;border-radius:2px;border:1.5px solid #3a3a3c;background:transparent;color:#d7dadc;font-size:1rem;font-family:inherit;font-weight:600;letter-spacing:0.3em;text-transform:uppercase;outline:none;transition:border-color 0.15s}.input-row input::placeholder{color:#565758;letter-spacing:0.02em;text-transform:none;font-weight:400}.input-row input:focus{border-color:#818384}.input-row button{padding:13px 22px;border-radius:2px;border:none;background:#e8e8ea;color:#0e0e10;font-weight:700;font-size:0.78rem;text-transform:uppercase;letter-spacing:0.1em;cursor:pointer;transition:background 0.15s}.input-row button:hover{background:#fff}.keyboard{display:flex;flex-direction:column;gap:6px;width:100%;max-width:484px}.kb-row{display:flex;gap:5px;justify-content:center}.key{flex:1;min-width:0;height:50px;display:flex;align-items:center;justify-content:center;border-radius:3px;font-size:0.85rem;font-weight:700;color:#d7dadc;background:#818384;text-transform:uppercase;transition:background 0.2s,color 0.2s}.key.absent{background:#262628;color:#5a5a5c}.key.present{background:#b59f3b;color:#fff}.key.correct{background:#538d4e;color:#fff}.share-row{display:flex;gap:8px;width:100%;max-width:336px;margin-bottom:32px}.share-input{flex:1;padding:10px 14px;border-radius:2px;border:1px solid #3a3a3c;background:transparent;color:#818384;font-size:0.78rem;font-family:'Menlo','SF Mono',monospace;outline:none;letter-spacing:0}@keyframes toastIn{from{opacity:0;transform:translate(-50%,-10px)}to{opacity:1;transform:translate(-50%,0)}}@keyframes toastOut{to{opacity:0;transform:translate(-50%,-10px)}}@media(max-width:420px){.tile{width:52px;height:52px;font-size:1.5rem}.key{height:46px;font-size:0.78rem}}"##);
    s
}

// ============================================================================
// PAGE RENDERING
// ============================================================================

public func pageHtml(seed: Int64, guesses: Array[String], answer: String, errorMsg: String) -> String {
    var h = String(capacity: 8192);
    let result = outcome(guesses, answer);

    h.append(##"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Kestrel Wordle</title><link rel="preconnect" href="https://fonts.googleapis.com"><link rel="preconnect" href="https://fonts.gstatic.com" crossorigin><link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=Newsreader:ital,opsz,wght@1,16..72,500;1,16..72,600&display=swap" rel="stylesheet"><style>"##);
    h.append(pageCss());
    h.append("</style></head><body>");

    // Toast for errors
    if errorMsg.byteCount > 0 {
        var t = Template();
        t.put("msg", errorMsg);
        h.append(t.render(##"<div class="toast">{msg}</div>"##))
    };

    // Header
    var nextSeedStr = String();
    nextSeedStr.append((seed * 2654435761 % 2147483647).formatted());
    var th = Template();
    th.setRaw("nextSeed", nextSeedStr);
    h.append(th.render(##"<div class="header"><div class="title">Kestrel Wordle</div><a class="new-game" href="/?s={nextSeed}">New Game &rarr;</a></div>"##));

    // End-of-game banner
    match result {
        .Won => {
            var t = Template();
            t.setRaw("guesses", guesses.count.formatted());
            t.put("answer", answer);
            h.append(t.render(##"<div class="banner won">Solved in {guesses}<span class="answer">{answer}</span></div>"##))
        },
        .Lost => {
            var t = Template();
            t.put("answer", answer);
            h.append(t.render(##"<div class="banner lost">Out of guesses<span class="answer">{answer}</span></div>"##))
        },
        .InProgress => {}
    };

    // Board
    h.append(boardHtml(guesses, answer));

    // Input form (or share row, if game is over)
    match result {
        .InProgress => h.append(inputFormHtml(seed, guesses)),
        _ => h.append(shareRowHtml(seed))
    };

    // Keyboard
    h.append(keyboardHtml(guesses, answer));

    h.append("</body></html>");
    h
}

func boardHtml(guesses: Array[String], answer: String) -> String {
    var h = String(capacity: 2048);
    h.append(##"<div class="board">"##);

    var row: Int64 = 0;
    while row < MAX_GUESSES {
        h.append(##"<div class="row">"##);
        if row < guesses.count {
            let g = guesses(unchecked: row);
            let scored = scoreGuess(g, answer);
            var i: Int64 = 0;
            while i < WORD_LEN {
                let ch = g.asSlice().subslice(from: i, to: i + 1).toOwned();
                let cls = stateClass(scored(unchecked: i));
                var t = Template();
                t.setRaw("cls", cls);
                t.put("ch", ch);
                t.setInt("delay", i * 100);
                h.append(t.render(##"<div class="tile {cls}" style="animation-delay:{delay}ms">{ch}</div>"##));
                i = i + 1
            }
        } else {
            var i: Int64 = 0;
            while i < WORD_LEN {
                h.append(##"<div class="tile empty"></div>"##);
                i = i + 1
            }
        };
        h.append("</div>");
        row = row + 1
    };

    h.append("</div>");
    h
}

func inputFormHtml(seed: Int64, guesses: Array[String]) -> String {
    var t = Template();
    t.setRaw("seed", seed.formatted());
    t.put("guesses", joinGuesses(guesses));
    t.render(##"<form class="input-row" method="get" action="/" autocomplete="off"><input type="hidden" name="s" value="{seed}"><input type="hidden" name="g" value="{guesses}"><input type="text" name="w" maxlength="5" minlength="5" placeholder="Enter guess" autofocus required pattern="[A-Za-z]{{5}}"><button type="submit">Guess</button></form>"##)
}

func shareRowHtml(seed: Int64) -> String {
    var t = Template();
    t.setRaw("seed", seed.formatted());
    t.render(##"<div class="share-row"><input class="share-input" readonly value="http://localhost:8090/?s={seed}" onclick="this.select()"></div>"##)
}

func keyboardHtml(guesses: Array[String], answer: String) -> String {
    let states = keyboardState(guesses, answer);
    var h = String(capacity: 1024);
    h.append(##"<div class="keyboard">"##);
    h.append(kbRowHtml("QWERTYUIOP", states));
    h.append(kbRowHtml("ASDFGHJKL", states));
    h.append(kbRowHtml("ZXCVBNM", states));
    h.append("</div>");
    h
}

func kbRowHtml(letters: String, states: Array[LetterState]) -> String {
    var h = String();
    h.append(##"<div class="kb-row">"##);
    var i: Int64 = 0;
    while i < letters.byteCount {
        let b = letters.bytes(unchecked: i);
        let idx = Int64(from: b) - 65;
        let cls = stateClass(states(unchecked: idx));
        let ch = letters.asSlice().subslice(from: i, to: i + 1).toOwned();
        var t = Template();
        t.setRaw("cls", cls);
        t.put("ch", ch);
        h.append(t.render(##"<div class="key {cls}">{ch}</div>"##));
        i = i + 1
    }
    h.append("</div>");
    h
}

func stateClass(s: LetterState) -> String {
    match s {
        .Untouched => "",
        .Absent => "absent",
        .Present => "present",
        .Correct => "correct"
    }
}
