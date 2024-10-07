//! One of the goal of this crate is to provide facilities to convert scheme such as the following:
//!
//! class IOBES(Token):
//!     allowed_prefix = Prefix.I | Prefix.O | Prefix.B | Prefix.E | Prefix.S
//!     start_patterns = {
//!         (Prefix.ANY, Prefix.B, Tag.ANY),
//!         (Prefix.ANY, Prefix.S, Tag.ANY)
//!     }
//!     inside_patterns = {
//!         (Prefix.B, Prefix.I, Tag.SAME),
//!         (Prefix.B, Prefix.E, Tag.SAME),
//!         (Prefix.I, Prefix.I, Tag.SAME),
//!         (Prefix.I, Prefix.E, Tag.SAME)
//!     }
//!     end_patterns = {
//!         (Prefix.S, Prefix.ANY, Tag.ANY),
//!         (Prefix.E, Prefix.ANY, Tag.ANY)
//!     }
//!
//! into this:
//!
//! const IOBES_ALLOWED_PREFIXES: [Prefix; 5] =
//!     [Prefix::I, Prefix::O, Prefix::E, Prefix::B, Prefix::S];
//! const IOBES_START_PATTERNS: [(Prefix, Prefix, Tag); 4] = [
//!     (Prefix::B, Prefix::I, Tag::Same),
//!     (Prefix::B, Prefix::E, Tag::Same),
//!     (Prefix::I, Prefix::I, Tag::Same),
//!     (Prefix::I, Prefix::E, Tag::Same),
//! ];
//! const IOBES_INSIDE_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
//!     (Prefix::S, Prefix::PrefixAny, Tag::TagAny),
//!     (Prefix::E, Prefix::PrefixAny, Tag::TagAny),
//! ];
//! const IOBES_END_PATTERNS: [(Prefix, Prefix, Tag); 2] = [
//!     (Prefix::S, Prefix::PrefixAny, Tag::TagAny),
//!     (Prefix::E, Prefix::PrefixAny, Tag::TagAny),
//! ];
