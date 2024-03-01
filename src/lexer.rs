#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum FuncType {
    Sqrt,
    Log,
    Sin,
    Cos,
    Pow,
}

impl FuncType {
    pub fn arity(&self) -> usize {
        match self {
            Self::Pow => 2,
            Self::Sqrt | Self::Log | Self::Cos | Self::Sin => 1,
        }
    }
}

impl From<FuncType> for String {
    fn from(value: FuncType) -> Self {
        match value {
            FuncType::Log => "Log".into(),
            FuncType::Sin => "sin".into(),
            FuncType::Cos => "cos".into(),
            FuncType::Sqrt => "sqrt".into(),
            FuncType::Pow => "pow".into(),
        }
    }
}

impl From<FuncType> for u8 {
    fn from(value: FuncType) -> Self {
        value as u8
    }
}

#[derive(Debug)]
pub struct InvalidFuncCode(u8);

impl TryFrom<u8> for FuncType {
    type Error = InvalidFuncCode;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if FuncType::Sqrt as u8 == x => Ok(FuncType::Sqrt),
            x if FuncType::Log as u8 == x => Ok(FuncType::Log),
            x if FuncType::Sin as u8 == x => Ok(FuncType::Sin),
            x if FuncType::Cos as u8 == x => Ok(FuncType::Cos),
            x if FuncType::Pow as u8 == x => Ok(FuncType::Pow),
            x => Err(InvalidFuncCode(x)),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Token<'a> {
    Number(&'a [u8]),
    LeftParen,
    RightParen,
    Plus,
    Minus,
    Mult,
    Div,
    Func(FuncType),
    Comma,
    Ans,
}

impl<'a> From<Token<'a>> for String {
    fn from(value: Token) -> Self {
        match value {
            Token::Div => "/".to_string(),
            Token::Mult => "*".to_string(),
            Token::Number(digits) => String::from_utf8_lossy(digits).into_owned(),
            Token::Plus => "+".to_string(),
            Token::Minus => "-".to_string(),
            Token::LeftParen => "(".to_string(),
            Token::RightParen => ")".to_string(),
            Token::Func(f) => f.into(),
            Token::Comma => ",".to_string(),
            Token::Ans => "ans".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Null,
    Comma,
    Number,
    Term,
    Factor,
    Unary,
    Group,
}

impl Priority {
    pub fn next(&self) -> Self {
        match self {
            Self::Null => Self::Comma,
            Self::Comma => Self::Number,
            Self::Number => Self::Term,
            Self::Term => Self::Factor,
            Self::Factor => Self::Unary,
            Self::Unary => Self::Group,
            Self::Group => Self::Group,
        }
    }
}

impl<'a> Token<'a> {
    pub fn priority(&self) -> Priority {
        match self {
            Token::Ans => Priority::Number,
            Token::Number(_) => Priority::Number,
            Token::Func(_) => Priority::Factor,
            Token::LeftParen => Priority::Group,
            Token::RightParen => Priority::Null,
            Token::Plus => Priority::Term,
            Token::Minus => Priority::Term,
            Token::Mult => Priority::Factor,
            Token::Div => Priority::Factor,
            Token::Comma => Priority::Comma,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Error {
    Eof,
    InvalidChar(char),
    InvalidNumberFormat(char),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

pub trait Scan<'a> {
    fn scan(&mut self) -> Result<Token<'a>, Error>;
}

pub struct Lexer<'a> {
    src: &'a [u8],
    src_index: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a [u8]) -> Self {
        Lexer { src, src_index: 0 }
    }

    fn advance(&mut self) {
        self.src_index += 1;
    }

    fn consume_token(&mut self, t: Token<'a>, bytes: usize) -> Token<'a> {
        for _ in 0..bytes {
            self.advance();
        }
        t
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.src_index).cloned()
    }

    fn peek_word(&self, ch_len: usize) -> &[u8] {
        &self.src[self.src_index..self.src_index + ch_len]
    }

    fn skip_whitespace(&mut self) -> Result<u8, Error> {
        while self.peek().ok_or(Error::Eof)?.is_ascii_whitespace() {
            self.src_index += 1;
        }
        Ok(self.src[self.src_index])
    }

    fn consume_number(&mut self) -> Result<Token<'a>, Error> {
        #[inline(always)]
        fn err<'b>(c: u8) -> Result<Token<'b>, Error> {
            Err(Error::InvalidNumberFormat(c as char))
        }

        let begin = self.src_index;
        let mut dot = false;
        let mut exponent = false;
        let mut prev: Option<u8> = None;

        while let Some(c) = self.peek() {
            if c == b'.' {
                if dot || exponent {
                    return err(c);
                }
                dot = true;
            } else if c == b'-' {
                if prev.is_some_and(|p| p != b'e') {
                    return err(c);
                }
            } else if c == b'e' {
                if exponent || prev.is_some_and(|p| p != b'.' && !p.is_ascii_digit()) {
                    return err(c);
                }
                exponent = true;
            } else if !c.is_ascii_digit() {
                if prev.is_some_and(|p| p == b'e' || p == b'.') {
                    return err(c);
                }
                // return number(self, begin);
                return Ok(Token::Number(&self.src[begin..self.src_index]));
            }
            self.advance();
            prev = Some(c);
        }
        Ok(Token::Number(&self.src[begin..self.src_index]))
    }

    fn parse_fn(&mut self, first_ch: u8) -> Result<Token<'a>, Error> {
        #[inline(always)]
        fn err<'b>(t: u8) -> Result<Token<'b>, Error> {
            Err(Error::InvalidChar(t as char))
        }

        match first_ch {
            b's' => {
                if self.peek_word(3) == b"sin" {
                    return Ok(self.consume_token(Token::Func(FuncType::Sin), 3));
                }
                if self.peek_word(4) == b"sqrt" {
                    return Ok(self.consume_token(Token::Func(FuncType::Sqrt), 4));
                }
                err(first_ch)
            }
            b'c' => {
                if self.peek_word(3) == b"cos" {
                    return Ok(self.consume_token(Token::Func(FuncType::Cos), 3));
                }
                err(first_ch)
            }
            b'l' => {
                if self.peek_word(3) == b"log" {
                    return Ok(self.consume_token(Token::Func(FuncType::Log), 3));
                }
                err(first_ch)
            }
            b'p' => {
                if self.peek_word(3) == b"pow" {
                    return Ok(self.consume_token(Token::Func(FuncType::Pow), 3));
                }
                err(first_ch)
            }
            _ => err(first_ch),
        }
    }
}

impl<'a> Scan<'a> for Lexer<'a> {
    fn scan(&mut self) -> Result<Token<'a>, Error> {
        let c = self.skip_whitespace()?;
        if c.is_ascii_digit() {
            return self.consume_number();
        }
        match c {
            b'(' => Ok(self.consume_token(Token::LeftParen, 1)),
            b')' => Ok(self.consume_token(Token::RightParen, 1)),
            b'+' => Ok(self.consume_token(Token::Plus, 1)),
            b'-' => Ok(self.consume_token(Token::Minus, 1)),
            b'*' => Ok(self.consume_token(Token::Mult, 1)),
            b'/' => Ok(self.consume_token(Token::Div, 1)),
            b',' => Ok(self.consume_token(Token::Comma, 1)),
            b'a' if self.peek_word(3) == b"ans" => Ok(self.consume_token(Token::Ans, 3)),
            ch => self.parse_fn(ch),
        }
    }
}

#[cfg(test)]
mod lexer_tests {
    use super::*;

    #[test]
    fn test_single_token() {
        let mut l = Lexer::new(b"(".as_slice());
        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::LeftParen);
        let eof = l.scan();
        assert!(eof.is_err());
        assert_eq!(eof.unwrap_err(), Error::Eof);
    }

    #[test]
    fn test_mult_no_other_chars() {
        let mut l = Lexer::new(b"*".as_slice());
        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Mult);
        let eof = l.scan();
        assert!(eof.is_err());
        assert_eq!(eof.unwrap_err(), Error::Eof);
    }

    #[test]
    fn test_mult_other_chars() {
        let mut l = Lexer::new(b"* 2".as_slice());
        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Mult);
    }

    #[test]
    fn test_single_integer_single_digit_number() {
        let mut l = Lexer::new(b"1".as_slice());
        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Number(b"1".as_slice()));
        let eof = l.scan();
        assert!(eof.is_err());
        assert_eq!(eof.unwrap_err(), Error::Eof);
    }

    #[test]
    fn test_single_integer_multiple_digits_number() {
        let mut l = Lexer::new(b"1234".as_slice());
        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Number(b"1234".as_slice()));
        let eof = l.scan();
        assert!(eof.is_err());
        assert_eq!(eof.unwrap_err(), Error::Eof);
    }

    #[test]
    fn test_single_floating_point() {
        let mut l = Lexer::new(b"1.25".as_slice());
        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Number(b"1.25".as_slice()));
        let eof = l.scan();
        assert!(eof.is_err());
        assert_eq!(eof.unwrap_err(), Error::Eof);
    }

    #[test]
    fn test_single_floating_point_exponential_lowercase() {
        let mut l = Lexer::new(b"1e2".as_slice());
        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Number(b"1e2".as_slice()));
        let eof = l.scan();
        assert!(eof.is_err());
        assert_eq!(eof.unwrap_err(), Error::Eof);
    }

    #[test]
    fn test_single_floating_point_negative_exponential_lowercase() {
        let mut l = Lexer::new(b"1e-2".as_slice());
        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Number(b"1e-2".as_slice()));
        let eof = l.scan();
        assert!(eof.is_err());
        assert_eq!(eof.unwrap_err(), Error::Eof);
    }

    #[test]
    fn test_single_invalid_integer() {
        let mut l = Lexer::new(b"1.4.e1".as_slice());
        let token = l.scan();
        assert!(token.is_err());
        assert_eq!(token.unwrap_err(), Error::InvalidNumberFormat('.'));
    }

    #[test]
    fn test_expression_with_parens() {
        let mut l = Lexer::new(b"(1.2 / 3.0e-1)".as_slice());

        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::LeftParen);

        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Number(b"1.2"));

        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Div);

        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Number(b"3.0e-1"));

        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::RightParen);

        let eof = l.scan();
        assert!(eof.is_err());
        assert_eq!(eof.unwrap_err(), Error::Eof);
    }

    #[test]
    fn test_expression() {
        let mut l = Lexer::new(b" 1.2 + 10 - 2e-3  ".as_slice());
        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Number(b"1.2"));

        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Plus);

        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Number(b"10"));

        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Minus);

        let token = l.scan();
        assert!(token.is_ok());
        assert_eq!(token.unwrap(), Token::Number(b"2e-3"));

        let eof = l.scan();
        assert!(eof.is_err());
        assert_eq!(eof.unwrap_err(), Error::Eof);
    }

    // #[test]
    // fn test_comma() {
    //     let mut l = Lexer::new(b",".as_slice());
    //     let t = l.scan();
    //     assert!(t.is_ok());
    //     assert_eq!(t.unwrap(), Token::Comma);
    //     assert_eq!(l.scan(), Err(Error::Eof))
    // }

    #[test]
    fn test_sin() {
        let mut l = Lexer::new(b"sin".as_slice());
        let t = l.scan();
        assert!(t.is_ok());
        assert_eq!(t.unwrap(), Token::Func(FuncType::Sin));
        assert_eq!(l.scan(), Err(Error::Eof))
    }

    #[test]
    fn test_cos() {
        let mut l = Lexer::new(b"cos".as_slice());
        let t = l.scan();
        assert!(t.is_ok());
        assert_eq!(t.unwrap(), Token::Func(FuncType::Cos));
        assert_eq!(l.scan(), Err(Error::Eof))
    }

    #[test]
    fn test_log() {
        let mut l = Lexer::new(b"log".as_slice());
        let t = l.scan();
        assert!(t.is_ok());
        assert_eq!(t.unwrap(), Token::Func(FuncType::Log));
        assert_eq!(l.scan(), Err(Error::Eof))
    }

    #[test]
    fn test_sqrt() {
        let mut l = Lexer::new(b"sqrt".as_slice());
        let t = l.scan();
        assert!(t.is_ok());
        assert_eq!(t.unwrap(), Token::Func(FuncType::Sqrt));
        assert_eq!(l.scan(), Err(Error::Eof))
    }

    #[test]
    fn test_pow() {
        let mut l = Lexer::new(b"pow".as_slice());
        let t = l.scan();
        assert!(t.is_ok());
        assert_eq!(t.unwrap(), Token::Func(FuncType::Pow));
        assert_eq!(l.scan(), Err(Error::Eof))
    }
}
