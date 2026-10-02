//! Segments straight from the packed encoding, without producing the path text.
//!
//! The lexer accepts only token streams on which the text grammar would read
//! the same arguments; anything else is declined, and the caller parses the
//! decoded text instead, so results (and error offsets) always match the text
//! parser.

use super::Segment;
use super::codec::{Item, Items, Number, POW10, Token};
use super::parse::{Lex, Next, run_with};
use alloc::vec::Vec;

/// Parsed segments, or `None` when the text parser has to decide.
pub(crate) fn segments(bytes: &[u8]) -> Option<Vec<Segment>> {
    let mut lex = Packed {
        items: Items::new(bytes),
        pending: None,
        first_arg: false,
        after_number: false,
        done: false,
    };
    run_with(&mut lex).ok()
}

struct Packed<'a> {
    items: Items<'a>,
    pending: Option<Item<'a>>,
    /// The next argument directly follows a command letter, where a comma
    /// separator is invalid.
    first_arg: bool,
    after_number: bool,
    done: bool,
}

fn blank(sep: &str) -> bool {
    sep.bytes().all(|b| b == b' ')
}

impl<'a> Packed<'a> {
    fn peek(&mut self) -> Option<Item<'a>> {
        if self.pending.is_none() && !self.done {
            self.pending = self.items.next();
            // `Items` resumes after an invalid opcode; the text ends there.
            self.done = self.pending.is_none();
        }
        self.pending
    }

    /// Consumes the next token if it is a number the text grammar would read
    /// as that single argument.
    fn argument(&mut self) -> Option<Number<'a>> {
        let item = self.peek()?;
        let Token::Number(n) = item.token else {
            return None;
        };
        let comma = item.sep.contains(',');
        if (self.first_arg && comma) || (self.after_number && item.sep.is_empty() && !minus(n)) {
            return None;
        }
        self.pending = None;
        self.first_arg = false;
        self.after_number = true;
        Some(n)
    }
}

/// Whether the number text starts with `-`, the only start that cannot merge
/// into a preceding number without a separator.
fn minus(n: Number<'_>) -> bool {
    match n {
        Number::Scaled { n, .. } => n < 0,
        Number::NegZero => true,
        Number::Raw(t) => t.starts_with('-'),
    }
}

/// The value the text parser reads from the same digits: an integer over a
/// power of ten, both exact, divides to the correctly rounded quotient.
fn value(n: Number<'_>) -> Option<f64> {
    match n {
        Number::Scaled { n, places } => {
            let scale = *POW10.get(usize::from(places))?;
            Some(f64::from(n) / f64::from(scale))
        }
        Number::NegZero => Some(-0.0),
        Number::Raw(t) => {
            let plain = t
                .bytes()
                .all(|b| b.is_ascii_digit() || b".eE+-".contains(&b));
            if plain { t.parse().ok() } else { None }
        }
    }
}

fn flag_value(n: Number<'_>) -> Option<bool> {
    match n {
        Number::Scaled { n: 0, .. } => Some(false),
        Number::Scaled { n, places } => {
            let one = i64::from(*POW10.get(usize::from(places))?);
            (i64::from(n) == one).then_some(true)
        }
        Number::Raw("0") => Some(false),
        Number::Raw("1") => Some(true),
        _ => None,
    }
}

impl Lex for Packed<'_> {
    fn next_command(&mut self, first: bool) -> Next {
        let Some(item) = self.peek() else {
            return Next::End;
        };
        let letter = match item.token {
            Token::Command(c) if blank(item.sep) && (!first || matches!(c, b'M' | b'm')) => c,
            Token::Tail if blank(item.sep) => {
                self.done = true;
                self.pending = None;
                // Text after the end marker is still part of the path text.
                return if self.items.next().is_some() {
                    Next::Invalid
                } else {
                    Next::End
                };
            }
            _ => return Next::Invalid,
        };
        self.pending = None;
        self.first_arg = true;
        self.after_number = false;
        Next::Command(letter)
    }

    fn at_number(&mut self) -> bool {
        matches!(
            self.peek(),
            Some(Item {
                token: Token::Number(_),
                ..
            })
        )
    }

    fn number(&mut self) -> Option<f64> {
        value(self.argument()?)
    }

    fn flag(&mut self) -> Option<bool> {
        flag_value(self.argument()?)
    }
}
