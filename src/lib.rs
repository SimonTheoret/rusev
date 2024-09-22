//! This library is  a re-implementation of the SeqEval library. SeqEval is built with python and
//! is too slow when handling a large amount of strings. This library hopes to fulfill the same
//! niche, but hopefully in a much more performant way.

use std::borrow::Cow;
use std::error::Error;
use std::fmt::Display;
use unicode_segmentation::UnicodeSegmentation;

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
enum Prefix {
    I,
    O,
    B,
    E,
    S,
    U,
    L,
    ANY,
}

#[derive(Debug, Clone)]
struct ParsingPrefixError<S: AsRef<str>>(S);

impl<S: AsRef<str> + Error> Display for ParsingPrefixError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = self.0.as_ref();
        write!(
            f,
            "Could not parse the following string into a Prefix: {}",
            content
        )
    }
}
impl<S: AsRef<str> + Error> Error for ParsingPrefixError<S> {}

impl<'a> TryFrom<&'a str> for Prefix {
    type Error = ParsingPrefixError<&'a str>;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "I" => Ok(Prefix::I),
            "O" => Ok(Prefix::O),
            "B" => Ok(Prefix::B),
            "E" => Ok(Prefix::E),
            "S" => Ok(Prefix::S),
            "U" => Ok(Prefix::U),
            "L" => Ok(Prefix::L),
            "ANY" => Ok(Prefix::ANY),
            _ => Err(ParsingPrefixError(value)),
        }
    }
}
impl TryFrom<String> for Prefix {
    type Error = ParsingPrefixError<String>;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let ref_val = value.as_ref();
        match ref_val {
            "I" => Ok(Prefix::I),
            "O" => Ok(Prefix::O),
            "B" => Ok(Prefix::B),
            "E" => Ok(Prefix::E),
            "S" => Ok(Prefix::S),
            "U" => Ok(Prefix::U),
            "L" => Ok(Prefix::L),
            "ANY" => Ok(Prefix::ANY),
            _ => Err(ParsingPrefixError(value)),
        }
    }
}

#[derive(Debug, PartialEq, Hash, Clone)]
enum Tag {
    Same,
    Diff,
    Any,
}

#[derive(Debug, PartialEq, Hash)]
struct InnerToken<'a> {
    token: Cow<'a, str>,
    prefix: Prefix,
    tag: Cow<'a, str>,
    allowed_prefix: Option<Vec<Prefix>>,
}

// TODO: Move this enum into its own module, as to hide its `new` function.
///
/// This enum represents the positon of the Prefix in a token (a Cow<'_, str>).
enum UnicodeIndex {
    Start(usize),
    End(usize),
}
impl UnicodeIndex {
    pub(crate) fn new<I: Iterator>(suffix: bool, unicode_iterator: I) -> Self {
        if suffix {
            UnicodeIndex::Start(0)
        } else {
            UnicodeIndex::End(unicode_iterator.count())
        }
    }
    pub(crate) fn to_index(&self) -> usize {
        match self {
            Self::Start(start) => *start,
            Self::End(end) => *end,
        }
    }
}

impl<'a> InnerToken<'a> {
    fn new<S: AsRef<str>>(
        token: Cow<'a, str>,
        suffix: bool,
        delimiter: Cow<'a, str>,
    ) -> Result<Self, ParsingPrefixError<&'a str>> {
        let ref_iter = token.graphemes(true);
        let unicode_index = UnicodeIndex::new(suffix, ref_iter);
        let (char_index, prefix_char) = token
            .grapheme_indices(true)
            .nth(unicode_index.to_index())
            .ok_or(ParsingPrefixError("None"))?;
        let prefix = Prefix::try_from(prefix_char)?;
        let tag_before_strip = match unicode_index {
            UnicodeIndex::Start(_) => &token[char_index..],
            UnicodeIndex::End(_) => &token[..char_index],
        };
        let tag = {

        }

    }

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
    #[inline]
    fn check_tag(&self, prev: &InnerToken, cond: &Tag) -> bool {
        match cond {
            Tag::Any => true,
            Tag::Same if prev.tag == self.tag => true,
            Tag::Diff if prev.tag != self.tag => true,
            _ => false,
        }
    }
    /// Check whether the prefix patterns are matched.
    ///
    /// * `prev`: Previous token
    /// * `patterns`: Patterns to match the token against
    fn check_patterns(
        &self,
        prev: &InnerToken,
        patterns_to_check: &[(Prefix, Prefix, Tag)],
    ) -> bool {
        for (prev_prefix, current_prefix, tag_cond) in patterns_to_check {
            if prev_prefix == &prev.prefix
                && current_prefix == &self.prefix
                && self.check_tag(prev, tag_cond)
            {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Hash)]
struct InvalidTokenError(String, Option<Vec<Prefix>>);

impl<'a> From<&InnerToken<'a>> for InvalidTokenError {
    fn from(value: &InnerToken<'a>) -> Self {
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

enum SchemeType {
    IOB1,
    IOE1,
    IOB2,
    IOE2,
    IOBES,
    BILOU,
}

enum Token<'a> {
    IOB1 { token: InnerToken<'a> },
    IOE1 { token: InnerToken<'a> },
    IOB2 { token: InnerToken<'a> },
    IOE2 { token: InnerToken<'a> },
    IOBES { token: InnerToken<'a> },
    BILOU { token: InnerToken<'a> },
}

impl<'a> Token<'a> {
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
    const IOB2_START_PATTERNS: [(Prefix, Prefix, Tag); 1] = [(Prefix::ANY, Prefix::I, Tag::Any)];
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
    const IOE2_END_PATTERNS: [(Prefix, Prefix, Tag); 1] = [(Prefix::E, Prefix::ANY, Tag::Any)];

    const IOBES_ALLOWED_PREFIXES: [Prefix; 5] =
        [Prefix::I, Prefix::O, Prefix::E, Prefix::B, Prefix::S];
    const IOBES_START_PATTERNS: [(Prefix, Prefix, Tag); 4] = [
        (Prefix::B, Prefix::I, Tag::Same),
        (Prefix::B, Prefix::E, Tag::Same),
        (Prefix::I, Prefix::I, Tag::Same),
        (Prefix::I, Prefix::E, Tag::Same),
    ];
    const IOBES_INSIDE_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::S, Prefix::ANY, Tag::Any),
        (Prefix::E, Prefix::ANY, Tag::Any),
    ];
    const IOBES_END_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::S, Prefix::ANY, Tag::Any),
        (Prefix::E, Prefix::ANY, Tag::Any),
    ];

    const BILOU_ALLOWED_PREFIXES: [Prefix; 5] =
        [Prefix::I, Prefix::O, Prefix::U, Prefix::B, Prefix::O];
    const BILOU_START_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::ANY, Prefix::B, Tag::Any),
        (Prefix::ANY, Prefix::U, Tag::Any),
    ];
    const BILOU_INSIDE_PATTERNS: [(Prefix, Prefix, Tag); 4] = [
        (Prefix::B, Prefix::I, Tag::Same),
        (Prefix::B, Prefix::L, Tag::Same),
        (Prefix::I, Prefix::I, Tag::Same),
        (Prefix::I, Prefix::L, Tag::Same),
    ];
    const BILOU_END_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::U, Prefix::ANY, Tag::Any),
        (Prefix::L, Prefix::ANY, Tag::Any),
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
    fn new<'b: 'a>(scheme: SchemeType, token: InnerToken<'b>) -> Self {
        match scheme {
            SchemeType::IOB1 => Token::IOB1 { token },
            SchemeType::IOB2 => Token::IOB2 { token },
            SchemeType::IOE1 => Token::IOE1 { token },
            SchemeType::IOE2 => Token::IOE2 { token },
            SchemeType::IOBES => Token::IOBES { token },
            SchemeType::BILOU => Token::BILOU { token },
        }
    }

    fn is_start(&self, prev: &InnerToken) -> bool {
        match self {
            Self::IOB1 { token } => token.check_patterns(prev, self.start_patterns()),
            Self::IOB2 { token } => token.check_patterns(prev, self.start_patterns()),
            Self::IOE1 { token } => token.check_patterns(prev, self.start_patterns()),
            Self::IOE2 { token } => token.check_patterns(prev, self.start_patterns()),
            Self::IOBES { token } => token.check_patterns(prev, self.start_patterns()),
            Self::BILOU { token } => token.check_patterns(prev, self.start_patterns()),
        }
    }
    fn is_inside(&self, prev: &InnerToken) -> bool {
        match self {
            Self::IOB1 { token } => token.check_patterns(prev, self.inside_patterns()),
            Self::IOB2 { token } => token.check_patterns(prev, self.inside_patterns()),
            Self::IOE1 { token } => token.check_patterns(prev, self.inside_patterns()),
            Self::IOE2 { token } => token.check_patterns(prev, self.inside_patterns()),
            Self::IOBES { token } => token.check_patterns(prev, self.inside_patterns()),
            Self::BILOU { token } => token.check_patterns(prev, self.inside_patterns()),
        }
    }
    fn is_end(&self, prev: &InnerToken) -> bool {
        match self {
            Self::IOB1 { token } => token.check_patterns(prev, self.end_patterns()),
            Self::IOB2 { token } => token.check_patterns(prev, self.end_patterns()),
            Self::IOE1 { token } => token.check_patterns(prev, self.end_patterns()),
            Self::IOE2 { token } => token.check_patterns(prev, self.end_patterns()),
            Self::IOBES { token } => token.check_patterns(prev, self.end_patterns()),
            Self::BILOU { token } => token.check_patterns(prev, self.end_patterns()),
        }
    }
}

struct Tokens<'a> {
    tokens: Vec<Cow<'a, str>>,
    extended_tokens: Vec<Token<'a>>,
    sent_id: Option<usize>,
}
impl<'a> Tokens<'a> {
    fn new(
        tokens: Vec<Cow<'a, str>>,
        scheme: SchemeType,
        delimiter: Cow<'a, str>,
        sent_id: Option<usize>,
    ) -> Self {
        // let inner_token_prefix =
        // let outside_token_inner_token = Token{token: Cow::Borrowed("O"), };
        let outside_token = Token::new(scheme, "O");
    }
}
