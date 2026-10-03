//! Quoting and escaping for SNBT strings and compound keys.

use std::borrow::Cow;
use std::fmt::Write;

/// Quotes and escapes a string using vanilla `StringTag.quoteAndEscape`.
#[must_use]
pub fn quote_and_escape(input: &str) -> String {
    let quote = input
        .chars()
        .find(|c| matches!(c, '\'' | '"'))
        .map_or('"', |c| if c == '"' { '\'' } else { '"' });
    let mut result = String::with_capacity(input.len() + 2);
    result.push(quote);
    for c in input.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '\u{8}' => result.push_str("\\b"),
            '\t' => result.push_str("\\t"),
            '\n' => result.push_str("\\n"),
            '\u{c}' => result.push_str("\\f"),
            '\r' => result.push_str("\\r"),
            c if c < ' ' => {
                // Formatting an integer into a String cannot fail.
                let _ = write!(result, "\\x{:02X}", u32::from(c));
            }
            c => {
                if c == quote {
                    result.push('\\');
                }
                result.push(c);
            }
        }
    }
    result.push(quote);
    result
}

/// Leaves simple compound keys unquoted and escapes all other keys.
#[must_use]
pub fn handle_escape_pretty(input: &str) -> Cow<'_, str> {
    if !input.is_empty()
        && input
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'+' | b'-'))
    {
        Cow::Borrowed(input)
    } else {
        Cow::Owned(quote_and_escape(input))
    }
}
