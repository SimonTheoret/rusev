//! This library is  a re-implementation of the SeqEval library. SeqEval is built with python and
//! is too slow when handling a large amount of strings. This library hopes to fulfill the same
//! niche, but hopefully in a much more performant way.

use std::borrow::Cow;
use std::error::Error;
use std::fmt::Display;

#[derive(Debug, Hash, PartialEq, Clone)]
struct Entity<'a> {
    sent_id: usize,
    start: usize,
    end: usize,
    tag: Cow<'a, str>,
}

impl<'a> Display for Entity<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}, {}, {})",
            self.sent_id, self.tag, self.start, self.end
        )
    }
}

impl<'a> Entity<'a> {
    fn as_tuple(&'a self) -> (usize, usize, usize, &'a str) {
        (self.sent_id, self.start, self.end, self.tag.as_ref())
    }
}

#[derive(Debug, PartialEq, Hash, Clone)]
pub enum Prefix {
    I,
    O,
    B,
    E,
    S,
    U,
    L,
    Any,
}
#[derive(Debug, PartialEq, Hash, Clone)]
pub enum Tag {
    Same,
    Diff,
    Any,
}

#[derive(Debug, PartialEq, Hash)]
pub struct Token<'a> {
    token: Cow<'a, str>,
    prefix: Prefix,
    tag: Cow<'a, str>,
    allowed_prefix: Option<Vec<Prefix>>,
}

impl<'a> Token<'a> {
    /// Check whether the prefix is allowed or not
    fn is_valid(&self) -> Result<bool, InvalidTokenError> {
        match &self.allowed_prefix {
            None => Err(InvalidTokenError::from(self)),
            Some(vec_of_allowed_prefixes) => {
                let prefix_is_allowed = vec_of_allowed_prefixes.contains(&self.prefix);
                if prefix_is_allowed {
                    Ok(true)
                } else {
                    Err(InvalidTokenError::from(self))
                }
            }
        }
    }
    fn get_token_ref(&'a self) -> &'a str {
        &self.token
    }
    fn get_token_owned(&'a self) -> String {
        match &self.token {
            Cow::Owned(owned_string) => owned_string.clone(),
            Cow::Borrowed(borrowed_string) => borrowed_string.to_string(),
        }
    }
    fn get_allowed_prefixes_ref(&'a self) -> &Option<Vec<Prefix>> {
        &self.allowed_prefix
    }
    fn get_allowed_prefixes_owned(&'a self) -> Option<Vec<Prefix>> {
        self.allowed_prefix.clone()
    }
    fn is_start(&self, prev: &Token) -> bool {
        todo!()
    }
    fn is_inside(&self, prev: &Token) -> bool {
        todo!()
    }
    fn is_end(&self, prev: &Token) -> bool {
        todo!()
    }
    fn check_tag(&self, prev: &Token, cond: Tag) -> bool {
        match cond {
            Tag::Any => true,
            Tag::Same if prev.tag == self.tag => true,
            Tag::Diff if prev.tag != self.tag => true,
            _ => false,
        }
    }
    /// """Check whether the prefix patterns are matched."""
    ///
    /// * `prev`: Previous token
    /// * `patterns`: Patterns to match the token against
    fn check_patterns(&self, prev: &Token, outer: TokenWithPatterns, pattern_to_check: Pattern) -> bool {
        let pattern = (self.prefix, prev.prefix,  );
    }
}

#[derive(Debug, Hash)]
struct InvalidTokenError(String, Option<Vec<Prefix>>);

impl<'a> From<&Token<'a>> for InvalidTokenError {
    fn from(value: &Token<'a>) -> Self {
        InvalidTokenError(value.get_token_owned(), value.get_allowed_prefixes_owned())
    }
}

impl Display for InvalidTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The current token ({}) is not allowed. Only the following tokens are allowd: {:?}",
            self.0, self.1
        )
    }
}

impl Error for InvalidTokenError {}

enum Pattern {
    Start,
    Inside,
    End,
}

enum TokenWithPatterns<'a> {
    IOB1 { token: Token<'a> },
    IOE1 { token: Token<'a> },
    IOB2 { token: Token<'a> },
    IOE2 { token: Token<'a> },
    IOBES { token: Token<'a> },
    BILOU { token: Token<'a> },
}

impl<'a> TokenWithPatterns<'a> {
    const IOB1_ALLOWED_PREFIXES: [Prefix; 3] = [Prefix::I, Prefix::O, Prefix::B];
    const IOB1_START_PATTERNS: [(Prefix, Prefix, Tag); 5] = [
        (Prefix::O, Prefix::I, Tag::Any),
        (Prefix::I, Prefix::I, Tag::Diff),
        (Prefix::B, Prefix::I, Tag::Any),
        (Prefix::I, Prefix::B, Tag::Same),
        (Prefix::B, Prefix::B, Tag::Same),
    ];
    const IOB1_INSIDE_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::B, Prefix::I, Tag::Same),
        (Prefix::I, Prefix::I, Tag::Same),
    ];
    const IOB1_END_PATTERNS: [(Prefix, Prefix, Tag); 6] = [
        (Prefix::I, Prefix::I, Tag::Diff),
        (Prefix::I, Prefix::O, Tag::Any),
        (Prefix::I, Prefix::B, Tag::Any),
        (Prefix::B, Prefix::O, Tag::Any),
        (Prefix::B, Prefix::I, Tag::Diff),
        (Prefix::B, Prefix::B, Tag::Same),
    ];
    const IOE1_ALLOWED_PREFIXES: [Prefix; 3] = [Prefix::I, Prefix::O, Prefix::E];
    const IOE1_START_PATTERNS: [(Prefix, Prefix, Tag); 4] = [
        (Prefix::O, Prefix::I, Tag::Any),
        (Prefix::I, Prefix::I, Tag::Diff),
        (Prefix::E, Prefix::I, Tag::Any),
        (Prefix::E, Prefix::E, Tag::Same),
    ];
    const IOE1_INSIDE_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::I, Prefix::I, Tag::Same),
        (Prefix::I, Prefix::E, Tag::Same),
    ];
    const IOE1_END_PATTERNS: [(Prefix, Prefix, Tag); 5] = [
        (Prefix::I, Prefix::I, Tag::Diff),
        (Prefix::I, Prefix::O, Tag::Any),
        (Prefix::I, Prefix::E, Tag::Diff),
        (Prefix::E, Prefix::I, Tag::Same),
        (Prefix::E, Prefix::E, Tag::Same),
    ];

    const IOB2_ALLOWED_PREFIXES: [Prefix; 3] = [Prefix::I, Prefix::O, Prefix::B];
    const IOB2_START_PATTERNS: [(Prefix, Prefix, Tag); 1] = [(Prefix::Any, Prefix::I, Tag::Any)];
    const IOB2_INSIDE_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::I, Prefix::I, Tag::Same),
        (Prefix::I, Prefix::E, Tag::Same),
    ];
    const IOB2_END_PATTERNS: [(Prefix, Prefix, Tag); 6] = [
        (Prefix::I, Prefix::O, Tag::Any),
        (Prefix::I, Prefix::I, Tag::Diff),
        (Prefix::I, Prefix::B, Tag::Any),
        (Prefix::B, Prefix::O, Tag::Any),
        (Prefix::B, Prefix::I, Tag::Diff),
        (Prefix::B, Prefix::B, Tag::Any),
    ];

    const IOE2_ALLOWED_PREFIXES: [Prefix; 3] = [Prefix::I, Prefix::O, Prefix::E];
    const IOE2_START_PATTERNS: [(Prefix, Prefix, Tag); 6] = [
        (Prefix::O, Prefix::I, Tag::Any),
        (Prefix::O, Prefix::E, Tag::Any),
        (Prefix::E, Prefix::I, Tag::Any),
        (Prefix::E, Prefix::E, Tag::Any),
        (Prefix::I, Prefix::I, Tag::Diff),
        (Prefix::I, Prefix::E, Tag::Diff),
    ];
    const IOE2_INSIDE_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::I, Prefix::E, Tag::Same),
        (Prefix::I, Prefix::I, Tag::Same),
    ];
    const IOE2_END_PATTERNS: [(Prefix, Prefix, Tag); 1] = [(Prefix::E, Prefix::Any, Tag::Any)];

    const IOBES_ALLOWED_PREFIXES: [Prefix; 5] =
        [Prefix::I, Prefix::O, Prefix::E, Prefix::B, Prefix::S];
    const IOBES_START_PATTERNS: [(Prefix, Prefix, Tag); 4] = [
        (Prefix::B, Prefix::I, Tag::Same),
        (Prefix::B, Prefix::E, Tag::Same),
        (Prefix::I, Prefix::I, Tag::Same),
        (Prefix::I, Prefix::E, Tag::Same),
    ];
    const IOBES_INSIDE_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::S, Prefix::Any, Tag::Any),
        (Prefix::E, Prefix::Any, Tag::Any),
    ];
    const IOBES_END_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::S, Prefix::Any, Tag::Any),
        (Prefix::E, Prefix::Any, Tag::Any),
    ];

    const BILOU_ALLOWED_PREFIXES: [Prefix; 5] =
        [Prefix::I, Prefix::O, Prefix::U, Prefix::B, Prefix::O];
    const BILOU_START_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::Any, Prefix::B, Tag::Any),
        (Prefix::Any, Prefix::U, Tag::Any),
    ];
    const BILOU_INSIDE_PATTERNS: [(Prefix, Prefix, Tag); 4] = [
        (Prefix::B, Prefix::I, Tag::Same),
        (Prefix::B, Prefix::L, Tag::Same),
        (Prefix::I, Prefix::I, Tag::Same),
        (Prefix::I, Prefix::L, Tag::Same),
    ];
    const BILOU_END_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::U, Prefix::Any, Tag::Any),
        (Prefix::L, Prefix::Any, Tag::Any),
    ];
    fn allowed_prefixes<'b>(&'a self) -> &'a [Prefix] {
        match self {
            Self::IOB1 { .. } => &Self::IOB1_ALLOWED_PREFIXES,
            Self::IOE1 { .. } => &Self::IOE1_ALLOWED_PREFIXES,
            Self::IOB2 { .. } => &Self::IOB2_ALLOWED_PREFIXES,
            Self::IOE2 { .. } => &Self::IOE2_ALLOWED_PREFIXES,
            Self::IOBES { .. } => &Self::IOBES_ALLOWED_PREFIXES,
            Self::BILOU { .. } => &Self::BILOU_ALLOWED_PREFIXES,
        }
    }
    fn start_patterns<'b>(&'a self) -> &'a [(Prefix, Prefix, Tag)] {
        match self {
            Self::IOB1 { .. } => &Self::IOB1_START_PATTERNS,
            Self::IOE1 { .. } => &Self::IOE1_START_PATTERNS,
            Self::IOB2 { .. } => &Self::IOB2_START_PATTERNS,
            Self::IOE2 { .. } => &Self::IOE2_START_PATTERNS,
            Self::IOBES { .. } => &Self::IOBES_START_PATTERNS,
            Self::BILOU { .. } => &Self::BILOU_START_PATTERNS,
        }
    }
    fn inside_patterns<'b>(&'a self) -> &'a [(Prefix, Prefix, Tag)] {
        match self {
            Self::IOB1 { .. } => &Self::IOB1_INSIDE_PATTERNS,
            Self::IOE1 { .. } => &Self::IOE1_INSIDE_PATTERNS,
            Self::IOB2 { .. } => &Self::IOB2_INSIDE_PATTERNS,
            Self::IOE2 { .. } => &Self::IOE2_INSIDE_PATTERNS,
            Self::IOBES { .. } => &Self::IOBES_INSIDE_PATTERNS,
            Self::BILOU { .. } => &Self::BILOU_INSIDE_PATTERNS,
        }
    }
    fn end_patterns<'b>(&'a self) -> &'a [(Prefix, Prefix, Tag)] {
        match self {
            Self::IOB1 { .. } => &Self::IOB1_END_PATTERNS,
            Self::IOE1 { .. } => &Self::IOE1_END_PATTERNS,
            Self::IOB2 { .. } => &Self::IOB2_END_PATTERNS,
            Self::IOE2 { .. } => &Self::IOE2_END_PATTERNS,
            Self::IOBES { .. } => &Self::IOBES_END_PATTERNS,
            Self::BILOU { .. } => &Self::BILOU_END_PATTERNS,
        }
    }
}

struct Tokens
