//! Lexical tokens for the expression language.
//!
//! Port of `filters/private/expr/Token.hpp`.

/// The kind of a lexical token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType {
    /// End of input.
    Eof,
    /// A malformed token.
    Error,
    /// `=`
    Assign,
    /// `+`
    Plus,
    /// `-`
    Dash,
    /// `/`
    Slash,
    /// `*`
    Asterisk,
    /// `(`
    Lparen,
    /// `)`
    Rparen,
    /// `!`
    Not,
    /// `||`
    Or,
    /// `&&`
    And,
    /// `>`
    Greater,
    /// `<`
    Less,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<=`
    LessEqual,
    /// `>=`
    GreaterEqual,
    /// A numeric literal.
    Number,
    /// An identifier (dimension or function name).
    Identifier,
}

/// A lexical token: its kind, source span `[start, end)`, and value.
#[derive(Clone, Debug)]
pub struct Token {
    ty: TokenType,
    start: usize,
    end: usize,
    sval: String,
    dval: f64,
}

impl Token {
    /// A fully-specified token.
    pub fn new(
        ty: TokenType,
        start: usize,
        end: usize,
        sval: impl Into<String>,
        dval: f64,
    ) -> Self {
        Token {
            ty,
            start,
            end,
            sval: sval.into(),
            dval,
        }
    }

    /// A token with only a kind (zero span, empty value).
    pub fn of(ty: TokenType) -> Self {
        Token {
            ty,
            start: 0,
            end: 0,
            sval: String::new(),
            dval: 0.0,
        }
    }

    /// The token kind.
    pub fn ty(&self) -> TokenType {
        self.ty
    }

    /// Start offset of the token in the source string.
    pub fn start(&self) -> usize {
        self.start
    }

    /// End offset (one past the last character) of the token.
    pub fn end(&self) -> usize {
        self.end
    }

    /// The token's string value (the literal text, for identifiers/operators).
    pub fn sval(&self) -> &str {
        &self.sval
    }

    /// The token's numeric value (meaningful for [`TokenType::Number`]).
    pub fn dval(&self) -> f64 {
        self.dval
    }

    /// Whether the token is well-formed (not an error).
    pub fn valid(&self) -> bool {
        self.ty != TokenType::Error
    }

    /// Whether the token carries content -- valid and not end-of-input.
    /// The Rust analog of PDAL's `Token::operator bool`.
    pub fn is_content(&self) -> bool {
        self.valid() && self.ty != TokenType::Eof
    }

    /// Token equality as PDAL defines it: same kind, and -- for non-empty
    /// identifiers -- a case-insensitive value match.
    pub fn matches(&self, other: &Token) -> bool {
        if self.ty != other.ty {
            return false;
        }
        if self.ty == TokenType::Identifier && !self.sval.is_empty() {
            return self.sval.eq_ignore_ascii_case(&other.sval);
        }
        true
    }
}

impl Default for Token {
    /// PDAL's default-constructed `Token` is an error token.
    fn default() -> Self {
        Token::of(TokenType::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_error_token() {
        let t = Token::default();
        assert_eq!(t.ty(), TokenType::Error);
        assert!(!t.valid());
        assert!(!t.is_content());
    }

    #[test]
    fn valid_and_is_content_distinguish_error_and_eof() {
        let err = Token::of(TokenType::Error);
        assert!(!err.valid());
        assert!(!err.is_content());

        let eof = Token::of(TokenType::Eof);
        assert!(eof.valid());
        assert!(!eof.is_content());

        let plus = Token::new(TokenType::Plus, 0, 1, "+", 0.0);
        assert!(plus.valid());
        assert!(plus.is_content());
    }

    #[test]
    fn matches_handles_identifier_case_and_other_types() {
        let a = Token::new(TokenType::Identifier, 0, 5, "Class", 0.0);
        let b = Token::new(TokenType::Identifier, 5, 10, "CLASS", 0.0);
        let c = Token::new(TokenType::Identifier, 10, 15, "Other", 0.0);
        assert!(a.matches(&b));
        assert!(!a.matches(&c));

        let plus_a = Token::new(TokenType::Plus, 0, 1, "+", 0.0);
        let plus_b = Token::new(TokenType::Plus, 5, 6, "+", 0.0);
        assert!(plus_a.matches(&plus_b));

        let dash = Token::new(TokenType::Dash, 0, 1, "-", 0.0);
        assert!(!plus_a.matches(&dash));
    }

    #[test]
    fn accessor_helpers_round_trip() {
        let t = Token::new(TokenType::Number, 1, 4, "42", 42.0);
        assert_eq!(t.start(), 1);
        assert_eq!(t.end(), 4);
        assert_eq!(t.sval(), "42");
        assert_eq!(t.dval(), 42.0);
    }
}
