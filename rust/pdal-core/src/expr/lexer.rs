//! Lexer for the expression language.
//!
//! Port of `filters/private/expr/Lexer.{hpp,cpp}`. The expression language is
//! ASCII, so the lexer works over bytes with byte offsets, matching PDAL's
//! byte-indexed `std::string`.

use super::token::{Token, TokenType};

/// Tokenizer over an expression string.
pub struct Lexer {
    buf: Vec<u8>,
    pos: usize,
    tok_pos: usize,
}

impl Lexer {
    /// A lexer over `s`.
    pub fn new(s: &str) -> Self {
        Lexer {
            buf: s.as_bytes().to_vec(),
            pos: 0,
            tok_pos: 0,
        }
    }

    /// Re-point the lexer at a new string.
    pub fn reset(&mut self, s: &str) {
        self.buf = s.as_bytes().to_vec();
        self.pos = 0;
        self.tok_pos = 0;
    }

    /// Current scan position.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Rewind so the next `get` re-reads `t` (PDAL's `Lexer::put`).
    pub fn put(&mut self, t: &Token) {
        self.pos = t.start();
    }

    /// Position the lexer just past `t` (PDAL's `Lexer::putEnd`).
    pub fn put_end(&mut self, t: &Token) {
        self.pos = t.end();
    }

    /// Read one byte and advance; yields `0` once past the end.
    fn get_char(&mut self) -> u8 {
        let c = if self.pos < self.buf.len() {
            self.buf[self.pos]
        } else {
            0
        };
        self.pos += 1;
        c
    }

    /// Step back one byte.
    fn put_char(&mut self) {
        debug_assert!(self.pos != 0);
        self.pos -= 1;
    }

    /// Produce the next token.
    pub fn get(&mut self) -> Token {
        if self.pos >= self.buf.len() {
            return Token::new(TokenType::Eof, self.buf.len(), self.buf.len(), "", 0.0);
        }
        loop {
            self.tok_pos = self.pos;
            let c = self.get_char();
            if c.is_ascii_whitespace() {
                continue;
            }
            return self.top(c);
        }
    }

    /// Dispatch on the first character of a token.
    fn top(&mut self, c: u8) -> Token {
        match c {
            b'&' => self.ampersand(),
            b'|' => self.bar(),
            b'!' => self.exclamation(),
            b'-' => self.dash(),
            b'<' => self.less(),
            b'>' => self.greater(),
            b'=' => self.equal(),
            b'+' => Token::new(TokenType::Plus, self.tok_pos, self.pos, "+", 0.0),
            b'*' => Token::new(TokenType::Asterisk, self.tok_pos, self.pos, "*", 0.0),
            b'/' => Token::new(TokenType::Slash, self.tok_pos, self.pos, "/", 0.0),
            b'(' => Token::new(TokenType::Lparen, self.tok_pos, self.pos, "(", 0.0),
            b')' => Token::new(TokenType::Rparen, self.tok_pos, self.pos, ")", 0.0),
            b'0'..=b'9' => self.number(),
            b'a'..=b'z' | b'A'..=b'Z' => self.letter(),
            _ => Token::new(
                TokenType::Error,
                self.tok_pos,
                self.pos,
                (c as char).to_string(),
                0.0,
            ),
        }
    }

    fn ampersand(&mut self) -> Token {
        if self.get_char() == b'&' {
            return Token::new(TokenType::And, self.tok_pos, self.pos, "&&", 0.0);
        }
        self.put_char();
        Token::new(
            TokenType::Error,
            self.tok_pos,
            self.pos,
            "'&' invalid in this context.",
            0.0,
        )
    }

    fn bar(&mut self) -> Token {
        if self.get_char() == b'|' {
            return Token::new(TokenType::Or, self.tok_pos, self.pos, "||", 0.0);
        }
        self.put_char();
        // NOTE: PDAL's message text says '!' here; kept for a faithful port.
        Token::new(
            TokenType::Error,
            self.tok_pos,
            self.pos,
            "'!' invalid in this context.",
            0.0,
        )
    }

    fn exclamation(&mut self) -> Token {
        if self.get_char() == b'=' {
            return Token::new(TokenType::NotEqual, self.tok_pos, self.pos, "!=", 0.0);
        }
        self.put_char();
        Token::new(TokenType::Not, self.tok_pos, self.pos, "!", 0.0)
    }

    fn dash(&mut self) -> Token {
        let c = self.get_char();
        self.put_char();
        if c != b'-' {
            return Token::new(TokenType::Dash, self.tok_pos, self.pos, "-", 0.0);
        }
        Token::new(
            TokenType::Error,
            self.tok_pos,
            self.pos,
            "Found disallowed consecutive dashes: '--'",
            0.0,
        )
    }

    fn equal(&mut self) -> Token {
        if self.get_char() == b'=' {
            return Token::new(TokenType::Equal, self.tok_pos, self.pos, "==", 0.0);
        }
        self.put_char();
        Token::new(TokenType::Assign, self.tok_pos, self.pos, "=", 0.0)
    }

    fn less(&mut self) -> Token {
        if self.get_char() == b'=' {
            return Token::new(TokenType::LessEqual, self.tok_pos, self.pos, "<=", 0.0);
        }
        self.put_char();
        Token::new(TokenType::Less, self.tok_pos, self.pos, "<", 0.0)
    }

    fn greater(&mut self) -> Token {
        if self.get_char() == b'=' {
            return Token::new(TokenType::GreaterEqual, self.tok_pos, self.pos, ">=", 0.0);
        }
        self.put_char();
        Token::new(TokenType::Greater, self.tok_pos, self.pos, ">", 0.0)
    }

    /// Lex a numeric literal: the longest `f64`-parseable prefix at `tok_pos`.
    /// PDAL extracts the number with a stream; this takes the longest prefix
    /// the standard float parser accepts, which agrees on well-formed input.
    fn number(&mut self) -> Token {
        let rest = &self.buf[self.tok_pos..];
        let mut run = 0;
        while run < rest.len() {
            match rest[run] {
                b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-' => run += 1,
                _ => break,
            }
        }
        // The triggering digit always parses, so length is at least 1.
        let mut len = 1;
        let mut value = f64::from(rest[0] - b'0');
        let mut n = run;
        while n >= 1 {
            if let Ok(text) = std::str::from_utf8(&rest[..n]) {
                if let Ok(v) = text.parse::<f64>() {
                    len = n;
                    value = v;
                    break;
                }
            }
            n -= 1;
        }
        self.pos = self.tok_pos + len;
        let sval = String::from_utf8_lossy(&rest[..len]).into_owned();
        Token::new(TokenType::Number, self.tok_pos, self.pos, sval, value)
    }

    /// Lex an identifier: a run of alphanumerics and underscores.
    fn letter(&mut self) -> Token {
        loop {
            let c = self.get_char();
            if !(c.is_ascii_alphanumeric() || c == b'_') {
                self.put_char();
                let sval = String::from_utf8_lossy(&self.buf[self.tok_pos..self.pos]).into_owned();
                return Token::new(TokenType::Identifier, self.tok_pos, self.pos, sval, 0.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collect token kinds until (and including) the terminating Eof/Error.
    fn kinds(src: &str) -> Vec<TokenType> {
        let mut lex = Lexer::new(src);
        let mut out = Vec::new();
        loop {
            let t = lex.get();
            out.push(t.ty());
            if matches!(t.ty(), TokenType::Eof | TokenType::Error) {
                break;
            }
        }
        out
    }

    #[test]
    fn comparison_expression() {
        use TokenType::*;
        assert_eq!(
            kinds("Classification == 2"),
            vec![Identifier, Equal, Number, Eof]
        );
    }

    #[test]
    fn logical_and_relational_operators() {
        use TokenType::*;
        assert_eq!(
            kinds("X >= 0 || Y <= 10 && !(Z != 1)"),
            vec![
                Identifier,
                GreaterEqual,
                Number,
                Or,
                Identifier,
                LessEqual,
                Number,
                And,
                Not,
                Lparen,
                Identifier,
                NotEqual,
                Number,
                Rparen,
                Eof,
            ]
        );
    }

    #[test]
    fn arithmetic_operators() {
        use TokenType::*;
        assert_eq!(
            kinds("Intensity * 2 + 1 / 3 - 4"),
            vec![Identifier, Asterisk, Number, Plus, Number, Slash, Number, Dash, Number, Eof,]
        );
    }

    #[test]
    fn number_values() {
        let mut lex = Lexer::new("50.5 100 2e3");
        assert_eq!(lex.get().dval(), 50.5);
        assert_eq!(lex.get().dval(), 100.0);
        assert_eq!(lex.get().dval(), 2000.0);
        assert_eq!(lex.get().ty(), TokenType::Eof);
    }

    #[test]
    fn identifier_value_and_span() {
        let mut lex = Lexer::new("Red_Green1");
        let t = lex.get();
        assert_eq!(t.ty(), TokenType::Identifier);
        assert_eq!(t.sval(), "Red_Green1");
        assert_eq!((t.start(), t.end()), (0, 10));
    }

    #[test]
    fn stray_character_is_an_error() {
        assert_eq!(
            kinds("X @ Y"),
            vec![TokenType::Identifier, TokenType::Error]
        );
    }

    #[test]
    fn double_dash_is_rejected() {
        // A single dash lexes; consecutive dashes are disallowed by PDAL.
        assert_eq!(
            kinds("1 - 2"),
            vec![
                TokenType::Number,
                TokenType::Dash,
                TokenType::Number,
                TokenType::Eof
            ]
        );
        assert_eq!(kinds("1 -- 2"), vec![TokenType::Number, TokenType::Error]);
    }

    #[test]
    fn put_rewinds_to_reread_a_token() {
        let mut lex = Lexer::new("A B");
        let first = lex.get();
        assert_eq!(first.sval(), "A");
        lex.put(&first);
        assert_eq!(lex.get().sval(), "A"); // re-read
        assert_eq!(lex.get().sval(), "B");
    }

    #[test]
    fn reset_repoints_lexer() {
        let mut lex = Lexer::new("123");
        let t = lex.get();
        assert_eq!(t.dval(), 123.0);
        lex.reset("4 + 5");
        let t = lex.get();
        assert_eq!(t.dval(), 4.0);
    }

    #[test]
    fn pos_and_put_end_advance_helpers() {
        let mut lex = Lexer::new("12 + 34");
        let t = lex.get();
        assert_eq!(t.dval(), 12.0);
        let p_before = lex.pos();
        lex.put_end(&t);
        assert_eq!(lex.pos(), t.end());
        assert_eq!(p_before, t.end());
        lex.put(&t);
        assert_eq!(lex.pos(), t.start());
    }

    #[test]
    fn single_ampersand_is_error() {
        let mut lex = Lexer::new("&");
        let t = lex.get();
        assert_eq!(t.ty(), TokenType::Error);
    }

    #[test]
    fn single_bar_is_error() {
        let mut lex = Lexer::new("|");
        let t = lex.get();
        assert_eq!(t.ty(), TokenType::Error);
    }

    #[test]
    fn exclamation_alone_is_not() {
        let mut lex = Lexer::new("!a");
        let t = lex.get();
        assert_eq!(t.ty(), TokenType::Not);
    }

    #[test]
    fn double_ampersand_is_and() {
        let mut lex = Lexer::new("&&");
        let t = lex.get();
        assert_eq!(t.ty(), TokenType::And);
    }

    #[test]
    fn double_bar_is_or() {
        let mut lex = Lexer::new("||");
        let t = lex.get();
        assert_eq!(t.ty(), TokenType::Or);
    }

    #[test]
    fn unrecognized_character_is_error_token() {
        let mut lex = Lexer::new("@");
        let t = lex.get();
        assert_eq!(t.ty(), TokenType::Error);
    }
}
