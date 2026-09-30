//! Lexical Tokenizer for IEC 61131-3 Structured Text (ST).

use crate::error::ControlError;

/// Token kinds produced by the ST lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    If,
    Then,
    Elsif,
    Else,
    EndIf,
    Case,
    Of,
    EndCase,
    For,
    To,
    By,
    Do,
    EndFor,
    While,
    EndWhile,
    Repeat,
    Until,
    EndRepeat,
    Return,
    Exit,
    Var,
    VarInput,
    VarOutput,
    VarInOut,
    VarGlobal,
    VarConstant,
    VarTemp,
    EndVar,
    Program,
    EndProgram,
    FunctionBlock,
    EndFunctionBlock,
    True,
    False,

    // Operators & Logical Keywords
    And,
    Or,
    Xor,
    Not,
    Mod,

    // Relational Operators
    Equal,        // =
    NotEqual,     // <>
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=

    // Arithmetic Operators
    Plus,  // +
    Minus, // -
    Star,  // *
    Slash, // /
    Power, // **

    // Punctuation & Delimiters
    Assign,    // :=
    Colon,     // :
    Semicolon, // ;
    Comma,     // ,
    Dot,       // .
    DotDot,    // ..
    Arrow,     // =>
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]

    // Literals & Identifiers
    Ident(String),
    Int(i64),
    Real(f64),
    Time(f64), // Seconds
    Str(String),
    Eof,
}

/// Tokenizer that scans Structured Text source code.
pub struct Lexer<'a> {
    input: &'a str,
    chars: Vec<(usize, char)>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let chars = input.char_indices().collect();
        Self {
            input,
            chars,
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).map(|&(_, c)| c)
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).map(|&(_, c)| c)
    }

    fn advance(&mut self) -> Option<char> {
        let res = self.peek();
        if res.is_some() {
            self.pos += 1;
        }
        res
    }

    /// Tokenizes the entire source string into a sequence of Tokens.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, ControlError> {
        let mut tokens = Vec::new();
        while self.pos < self.chars.len() {
            let c = self.peek().unwrap();

            // Skip whitespace
            if c.is_whitespace() {
                self.advance();
                continue;
            }

            // Single line comment: //
            if c == '/' && self.peek_next() == Some('/') {
                while let Some(ch) = self.advance() {
                    if ch == '\n' {
                        break;
                    }
                }
                continue;
            }

            // Block comment: (* ... *)
            if c == '(' && self.peek_next() == Some('*') {
                self.advance();
                self.advance();
                let mut closed = false;
                while let Some(ch) = self.advance() {
                    if ch == '*' && self.peek() == Some(')') {
                        self.advance();
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    return Err(ControlError::InvalidParameter(
                        "Unterminated block comment (* ... *)".to_string(),
                    ));
                }
                continue;
            }

            // String literal: '...'
            if c == '\'' {
                self.advance();
                let mut s = String::new();
                let mut closed = false;
                while let Some(ch) = self.advance() {
                    if ch == '\'' {
                        closed = true;
                        break;
                    }
                    s.push(ch);
                }
                if !closed {
                    return Err(ControlError::InvalidParameter(
                        "Unterminated string literal".to_string(),
                    ));
                }
                tokens.push(Token::Str(s));
                continue;
            }

            // Two-character punctuation
            if c == ':' && self.peek_next() == Some('=') {
                self.advance();
                self.advance();
                tokens.push(Token::Assign);
                continue;
            }
            if c == '<' && self.peek_next() == Some('>') {
                self.advance();
                self.advance();
                tokens.push(Token::NotEqual);
                continue;
            }
            if c == '<' && self.peek_next() == Some('=') {
                self.advance();
                self.advance();
                tokens.push(Token::LessEqual);
                continue;
            }
            if c == '>' && self.peek_next() == Some('=') {
                self.advance();
                self.advance();
                tokens.push(Token::GreaterEqual);
                continue;
            }
            if c == '.' && self.peek_next() == Some('.') {
                self.advance();
                self.advance();
                tokens.push(Token::DotDot);
                continue;
            }
            if c == '=' && self.peek_next() == Some('>') {
                self.advance();
                self.advance();
                tokens.push(Token::Arrow);
                continue;
            }
            if c == '*' && self.peek_next() == Some('*') {
                self.advance();
                self.advance();
                tokens.push(Token::Power);
                continue;
            }

            // Single character punctuation
            match c {
                '=' => {
                    self.advance();
                    tokens.push(Token::Equal);
                    continue;
                }
                '<' => {
                    self.advance();
                    tokens.push(Token::Less);
                    continue;
                }
                '>' => {
                    self.advance();
                    tokens.push(Token::Greater);
                    continue;
                }
                '+' => {
                    self.advance();
                    tokens.push(Token::Plus);
                    continue;
                }
                '-' => {
                    self.advance();
                    tokens.push(Token::Minus);
                    continue;
                }
                '*' => {
                    self.advance();
                    tokens.push(Token::Star);
                    continue;
                }
                '/' => {
                    self.advance();
                    tokens.push(Token::Slash);
                    continue;
                }
                ':' => {
                    self.advance();
                    tokens.push(Token::Colon);
                    continue;
                }
                ';' => {
                    self.advance();
                    tokens.push(Token::Semicolon);
                    continue;
                }
                ',' => {
                    self.advance();
                    tokens.push(Token::Comma);
                    continue;
                }
                '.' => {
                    self.advance();
                    tokens.push(Token::Dot);
                    continue;
                }
                '(' => {
                    self.advance();
                    tokens.push(Token::LParen);
                    continue;
                }
                ')' => {
                    self.advance();
                    tokens.push(Token::RParen);
                    continue;
                }
                '[' => {
                    self.advance();
                    tokens.push(Token::LBracket);
                    continue;
                }
                ']' => {
                    self.advance();
                    tokens.push(Token::RBracket);
                    continue;
                }
                _ => {}
            }

            // Time literal (T#... or TIME#...)
            if (c == 'T' || c == 't')
                && (self.peek_next() == Some('#')
                    || (self.pos + 4 < self.chars.len()
                        && &self.input[self.chars[self.pos].0..self.chars[self.pos + 5].0]
                            .to_ascii_uppercase()
                            == "TIME#"))
            {
                tokens.push(self.scan_time_literal()?);
                continue;
            }

            // Number literals (Integer or Real, Hex 16#..., Binary 2#...)
            if c.is_ascii_digit() {
                tokens.push(self.scan_number_literal()?);
                continue;
            }

            // Identifiers, Keywords, or I/O address tokens (e.g. `%IW100`, `P_101`)
            if c.is_alphabetic() || c == '_' || c == '%' {
                let id = self.scan_ident();
                let upper = id.to_ascii_uppercase();
                let tok = match upper.as_str() {
                    "IF" => Token::If,
                    "THEN" => Token::Then,
                    "ELSIF" => Token::Elsif,
                    "ELSE" => Token::Else,
                    "END_IF" => Token::EndIf,
                    "CASE" => Token::Case,
                    "OF" => Token::Of,
                    "END_CASE" => Token::EndCase,
                    "FOR" => Token::For,
                    "TO" => Token::To,
                    "BY" => Token::By,
                    "DO" => Token::Do,
                    "END_FOR" => Token::EndFor,
                    "WHILE" => Token::While,
                    "END_WHILE" => Token::EndWhile,
                    "REPEAT" => Token::Repeat,
                    "UNTIL" => Token::Until,
                    "END_REPEAT" => Token::EndRepeat,
                    "RETURN" => Token::Return,
                    "EXIT" => Token::Exit,
                    "VAR" => Token::Var,
                    "VAR_INPUT" => Token::VarInput,
                    "VAR_OUTPUT" => Token::VarOutput,
                    "VAR_IN_OUT" => Token::VarInOut,
                    "VAR_GLOBAL" => Token::VarGlobal,
                    "VAR_CONSTANT" => Token::VarConstant,
                    "VAR_TEMP" => Token::VarTemp,
                    "END_VAR" => Token::EndVar,
                    "PROGRAM" => Token::Program,
                    "END_PROGRAM" => Token::EndProgram,
                    "FUNCTION_BLOCK" => Token::FunctionBlock,
                    "END_FUNCTION_BLOCK" => Token::EndFunctionBlock,
                    "TRUE" => Token::True,
                    "FALSE" => Token::False,
                    "AND" => Token::And,
                    "OR" => Token::Or,
                    "XOR" => Token::Xor,
                    "NOT" => Token::Not,
                    "MOD" => Token::Mod,
                    _ => Token::Ident(id),
                };
                tokens.push(tok);
                continue;
            }

            return Err(ControlError::InvalidParameter(format!(
                "Unexpected character '{c}' at position {}",
                self.pos
            )));
        }

        tokens.push(Token::Eof);
        Ok(tokens)
    }

    fn scan_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '%' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    fn scan_number_literal(&mut self) -> Result<Token, ControlError> {
        let mut s = String::new();
        let mut is_real = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '#' || c.is_ascii_alphabetic() {
                s.push(c);
                self.advance();
            } else if c == '.' && self.peek_next() != Some('.') {
                is_real = true;
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Check for Hex 16#FF or Binary 2#1010
        if let Some(idx) = s.find('#') {
            let radix_str = &s[..idx];
            let num_str = &s[idx + 1..];
            let radix: u32 = radix_str.parse().map_err(|_| {
                ControlError::InvalidParameter(format!("Invalid radix in number literal '{s}'"))
            })?;
            let val = i64::from_str_radix(num_str, radix).map_err(|_| {
                ControlError::InvalidParameter(format!(
                    "Invalid integer '{num_str}' for base {radix}"
                ))
            })?;
            return Ok(Token::Int(val));
        }

        if is_real {
            let v: f64 = s.parse().map_err(|_| {
                ControlError::InvalidParameter(format!("Invalid real literal '{s}'"))
            })?;
            Ok(Token::Real(v))
        } else {
            let v: i64 = s.parse().map_err(|_| {
                ControlError::InvalidParameter(format!("Invalid integer literal '{s}'"))
            })?;
            Ok(Token::Int(v))
        }
    }

    fn scan_time_literal(&mut self) -> Result<Token, ControlError> {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '#' || c == '.' || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Strip T# or TIME#
        let body = if let Some(idx) = s.find('#') {
            &s[idx + 1..]
        } else {
            &s
        };

        // Parse e.g. "500ms", "2s", "1m30s", "1h"
        let mut total_sec = 0.0;
        let mut num_buf = String::new();
        let mut idx = 0;
        let chars: Vec<char> = body.chars().collect();

        while idx < chars.len() {
            let c = chars[idx];
            if c.is_ascii_digit() || c == '.' {
                num_buf.push(c);
                idx += 1;
            } else if c.is_alphabetic() {
                let mut unit = String::new();
                while idx < chars.len() && chars[idx].is_alphabetic() {
                    unit.push(chars[idx]);
                    idx += 1;
                }
                let val: f64 = num_buf.parse().unwrap_or(0.0);
                num_buf.clear();
                match unit.to_ascii_lowercase().as_str() {
                    "d" => total_sec += val * 86400.0,
                    "h" => total_sec += val * 3600.0,
                    "m" => total_sec += val * 60.0,
                    "s" => total_sec += val,
                    "ms" => total_sec += val / 1000.0,
                    "us" => total_sec += val / 1_000_000.0,
                    _ => {
                        return Err(ControlError::InvalidParameter(format!(
                            "Unknown time unit '{unit}' in time literal '{s}'"
                        )))
                    }
                }
            } else {
                idx += 1;
            }
        }

        Ok(Token::Time(total_sec))
    }
}
