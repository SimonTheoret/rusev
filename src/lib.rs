//! This library is  a re-implementation of the SeqEval library. SeqEval is built with python and
//! is too slow when handling a large amount of strings. This library hopes to fulfill the same
//! niche, but hopefully in a much more performant way.
//! # SCHEMES
//! The current schemes are supported:
//! - IOB1: Here, I is a token inside a chunk, O is a token outside a chunk and B is the beginning
//! of chunk immediately following another chunk of the same Named Entity.
//! - IOB2: It is same as IOB1, except that a B tag is given for every token, which exists at the
//! beginning of the chunk.
//! - IOE1: An E tag used to mark the last token of a chunk immediately preceding another chunk of
//! the same named entity.
//! - IOE2: It is same as IOE1, except that an E tag is given for every token, which exists at the
//! end of the chunk.
//! - BILOU/IOBES: 'E' and 'L' denotes Last or Ending character in a sequence and 'S' denotes a single
//! element  and 'U' a unit element.
//! # NOTE ON B-TAG
//! The B-prefix before a tag indicates that the tag is the beginning of a chunk that immediately
//! follows another chunk of the same type without O tags between them. It is used only in that
//! case: when a chunk comes after an O tag, the first token of the chunk takes the I- prefix.

use std::borrow::Cow;
use std::cell::RefCell;
use std::error::Error;
use std::fmt::Display;
use std::mem::take;
use std::str::FromStr;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Hash, PartialEq, Clone)]
struct Entity<'a> {
    sent_id: Option<usize>,
    start: usize,
    end: usize,
    tag: Cow<'a, str>,
}

impl<'a> Display for Entity<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:?}, {}, {}, {})",
            self.sent_id, self.tag, self.start, self.end
        )
    }
}

impl<'a> Entity<'a> {
    fn as_tuple(&'a self) -> (Option<usize>, usize, usize, &'a str) {
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

impl FromStr for Prefix {
    type Err = ParsingPrefixError<&'static str>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from_with_static_error(s)
    }
}

impl<'a> Prefix {
    fn try_from_with_static_error(
        value: &'a str,
    ) -> Result<Self, ParsingPrefixError<&'static str>> {
        match value {
            "I" => Ok(Prefix::I),
            "O" => Ok(Prefix::O),
            "B" => Ok(Prefix::B),
            "E" => Ok(Prefix::E),
            "S" => Ok(Prefix::S),
            "U" => Ok(Prefix::U),
            "L" => Ok(Prefix::L),
            "ANY" => Ok(Prefix::ANY),
            _ => Err(ParsingPrefixError(String::from(value).leak())),
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

#[derive(Debug, PartialEq, Hash, Clone)]
struct InnerToken<'a> {
    token: Cow<'a, str>,
    prefix: Prefix,
    tag: Cow<'a, str>,
}

impl<'a> Default for InnerToken<'a> {
    fn default() -> Self {
        InnerToken {
            token: Cow::Borrowed(""),
            prefix: Prefix::I,
            tag: Cow::Borrowed(""),
        }
    }
}

// TODO: Move this enum into its own module, as to hide its `new` function.
///
/// This enum represents the positon of the Prefix in a token (a Cow<'_, str>).
enum UnicodeIndex {
    /// This variant indicates that the prefix is located at the start of the token
    Start(usize),
    /// This variant indicates that the prefix is located at the end of the token
    End(usize),
}
impl UnicodeIndex {
    fn new<I: Iterator>(suffix: bool, unicode_iterator: I) -> Self {
        if !suffix {
            UnicodeIndex::Start(0)
        } else {
            UnicodeIndex::End(unicode_iterator.count())
        }
    }
    fn to_index(&self) -> usize {
        match self {
            Self::Start(start) => *start,
            Self::End(end) => *end,
        }
    }
}

impl<'a> InnerToken<'a> {
    /// Create InnerToken
    ///
    /// * `token`: str or String to parse the InnerToken from
    /// * `suffix`: Marker indicating if prefix is located at the end (when suffix is true) or the
    /// end (when suffix is false) of the token
    /// * `delimiter`: Indicates the char used to separate the Prefix from the rest of the tag
    fn new(
        token: Cow<'a, str>,
        suffix: bool,
        delimiter: char,
    ) -> Result<Self, ParsingPrefixError<&'a str>> {
        let ref_iter = token.graphemes(true);
        let unicode_index = UnicodeIndex::new(suffix, ref_iter);
        let (char_index, prefix_char) = token
            .grapheme_indices(true)
            .nth(unicode_index.to_index())
            .ok_or(ParsingPrefixError("None"))?;
        let prefix = Prefix::try_from_with_static_error(prefix_char)?;
        let tag_before_strip = match unicode_index {
            UnicodeIndex::Start(_) => &token[char_index + 1..],
            UnicodeIndex::End(_) => &token[..char_index],
        };
        let tag = Cow::Owned(String::from(tag_before_strip.trim_matches(delimiter)));
        Ok(Self { token, prefix, tag })
    }

    /// Check whether the prefix is allowed or not
    fn get_token_ref(&'a self) -> &'a str {
        &self.token
    }
    fn get_token_owned(&'a self) -> String {
        match &self.token {
            Cow::Owned(owned_string) => owned_string.clone(),
            Cow::Borrowed(borrowed_string) => borrowed_string.to_string(),
        }
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

// #[derive(Debug, Hash)]
// struct InvalidTokenError(String, Option<Vec<Prefix>>);
//
// impl<'a> From<&InnerToken<'a>> for InvalidTokenError {
//     fn from(value: &InnerToken<'a>) -> Self {
//         InvalidTokenError(value.get_token_owned(), value.get_allowed_prefixes_owned())
//     }
// }
//
// impl Display for InvalidTokenError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "The current token ({}) is not allowed. Only the following tokens are allowd: {:?}",
//             self.0, self.1
//         )
//     }
// }
//
// impl Error for InvalidTokenError {}
//
#[derive(Debug, Clone, Copy)]
enum Pattern {
    Start,
    Inside,
    End,
}

#[derive(Debug, Clone, Copy)]
enum SchemeType {
    IOB1,
    IOE1,
    IOB2,
    IOE2,
    IOBES,
    BILOU,
}

#[derive(Debug, Clone, PartialEq)]
struct InvalidToken(String);

impl Display for InvalidToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid token: {}", self.0)
    }
}

impl Error for InvalidToken {}

#[derive(Debug, Clone, PartialEq)]
enum Token<'a> {
    IOB1 { token: InnerToken<'a> },
    IOE1 { token: InnerToken<'a> },
    IOB2 { token: InnerToken<'a> },
    IOE2 { token: InnerToken<'a> },
    IOBES { token: InnerToken<'a> },
    BILOU { token: InnerToken<'a> },
}
// impl<'a> Default for &'a mut Token<'a> {
//     fn default() -> Self {
//         // let token: InnerToken = InnerToken::default();
//         Token::BILOU{token: InnerToken::default()}
//     }
// }

impl<'a> Default for Token<'a> {
    fn default() -> Self {
        Token::IOB1 {
            token: InnerToken::default(),
        }
    }
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
    const IOB2_START_PATTERNS: [(Prefix, Prefix, Tag); 1] = [(Prefix::ANY, Prefix::B, Tag::Any)];
    const IOB2_INSIDE_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
        (Prefix::B, Prefix::I, Tag::Same),
        (Prefix::I, Prefix::I, Tag::Same),
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
    fn allowed_prefixes(&'a self) -> &'static [Prefix] {
        match self {
            Self::IOB1 { .. } => &Self::IOB1_ALLOWED_PREFIXES,
            Self::IOE1 { .. } => &Self::IOE1_ALLOWED_PREFIXES,
            Self::IOB2 { .. } => &Self::IOB2_ALLOWED_PREFIXES,
            Self::IOE2 { .. } => &Self::IOE2_ALLOWED_PREFIXES,
            Self::IOBES { .. } => &Self::IOBES_ALLOWED_PREFIXES,
            Self::BILOU { .. } => &Self::BILOU_ALLOWED_PREFIXES,
        }
    }
    fn start_patterns(&'a self) -> &'static [(Prefix, Prefix, Tag)] {
        match self {
            Self::IOB1 { .. } => &Self::IOB1_START_PATTERNS,
            Self::IOE1 { .. } => &Self::IOE1_START_PATTERNS,
            Self::IOB2 { .. } => &Self::IOB2_START_PATTERNS,
            Self::IOE2 { .. } => &Self::IOE2_START_PATTERNS,
            Self::IOBES { .. } => &Self::IOBES_START_PATTERNS,
            Self::BILOU { .. } => &Self::BILOU_START_PATTERNS,
        }
    }
    fn inside_patterns(&'a self) -> &'static [(Prefix, Prefix, Tag)] {
        match self {
            Self::IOB1 { .. } => &Self::IOB1_INSIDE_PATTERNS,
            Self::IOE1 { .. } => &Self::IOE1_INSIDE_PATTERNS,
            Self::IOB2 { .. } => &Self::IOB2_INSIDE_PATTERNS,
            Self::IOE2 { .. } => &Self::IOE2_INSIDE_PATTERNS,
            Self::IOBES { .. } => &Self::IOBES_INSIDE_PATTERNS,
            Self::BILOU { .. } => &Self::BILOU_INSIDE_PATTERNS,
        }
    }
    fn end_patterns(&'a self) -> &'static [(Prefix, Prefix, Tag)] {
        match self {
            Self::IOB1 { .. } => &Self::IOB1_END_PATTERNS,
            Self::IOE1 { .. } => &Self::IOE1_END_PATTERNS,
            Self::IOB2 { .. } => &Self::IOB2_END_PATTERNS,
            Self::IOE2 { .. } => &Self::IOE2_END_PATTERNS,
            Self::IOBES { .. } => &Self::IOBES_END_PATTERNS,
            Self::BILOU { .. } => &Self::BILOU_END_PATTERNS,
        }
    }
    fn new(scheme: SchemeType, token: InnerToken<'a>) -> Self {
        match scheme {
            SchemeType::IOB1 => Token::IOB1 { token },
            SchemeType::IOB2 => Token::IOB2 { token },
            SchemeType::IOE1 => Token::IOE1 { token },
            SchemeType::IOE2 => Token::IOE2 { token },
            SchemeType::IOBES => Token::IOBES { token },
            SchemeType::BILOU => Token::BILOU { token },
        }
    }

    fn inner(&self) -> &InnerToken {
        match self {
            Self::IOE1 { token } => &token,
            Self::IOE2 { token } => &token,
            Self::IOB1 { token } => &token,
            Self::IOB2 { token } => &token,
            Self::BILOU { token } => &token,
            Self::IOBES { token } => &token,
        }
    }
    fn inner_mut(&'a mut self) -> &mut InnerToken {
        match self {
            Self::IOE1 { token } => token,
            Self::IOE2 { token } => token,
            Self::IOB1 { token } => token,
            Self::IOB2 { token } => token,
            Self::BILOU { token } => token,
            Self::IOBES { token } => token,
        }
    }

    fn is_valid(&self) -> bool {
        self.allowed_prefixes().contains(&self.inner().prefix)
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
    fn take_tag(&mut self) -> Cow<'a, str> {
        match self {
            Self::IOB1 { token } => take(&mut token.tag),
            Self::IOE1 { token } => take(&mut token.tag),
            Self::IOB2 { token } => take(&mut token.tag),
            Self::IOE2 { token } => take(&mut token.tag),
            Self::IOBES { token } => take(&mut token.tag),
            Self::BILOU { token } => take(&mut token.tag),
        }
    }
}
/// This struct a struct capable of building efficiently the Tokens with a given outside_token.
/// This iterator avoids reallocation and keeps good ergonomic inside the `new` function of
/// `Tokens`.
struct ExtendedTokensIterator<'a> {
    outside_token: Token<'a>,
    tokens: Vec<Cow<'a, str>>,
    scheme: SchemeType,
    suffix: bool,
    delimiter: char,
    index: usize,
    /// Total length to iterate over. This length is equal to outside_token.len()
    total_len: usize,
}
impl<'a> ExtendedTokensIterator<'a> {
    /// The `PARTIAL_OFFSET` constant is the offset used during the iteration of the tokens
    /// attribute. This is due to the use of the outside_token.
    const PARTIAL_OFFSET: usize = 1;
}
impl<'a> Iterator for ExtendedTokensIterator<'a> {
    type Item = Result<Token<'a>, ParsingPrefixError<&'a str>>;
    fn next(&mut self) -> Option<Self::Item> {
        let ret: Option<Result<Token, ParsingPrefixError<&'a str>>>;
        if self.index > self.total_len {
            ret = None;
        } else if self.index == 0 {
            ret = Some(Ok(take(&mut self.outside_token)));
        } else {
            let cow_str = unsafe {
                take(
                    self.tokens
                        .get_unchecked_mut(self.index - Self::PARTIAL_OFFSET),
                )
            };
            let inner_token = InnerToken::new(cow_str, self.suffix, self.delimiter);
            ret = match inner_token {
                Err(msg) => Some(Err(msg)),
                Ok(res) => Some(Ok(Token::new(self.scheme, res))),
            };
        }
        self.index += 1;
        ret
    }
}
impl<'a> ExtendedTokensIterator<'a> {
    fn new(
        outside_token: Token<'a>,
        tokens: Vec<Cow<'a, str>>,
        scheme: SchemeType,
        suffix: bool,
        delimiter: char,
        sent_id: Option<usize>,
    ) -> Self {
        let total_len = tokens.len();
        Self {
            outside_token,
            tokens,
            scheme,
            suffix,
            delimiter,
            index: 0,
            total_len,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Tokens<'a> {
    extended_tokens: Vec<Token<'a>>,
    sent_id: Option<usize>,
}
impl<'a> Tokens<'a> {
    pub fn new(
        tokens: Vec<Cow<'a, str>>,
        scheme: SchemeType,
        suffix: bool,
        delimiter: char,
        sent_id: Option<usize>,
    ) -> Result<Self, ParsingPrefixError<&'a str>> {
        // let inner_token_prefix =
        // let outside_token_inner_token = Token{token: Cow::Borrowed("O"), };
        let inner_token = InnerToken::new(Cow::Borrowed("O"), suffix, delimiter)?;
        let outside_token = Token::new(scheme, inner_token);
        let tokens_iter =
            ExtendedTokensIterator::new(outside_token, tokens, scheme, suffix, delimiter, sent_id);
        let extended_tokens: Result<Vec<Token>, ParsingPrefixError<&str>> = tokens_iter.collect();
        match extended_tokens {
            Err(prefix_error) => Err(prefix_error),
            Ok(tokens) => Ok(Self {
                extended_tokens: tokens,
                sent_id,
            }),
        }
    }

    /// Returns the index of the last token inside the current chunk when given a `start` index and
    /// the previous token.
    ///
    /// * `start`: Indexing at which we are starting to look for a token not inside.
    /// * `prev`: Previous token. This token is necessary to know if the token at index `start` is
    /// inside or not.
    fn forward(&self, start: usize, prev: &Token<'a>) -> usize {
        let slice_of_interest = &self.extended_tokens()[start..];
        let mut swap_token = prev;
        for (i, current_token) in slice_of_interest.iter().enumerate() {
            if current_token.is_inside(swap_token.inner()) {
                swap_token = &current_token;
            } else {
                return i + start;
            }
        }
        return &self.extended_tokens.len() - 2;
    }

    fn is_end(&self, i: usize) -> bool {
        let token = &self.extended_tokens()[i];
        let prev = &self.extended_tokens()[i - 1];
        token.is_end(prev.inner())
    }

    fn extended_tokens(&'a self) -> &'a Vec<Token<'a>> {
        let res: &Vec<Token> = self.extended_tokens.as_ref();
        res
    }
}

/// Iterator and adaptor for iterating over the entities of a Tokens struct
///
/// * `index`: Index of the current iteration
/// * `current`: Current token
/// * `prev`:  Previous token
/// * `prev_prev`: Previous token of the previous token
struct EntitiesAdaptor<'a> {
    index: usize,
    tokens: Tokens<'a>,
    len: usize,
    // current: &'a mut Token<'a>,
    // prev: &'a mut Token<'a>,
}
impl<'a> Iterator for EntitiesAdaptor<'a> {
    type Item = Option<Result<Entity<'a>, Box<dyn Error>>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }
        let mut_tokens: &mut Tokens = &mut self.tokens;
        let (current_pre_ref_cell, prev) = unsafe { Self::take_out_pair(mut_tokens, self.index) };
        let current = RefCell::new(current_pre_ref_cell);
        let borrowed_current = current.borrow();
        let is_valid = borrowed_current.is_valid();
        if !is_valid {
            Some(Some(Err(Box::new(InvalidToken(
                borrowed_current.inner().token.to_string(),
            )))))
        } else {
            if borrowed_current.is_start(prev.inner()) {
                let end = mut_tokens.forward(self.index, &prev);
                if mut_tokens.is_end(end) {
                    drop(borrowed_current);
                    let tag = current.into_inner().take_tag();
                    let entity = Entity {
                        sent_id: mut_tokens.sent_id,
                        start: self.index,
                        end,
                        tag,
                    };
                    self.index = end;
                    return Some(Some(Ok(entity)));
                } else {
                    self.index += 1;
                    return Some(None);
                }
            } else {
                self.index += 1;
                Some(None)
            }
        }
    }
}
impl<'a> EntitiesAdaptor<'a> {
    /// Takes out the current and previous tokens (in that order) when given an index. The index
    /// must be >= 1 and < tokens.len() or this function will result in UB. Calling this function
    /// with an already use index will result in default tokens.
    ///
    /// SAFETY: The index must be >= 1 and < tokens.len(), or this function will result in UB.
    ///
    /// * `tokens`: RefCell wrapping the tokens. The current and previous tokens are extracted from
    /// its extended_tokens field.
    /// * `index`: Index specifying the current token. `index-1` is used to take the previous
    /// token.
    unsafe fn take_out_pair(tokens: &mut Tokens<'a>, index: usize) -> (Token<'a>, Token<'a>) {
        let current_token = take(tokens.extended_tokens.get_unchecked_mut(index));
        let previous_token = take(tokens.extended_tokens.get_unchecked_mut(index - 1));
        (current_token, previous_token)
    }
    fn new(tokens: Tokens<'a>) -> Self {
        let len = tokens.extended_tokens.len();
        Self {
            index: 1,
            tokens,
            len,
        }
    }
}

// struct Entities
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_entity_adaptor_iterator() {
        let tokens = build_tokens();
        println!("{:?}", tokens);
        let mut iter = EntitiesAdaptor::new(tokens.clone());
        let first_entity = iter.next().unwrap();
        println!("{:?}", first_entity);
        assert!(first_entity.is_none());
        let second_entity = iter.next().unwrap().unwrap(); /* .unwrap();  */
        println!("{:?}", second_entity);
        // assert_eq!(
        //     second_entity,
        //     Entity {
        //         sent_id: None,
        //         start: 1,
        //         end: 2,
        //         tag: Cow::Borrowed("PER")
        //     }
        // )

        // let expected_first_entity = Entity {
        //     sent_id: None,
        //     start: 0,
        //     end: 2,
        //     tag: Cow::Borrowed("PER"),
        // };
        // assert_eq!(first_entity, expected_first_entity);
    }
    #[test]
    fn test_check_pattern() {
        todo!();
    }

    #[test]
    fn test_token_is_start() {
        let tokens = build_tokens();
        println!("{:?}", tokens);
        println!("{:?}", tokens.extended_tokens());
        let prev = tokens.extended_tokens().get(0).unwrap();
        let is_start = tokens
            .extended_tokens()
            .get(1)
            .unwrap()
            .is_start(&prev.inner());
        assert!(is_start)
    }
    #[test]
    fn test_forward_method() {
        let tokens = build_tokens();
        println!("{:?}", &tokens);
        let end = tokens.forward(1, tokens.extended_tokens.get(0).unwrap());
        let expected_end = 1;
        assert_eq!(end, expected_end)
    }
    #[test]
    fn test_new_tokens() {
        let tokens = build_tokens();
        println!("{:?}", tokens);
        assert_eq!(tokens.extended_tokens.len(), 5);
    }
    #[test]
    fn test_innertoken_new() {
        let token = Cow::from("B-PER");
        let suffix = false;
        let delimiter = '-';
        let inner_token = InnerToken::new(token, suffix, delimiter).unwrap();
        let expected_inner_token = InnerToken {
            token: Cow::Borrowed("B-PER"),
            prefix: Prefix::B,
            tag: Cow::Owned(String::from("PER")),
        };
        assert_eq!(inner_token, expected_inner_token)
    }
    fn build_tokens() -> Tokens<'static> {
        let tokens = build_tokens_vec();
        let scheme = SchemeType::IOB2;
        let delimiter = '-';
        let suffix = false;
        Tokens::new(tokens, scheme, suffix, delimiter, None).unwrap()
    }
    fn build_tokens_vec() -> Vec<Cow<'static, str>> {
        vec![
            Cow::from("B-PER"),
            Cow::from("I-PER"),
            Cow::from("O"),
            Cow::from("B-LOC"),
        ]
    }
}
