//! # Constructor Splitting
//!
//! Maranget's algorithm assumes the constructors a column switches on are
//! **disjoint**: specializing on `c` keeps exactly the rows whose head matches
//! every value of `c`. Most constructors are disjoint by construction (two enum
//! cases never share a value), but three families are not:
//!
//! | Family | Overlap example |
//! |--------|-----------------|
//! | integer literals and ranges | `0..=10` and `5..=15` share `5..=10` |
//! | char literals and ranges | `'a'..='m'` and `'f'..='z'` |
//! | array lengths | `[x, ..]` (len >= 1) and `[x, y]` (len 2) |
//!
//! Switching on the raw constructors routed a value to any arm that merely
//! *overlapped* the case it fell in, so `12` reached `0..=10` (F1). `split`
//! cuts each family into pieces that no row constructor straddles: every piece
//! lies entirely inside or entirely outside each row's constructor. Then
//! `Constructor::matches` (containment) is exact, for the decision tree and the
//! usefulness check alike.
//!
//! Only values some row constructor covers become pieces; gaps are left to the
//! default matrix, as before splitting.

use super::constructor::Constructor;

/// Split a column's constructors into disjoint pieces.
///
/// Non-splittable constructors pass through unchanged, in order and deduped.
pub fn split(ctors: &[Constructor]) -> Vec<Constructor> {
    let mut out: Vec<Constructor> = Vec::new();
    let mut ints: Vec<(i128, i128)> = Vec::new();
    let mut chars: Vec<(i128, i128)> = Vec::new();
    let mut arrays: Vec<(usize, usize, bool)> = Vec::new();

    for ctor in ctors {
        match ctor {
            Constructor::IntLiteral(_) | Constructor::IntRange { .. } => {
                ints.extend(int_interval(ctor));
            },
            Constructor::CharLiteral(_) | Constructor::CharRange { .. } => {
                chars.extend(char_interval(ctor));
            },
            Constructor::Array {
                prefix_len,
                suffix_len,
                has_rest,
            } => arrays.push((*prefix_len, *suffix_len, *has_rest)),
            _ if !out.contains(ctor) => out.push(ctor.clone()),
            _ => {},
        }
    }

    out.extend(
        split_intervals(&ints)
            .into_iter()
            .map(|(lo, hi)| int_piece(lo, hi)),
    );
    out.extend(
        split_intervals(&chars)
            .into_iter()
            .filter_map(|(lo, hi)| char_piece(lo, hi)),
    );
    out.extend(split_array_lengths(&arrays));
    out
}

/// True for the constructor families `split` cuts up.
pub fn is_splittable(ctor: &Constructor) -> bool {
    matches!(
        ctor,
        Constructor::IntLiteral(_)
            | Constructor::IntRange { .. }
            | Constructor::CharLiteral(_)
            | Constructor::CharRange { .. }
            | Constructor::Array { .. }
    )
}

// ===== Intervals =====
//
// Integers and chars are cut the same way, over `i128` so `i64::MAX + 1` and
// the open ends need no special cases.

const CHAR_MAX: i128 = char::MAX as i128;
const SURROGATE_LO: i128 = 0xD800;
const SURROGATE_HI: i128 = 0xDFFF;

/// Inclusive interval of an int literal or range. `None` for an empty range
/// (`10..=0`), which covers no value — bounds validation reports it.
pub(crate) fn int_interval(ctor: &Constructor) -> Option<(i128, i128)> {
    let (lo, hi) = match ctor {
        Constructor::IntLiteral(v) => (*v as i128, *v as i128),
        Constructor::IntRange { start, end } => (
            start.map_or(i64::MIN as i128, |s| s as i128),
            end.map_or(i64::MAX as i128, |e| e as i128),
        ),
        _ => return None,
    };
    (lo <= hi).then_some((lo, hi))
}

/// Inclusive codepoint interval of a char literal or range.
pub(crate) fn char_interval(ctor: &Constructor) -> Option<(i128, i128)> {
    let (lo, hi) = match ctor {
        Constructor::CharLiteral(c) => (*c as i128, *c as i128),
        Constructor::CharRange { start, end } => (
            start.map_or(0, |c| c as i128),
            end.map_or(CHAR_MAX, |c| c as i128),
        ),
        _ => return None,
    };
    (lo <= hi).then_some((lo, hi))
}

/// Cut `intervals` at every start and every end + 1, keeping the pieces that
/// some interval covers. Returned sorted and disjoint.
fn split_intervals(intervals: &[(i128, i128)]) -> Vec<(i128, i128)> {
    let mut cuts: Vec<i128> = intervals.iter().flat_map(|&(lo, hi)| [lo, hi + 1]).collect();
    cuts.sort_unstable();
    cuts.dedup();
    cuts.windows(2)
        .map(|w| (w[0], w[1] - 1))
        .filter(|&(lo, hi)| intervals.iter().any(|&(s, e)| s <= lo && hi <= e))
        .collect()
}

/// An integer piece. A single value becomes a literal so it compares equal to
/// the literal row it came from; open ends stay `None` (see #186).
fn int_piece(lo: i128, hi: i128) -> Constructor {
    if lo == hi {
        return Constructor::IntLiteral(lo as i64);
    }
    Constructor::IntRange {
        start: (lo != i64::MIN as i128).then_some(lo as i64),
        end: (hi != i64::MAX as i128).then_some(hi as i64),
    }
}

/// A char piece. Surrogate codepoints are not chars, so a piece's ends are
/// pulled out of the surrogate block; a piece inside it holds no char at all.
fn char_piece(lo: i128, hi: i128) -> Option<Constructor> {
    let lo = if (SURROGATE_LO..=SURROGATE_HI).contains(&lo) { SURROGATE_HI + 1 } else { lo };
    let hi = if (SURROGATE_LO..=SURROGATE_HI).contains(&hi) { SURROGATE_LO - 1 } else { hi };
    if lo > hi {
        return None;
    }
    let as_char = |v: i128| char::from_u32(v as u32);
    if lo == hi {
        return as_char(lo).map(Constructor::CharLiteral);
    }
    Some(Constructor::CharRange {
        start: if lo == 0 { None } else { Some(as_char(lo)?) },
        end: if hi == CHAR_MAX { None } else { Some(as_char(hi)?) },
    })
}

// ===== Array lengths =====

/// Cut array constructors into exact lengths plus one open-ended piece.
///
/// With a rest pattern present, every length from the shortest rest pattern's
/// minimum up to the longest fixed pattern gets its own exact piece, and one
/// `[prefix, .., suffix]` piece covers the lengths above that. Its prefix and
/// suffix are the widest any rest row uses, so every rest row can be laid over
/// it element by element; the prefix is padded so the piece starts past every
/// fixed length.
fn split_array_lengths(arrays: &[(usize, usize, bool)]) -> Vec<Constructor> {
    let exact = |len: usize| Constructor::Array {
        prefix_len: len,
        suffix_len: 0,
        has_rest: false,
    };
    let mut fixed: Vec<usize> = arrays
        .iter()
        .filter(|a| !a.2)
        .map(|&(p, s, _)| p + s)
        .collect();
    let rests: Vec<(usize, usize)> = arrays.iter().filter(|a| a.2).map(|&(p, s, _)| (p, s)).collect();

    if rests.is_empty() {
        fixed.sort_unstable();
        fixed.dedup();
        return fixed.into_iter().map(exact).collect();
    }

    let max_prefix = rests.iter().map(|r| r.0).max().unwrap_or(0);
    let max_suffix = rests.iter().map(|r| r.1).max().unwrap_or(0);
    let min_rest = rests.iter().map(|r| r.0 + r.1).min().unwrap_or(0);
    let max_fixed_plus_one = fixed.iter().max().map_or(0, |m| m + 1);
    let open_min = (max_prefix + max_suffix).max(max_fixed_plus_one);

    // Exact lengths: the fixed ones, plus every length a rest row covers below
    // the open piece.
    fixed.extend(min_rest..open_min);
    fixed.retain(|&len| len < open_min);
    fixed.sort_unstable();
    fixed.dedup();

    let mut out: Vec<Constructor> = fixed.into_iter().map(exact).collect();
    out.push(Constructor::Array {
        prefix_len: open_min - max_suffix,
        suffix_len: max_suffix,
        has_rest: true,
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(s: Option<i64>, e: Option<i64>) -> Constructor {
        Constructor::IntRange { start: s, end: e }
    }

    #[test]
    fn overlapping_int_ranges_split_at_every_edge() {
        let pieces = split(&[range(Some(0), Some(10)), range(Some(5), Some(15))]);
        assert_eq!(
            pieces,
            vec![
                range(Some(0), Some(4)),
                range(Some(5), Some(10)),
                range(Some(11), Some(15)),
            ]
        );
    }

    #[test]
    fn literal_inside_range_becomes_its_own_piece() {
        let pieces = split(&[Constructor::IntLiteral(5), range(Some(0), Some(10))]);
        assert_eq!(
            pieces,
            vec![
                range(Some(0), Some(4)),
                Constructor::IntLiteral(5),
                range(Some(6), Some(10)),
            ]
        );
    }

    #[test]
    fn open_ends_stay_open_and_gaps_are_dropped() {
        let pieces = split(&[range(None, Some(-1)), range(Some(10), None)]);
        assert_eq!(pieces, vec![range(None, Some(-1)), range(Some(10), None)]);
    }

    #[test]
    fn empty_range_contributes_nothing() {
        assert!(split(&[range(Some(10), Some(0))]).is_empty());
    }

    #[test]
    fn char_pieces_skip_the_surrogate_block() {
        let lo = Constructor::CharRange {
            start: Some('a'),
            end: None,
        };
        let hi = Constructor::CharLiteral('\u{E000}');
        let pieces = split(&[lo, hi]);
        assert_eq!(
            pieces,
            vec![
                Constructor::CharRange {
                    start: Some('a'),
                    end: Some('\u{D7FF}'),
                },
                Constructor::CharLiteral('\u{E000}'),
                Constructor::CharRange {
                    start: Some('\u{E001}'),
                    end: None,
                },
            ]
        );
    }

    #[test]
    fn array_lengths_split_into_exact_and_one_open_piece() {
        let array = |p, s, r| Constructor::Array {
            prefix_len: p,
            suffix_len: s,
            has_rest: r,
        };
        // `[x, y, ..]`, `[x, ..]`, `[a, b, c]`, `[.., z]`
        let pieces = split(&[
            array(2, 0, true),
            array(1, 0, true),
            array(3, 0, false),
            array(0, 1, true),
        ]);
        assert_eq!(
            pieces,
            vec![
                array(1, 0, false),
                array(2, 0, false),
                array(3, 0, false),
                array(3, 1, true),
            ]
        );
    }
}
