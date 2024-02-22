#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FuncType {
    Sqrt,
    Log,
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
    // TODO:
    // Func(FuncType)
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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Null,
    Number,
    Term,
    Factor,
    Unary,
    Group,
}

impl Priority {
    pub fn next(&self) -> Self {
        match self {
            Self::Null => Self::Number,
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
            Token::Number(_) => Priority::Number,
            Token::LeftParen => Priority::Group,
            Token::RightParen => Priority::Null,
            Token::Plus => Priority::Term,
            Token::Minus => Priority::Term,
            Token::Mult => Priority::Factor,
            Token::Div => Priority::Factor,
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

    fn consume_token<'b>(&mut self, t: Token<'b>, bytes: usize) -> Token<'b> {
        for _ in 0..bytes {
            self.advance();
        }
        t
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.src_index).cloned()
    }

    // fn peek_after(&self, after: usize) -> Option<u8> {
    //     self.src.get(self.src_index + after).cloned()
    // }

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
            invalid => Err(Error::InvalidChar(invalid as char)),
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
        println!("{:?}", token);
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
}
