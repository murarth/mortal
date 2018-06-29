//! Miscellaneous utility functions

use std::str::CharIndices;

/// Returns the width of a character in the terminal.
///
/// Returns `None` or `Some(0)` for control characters.
#[inline]
pub fn char_width(ch: char) -> Option<usize> {
    use unicode_width::UnicodeWidthChar;

    ch.width()
}

/// Returns whether the given character is a combining mark.
#[inline]
pub fn is_combining_mark(ch: char) -> bool {
    use unicode_normalization::char::is_combining_mark;

    is_combining_mark(ch)
}

const CTRL_MASK: u8 = 0x1f;
const UNCTRL_BIT: u8 = 0x40;

/// Returns the control character corresponding to the given character.
///
/// # Examples
///
/// ```
/// # use mortal::util::ctrl;
/// // Ctrl-C
/// assert_eq!(ctrl('c'), '\x03');
/// ```
#[inline]
pub fn ctrl(ch: char) -> char {
    ((ch as u8) & CTRL_MASK) as char
}

/// Returns whether the given character is a control character.
///
/// Control characters are in the range `'\0'` ... `'\x1f'`, inclusive.
#[inline]
pub fn is_ctrl(ch: char) -> bool {
    let ch = ch as u32;
    ch & (CTRL_MASK as u32) == ch
}

/// Returns the ASCII character corresponding to the given control character.
///
/// If `ch` is not a control character, the result is unspecified.
///
/// # Examples
///
/// ```
/// # use mortal::util::unctrl_upper;
/// // Ctrl-C
/// assert_eq!(unctrl_upper('\x03'), 'C');
/// ```
#[inline]
pub fn unctrl_upper(ch: char) -> char {
    ((ch as u8) | UNCTRL_BIT) as char
}

/// Returns the lowercase ASCII character corresponding to the given control character.
///
/// If `ch` is not a control character, the result is unspecified.
///
/// # Examples
///
/// ```
/// # use mortal::util::unctrl_lower;
///
/// // Ctrl-C
/// assert_eq!(unctrl_lower('\x03'), 'c');
/// ```
#[inline]
pub fn unctrl_lower(ch: char) -> char {
    unctrl_upper(ch).to_ascii_lowercase()
}

/// Iterator over string prefixes.
///
/// An instance of this type is returned by the free function [`prefixes`].
///
/// [`prefixes`]: fn.prefixes.html
pub struct Prefixes<'a> {
    s: &'a str,
    iter: CharIndices<'a>,
}

/// Returns an iterator over all non-empty prefixes of `s`, beginning with
/// the shortest.
///
/// If `s` is an empty string, the iterator will yield no elements.
///
/// # Examples
///
/// ```
/// # use mortal::util::prefixes;
/// let mut pfxs = prefixes("foo");
///
/// assert_eq!(pfxs.next(), Some("f"));
/// assert_eq!(pfxs.next(), Some("fo"));
/// assert_eq!(pfxs.next(), Some("foo"));
/// assert_eq!(pfxs.next(), None);
/// ```
#[inline]
pub fn prefixes(s: &str) -> Prefixes {
    Prefixes{
        s,
        iter: s.char_indices(),
    }
}

impl<'a> Iterator for Prefixes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        self.iter.next().map(|(idx, ch)| &self.s[..idx + ch.len_utf8()])
    }
}

#[cfg(test)]
mod test {
    use super::{ctrl, is_ctrl, unctrl_lower, unctrl_upper, prefixes};

    #[test]
    fn test_unctrl() {
        for ch in 0u8..255 {
            let ch = ch as char;

            if is_ctrl(ch) {
                assert_eq!(ch, ctrl(unctrl_lower(ch)));
                assert_eq!(ch, ctrl(unctrl_upper(ch)));
            }
        }
    }

    #[test]
    fn test_prefix_iter() {
        let mut pfxs = prefixes("foobar");

        assert_eq!(pfxs.next(), Some("f"));
        assert_eq!(pfxs.next(), Some("fo"));
        assert_eq!(pfxs.next(), Some("foo"));
        assert_eq!(pfxs.next(), Some("foob"));
        assert_eq!(pfxs.next(), Some("fooba"));
        assert_eq!(pfxs.next(), Some("foobar"));
        assert_eq!(pfxs.next(), None);

        let mut pfxs = prefixes("a");

        assert_eq!(pfxs.next(), Some("a"));
        assert_eq!(pfxs.next(), None);

        let mut pfxs = prefixes("");

        assert_eq!(pfxs.next(), None);
    }
}
