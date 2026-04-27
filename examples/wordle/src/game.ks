// Wordle scoring and game-state helpers.

module wordle.game

public enum LetterState {
    case Untouched
    case Absent
    case Present
    case Correct
}

public enum Outcome {
    case InProgress
    case Won
    case Lost
}

public let MAX_GUESSES: Int64 = 6
public let WORD_LEN: Int64 = 5

/// Score a guess against the answer using two-pass duplicate-aware logic:
/// pass 1 marks exact matches and consumes those answer letters; pass 2
/// resolves "present" against what remains. This is what stops a second
/// 'A' in the guess from showing yellow when the answer only has one.
public func scoreGuess(guess: String, answer: String) -> Array[LetterState] {
    var result = Array[LetterState](repeating: LetterState.Absent, count: WORD_LEN);
    // Track which answer letters are still available after pass 1.
    var consumed = Array[Bool](repeating: false, count: WORD_LEN);

    var i: Int64 = 0;
    while i < WORD_LEN {
        if guess.bytes(unchecked: i) == answer.bytes(unchecked: i) {
            result(i) = LetterState.Correct;
            consumed(i) = true
        };
        i = i + 1
    }

    i = 0;
    while i < WORD_LEN {
        match result(unchecked: i) {
            .Correct => {},
            _ => {
                let g = guess.bytes(unchecked: i);
                var j: Int64 = 0;
                var found = false;
                while j < WORD_LEN and not found {
                    if not consumed(unchecked: j) and answer.bytes(unchecked: j) == g {
                        result(i) = LetterState.Present;
                        consumed(j) = true;
                        found = true
                    };
                    j = j + 1
                }
            }
        };
        i = i + 1
    }
    result
}

/// Best state seen for each letter A-Z across all guesses, used to color
/// the on-screen keyboard. Correct > Present > Absent > Untouched.
public func keyboardState(guesses: Array[String], answer: String) -> Array[LetterState] {
    var states = Array[LetterState](repeating: LetterState.Untouched, count: 26);
    var gi: Int64 = 0;
    while gi < guesses.count {
        let g = guesses(unchecked: gi);
        let scored = scoreGuess(g, answer);
        var i: Int64 = 0;
        while i < WORD_LEN {
            let letter = g.bytes(unchecked: i);
            let idx = Int64(from: letter) - 65;
            if idx >= 0 and idx < 26 {
                states(idx) = betterState(states(unchecked: idx), scored(unchecked: i))
            };
            i = i + 1
        };
        gi = gi + 1
    }
    states
}

func betterState(a: LetterState, b: LetterState) -> LetterState {
    if rank(b) > rank(a) { b } else { a }
}

func rank(s: LetterState) -> Int64 {
    match s {
        .Untouched => 0,
        .Absent => 1,
        .Present => 2,
        .Correct => 3
    }
}

public func outcome(guesses: Array[String], answer: String) -> Outcome {
    if guesses.count > 0 {
        let last = guesses(unchecked: guesses.count - 1);
        if last == answer { return Outcome.Won }
    };
    if guesses.count >= MAX_GUESSES { return Outcome.Lost };
    Outcome.InProgress
}
