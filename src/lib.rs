//! This library is  a re-implementation of the SeqEval library. SeqEval is built with python and
//! is too slow when handling a large amount of strings. This library hopes to fulfill the same
//! niche, but hopefully in a much more performant way.

use std::borrow::Cow;
use std::error::Error;
use std::fmt::Display;
use std::marker::PhantomData;
use std::mem::replace;
use std::mem::swap;
use std::mem::take;
use std::rc::Rc;
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
            UnicodeIndex::Start(_) => &token[char_index..],
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
    fn inner_mut(&'a mut self) -> &'a mut InnerToken {
        match self {
            Self::IOE1 { token } => token,
            Self::IOE2 { token } => token,
            Self::IOB1 { token } => token,
            Self::IOB2 { token } => token,
            Self::BILOU { token } => token,
            Self::IOBES { token } => token,
        }
    }
    // fn switch_tag<'b: 'a>(&'a mut self, other: Cow<'b, str>) -> Cow<'b, str> {
    //     replace(&mut self.inner().tag, other)
    // }

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
}

#[derive(Debug, Clone, Copy)]
struct NotInit();
#[derive(Debug, Clone, Copy)]
struct Init();

#[derive(Debug, Clone, PartialEq)]
struct Tokens<'a, Init> {
    extended_tokens: Vec<Token<'a>>,
    sent_id: Option<usize>,
    init: PhantomData<Init>,
}
impl<'a> Tokens<'a, NotInit> {
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
        let mut tokens: Vec<Token> = tokens
            .into_iter()
            .map(|cow_str| {
                let inner = InnerToken::new(cow_str, suffix, delimiter)?;
                Ok::<Token, ParsingPrefixError<&'a str>>(Token::new(scheme, inner))
            })
            .collect::<Result<Vec<Token>, ParsingPrefixError<&'a str>>>()?;
        tokens.push(outside_token); // Tokens are now extended_tokens
        Ok(Self {
            extended_tokens: tokens,
            sent_id,
            init: PhantomData,
        })
    }

    /// Extract the entities from the Tokens.
    pub(crate) fn entities(self) -> Result<Vec<Entity<'a>>, Box<dyn Error>> {
        let mut i = 0;
        let mut prev: &Token = self.extended_tokens.get(0).ok_or_else(|| {
            Box::new(Err(
                "Trying to convert an empty list of tokens into a list of entities",
            ))
        })?;
        // let mut entities: Vec<Entity> = vec![];
        // let rc_extended_tokens = Rc::new(self.extended_tokens.unwrap()); // Can unwrap because we are in the
        //                                                                  // NotInit impl block
        // let num_extended_tokens = rc_extended_tokens.len();
        // let mut prev = rc_extended_tokens.last().ok_or_else(|| {
        //     format!(
        //         "This Tokens struct does not contains any token. self.extended_tokens: {:?}",
        //         rc_extended_tokens
        //     )
        // })?;
        // while i < num_extended_tokens {
        //     let token = unsafe { rc_extended_tokens.get_unchecked(i) }; // Safe due to us always making sure i<num_extended_tokens
        //     if !token.is_valid() {
        //         return Err(Box::new(InvalidToken(token.inner().token.to_string())));
        //     }
        //     if token.is_start(prev.inner()) {
        //         let end = Self::unassociated_forward(
        //             i + 1,
        //             &token,
        //             rc_extended_tokens.as_ref(),
        //             num_extended_tokens,
        //         );
        //         if Self::unassociated_is_end(end, &rc_extended_tokens) {
        //             let entity = Entity {
        //                 sent_id: match &self.sent_id {
        //                     Some(i) => Some(*i),
        //                     None => None,
        //                 },
        //                 start: i,
        //                 end,
        //                 tag: match &token.inner().tag {
        //                     Cow::Owned(owned_string) => Cow::Owned(owned_string.clone()),
        //                     Cow::Borrowed(ref_string) => Cow::Borrowed(*ref_string),
        //                 },
        //             };
        //             entities.push(entity)
        //         }
        //         i = end;
        //     } else {
        //         i += 1;
        //     }
        //     prev = &rc_extended_tokens[i - 1];
        // }
        // Ok(Tokens {
        //     entities: Some(entities),
        //     extended_tokens: None,
        //     sent_id: None,
        //     init: PhantomData,
        // })
    }

    /// Returns the index of the next token not inside, starting from the `start` index.
    ///
    /// * `start`: Indexing at which we are starting to look for a token not inside.
    /// * `prev`: Previous token. This token is necessary to know if the token at index `start` is
    /// inside or not.
    fn forward(&self, start: usize, prev: &Token<'a>) -> usize {
        let slice_of_interest = &self.extended_tokens()[start..];
        let len_of_slice_of_interest = slice_of_interest.len();
        let mut counter = start; // copies the start index
        let mut swap_token = prev;
        loop {
            let current_token = &slice_of_interest[counter];
            if current_token.is_inside(swap_token.inner()) {
                swap_token = &current_token;
            } else {
                return counter;
            }
            counter += 1;
            if counter >= len_of_slice_of_interest {
                break &self.extended_tokens().len() - 1;
            }
        }
    }

    fn unassociated_forward(
        start: usize,
        prev: &Token<'a>,
        slice: &[Token],
        slice_len: usize,
    ) -> usize {
        let slice_of_interest = &slice[start..];
        let len_of_slice_of_interest = slice_of_interest.len();
        let mut counter = start; // copies the start index
        let mut swap_token = prev;
        loop {
            let current_token = &slice_of_interest[counter];
            if current_token.is_inside(swap_token.inner()) {
                swap_token = &current_token;
            } else {
                return counter;
            }
            counter += 1;
            if counter >= len_of_slice_of_interest {
                break slice_len - 1;
            }
        }
    }

    fn is_end(&self, i: usize) -> bool {
        let token = &self.extended_tokens()[i];
        let prev = &self.extended_tokens()[i - 1];
        token.is_end(prev.inner())
    }
    fn unassociated_is_end<'b>(i: usize, tokens: &'b Vec<Token<'b>>) -> bool {
        let token = &tokens[i];
        let prev = &tokens[i - 1];
        token.is_end(prev.inner())
    }

    fn extended_tokens(&'a self) -> &'a Vec<Token<'a>> {
        self.extended_tokens.as_ref().unwrap()
    }
}

/// Iterator for iterating over the entities of a Tokens struct
///
/// * `index`: Index of the current iteration
/// * `current`: Current token
/// * `prev`:  Previous token
/// * `prev_prev`: Previous token of the previous token
struct Entities<'a> {
    index: usize,
    tokens: Tokens<'a, NotInit>,
    current: &'a mut Token<'a>,
    prev: &'a Token<'a>,
    prev_prev: &'a Token<'a>,
}
impl<'a> Iterator for Entities<'a> {
    type Item = Result<Entity<'a>, Box<dyn Error>>;
    fn next(&mut self) -> Option<Self::Item> {
        let res: Option<Result<Entity<'a>, Box<dyn Error>>>;
        self.current.is_valid().then_some({
            if self.current.is_start(self.prev.inner()) {
                let end = self.tokens.forward(self.index + 1, self.prev);
                if self.tokens.is_end(end) {
                    let entity = Entity {
                        sent_id: self.tokens.sent_id,
                        start: self.index,
                        end,
                        tag: replace(&mut self.current.inner_mut().tag, Cow::Borrowed("")),
                    };
                }
            }
        });
        res = Some(Err(Box::new(InvalidToken(
            self.current.inner().token.to_string(),
        ))));
        self.index += 1;
        //TODO: Self.current becomes previous, current becomes tokens[index]
        res
    }
}
