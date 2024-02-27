use crate::lexer::{Error as LexerError, Priority, Scan, Token};

pub trait Compile<'a> {
    fn compile(&mut self, lexer: &mut impl Scan<'a>) -> Result<(), Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    FromLexer(LexerError),
    InvalidNumber(Vec<u8>),
    InvalidTokenBefore {
        prev: String,
        current: Option<String>,
    },
    UnterminedGroup,
    MissingExpression,
    InvalidToken(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

impl From<LexerError> for Error {
    fn from(value: LexerError) -> Self {
        Self::FromLexer(value)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    // NOTE: a number is a 64 bit float/u64. Maybe if
    // it is an integer and for e.g. in [0, 255] could
    // make a Op::SmallNumber that just needs an extra byte
    Number = 0,
    Plus = 1,
    Minus = 2,
    Mult = 3,
    Div = 4,
    Negate = 5,
}

impl From<Op> for u8 {
    fn from(value: Op) -> Self {
        value as u8
    }
}

#[derive(Debug)]
pub struct InvalidOpcode(u8);

impl TryFrom<u8> for Op {
    type Error = InvalidOpcode;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Op::Number),
            1 => Ok(Op::Plus),
            2 => Ok(Op::Minus),
            3 => Ok(Op::Mult),
            4 => Ok(Op::Div),
            5 => Ok(Op::Negate),
            x => Err(InvalidOpcode(x)),
        }
    }
}

pub struct Compiler<'a> {
    prev_token: Option<Token<'a>>,
    current_token: Option<Token<'a>>,
    chunk: Vec<u8>,
}

pub type CompilerResult = Result<(), Error>;

impl<'a> Compile<'a> for Compiler<'a> {
    fn compile(&mut self, lexer: &mut impl Scan<'a>) -> CompilerResult {
        self.advance(lexer)?;
        self.expression(lexer, Priority::Term)?;
        match self.current_token {
            Some(t) => {
                panic!("Invalid last token: {:?}", t);
            }
            None => Ok(()),
        }
    }
}

const INITIAL_CHUNK_SIZE: usize = 100;

impl<'a> Default for Compiler<'a> {
    fn default() -> Self {
        let chunk = Vec::with_capacity(INITIAL_CHUNK_SIZE);
        Self {
            chunk,
            prev_token: None,
            current_token: None,
        }
    }
}

impl<'a> Compiler<'a> {
    pub fn opcodes(&self) -> &[u8] {
        &self.chunk
    }

    pub fn expression(&mut self, lexer: &mut impl Scan<'a>, priority: Priority) -> CompilerResult {
        self.advance(lexer)?;
        if let Some(prev) = self.prev_token {
            match prev {
                Token::Minus => self.emit_unary(lexer),
                Token::Number(num_str) => self.emit_number(num_str),
                Token::LeftParen => self.parse_group(lexer),
                t => Err(Error::InvalidTokenBefore {
                    prev: t.into(),
                    current: self.current_token.map(|tok| tok.into()),
                }),
            }?;
        }
        while self.current_token.is_some_and(|t| t.priority() >= priority) {
            self.advance(lexer)?;
            if let Some(prev) = self.prev_token {
                match prev {
                    Token::Div | Token::Plus | Token::Mult | Token::Minus => {
                        self.parse_binary(lexer, prev)
                    }
                    t => Err(Error::InvalidToken(t.into())),
                }?;
            }
        }
        Ok(())
    }

    fn parse_binary(&mut self, lexer: &mut impl Scan<'a>, tok: Token<'a>) -> CompilerResult {
        self.expression(lexer, tok.priority().next())?;
        match tok {
            Token::Minus => {
                self.chunk.push(Op::Minus.into());
                Ok(())
            }
            Token::Plus => {
                self.chunk.push(Op::Plus.into());
                Ok(())
            }
            Token::Div => {
                self.chunk.push(Op::Div.into());
                Ok(())
            }
            Token::Mult => {
                self.chunk.push(Op::Mult.into());
                Ok(())
            }
            t => Err(Error::InvalidToken(t.into())),
        }
    }

    fn advance(&mut self, lexer: &mut impl Scan<'a>) -> CompilerResult {
        self.prev_token = self.current_token;
        let tok = lexer.scan();
        self.current_token = tok.ok();
        match tok {
            Ok(_) => Ok(()),
            Err(e) => {
                if e != LexerError::Eof {
                    Err(e.into())
                } else {
                    Ok(())
                }
            }
        }
    }

    fn consume(
        &mut self,
        lexer: &mut impl Scan<'a>,
        target: Token<'a>,
        err: Error,
    ) -> CompilerResult {
        if self.current_token.is_some_and(|tok| tok == target) {
            self.advance(lexer)
        } else {
            Err(err)
        }
    }

    fn parse_group(&mut self, lexer: &mut impl Scan<'a>) -> CompilerResult {
        self.expression(lexer, Priority::Term)?;
        self.consume(lexer, Token::RightParen, Error::UnterminedGroup)?;
        Ok(())
    }

    fn emit_unary(&mut self, lexer: &mut impl Scan<'a>) -> CompilerResult {
        self.expression(lexer, Priority::Unary)?;
        self.chunk.push(Op::Negate.into());
        Ok(())
    }

    fn emit_number(&mut self, digits: &[u8]) -> CompilerResult {
        let num = std::str::from_utf8(digits)
            .ok()
            .and_then(|chars| chars.parse::<f64>().ok());

        match num {
            Some(n) => {
                self.chunk.push(Op::Number.into());
                let as_u64 = n.to_bits();
                for i in 0u64..8u64 {
                    self.chunk.push(((as_u64 >> (i * 8)) & 0xFF) as u8);
                }
                Ok(())
            }
            None => Err(Error::InvalidNumber(Vec::from(digits))),
        }
    }
}

#[cfg(test)]
mod compiler_tests {
    use super::*;

    struct MockLexer<'a> {
        scan_results: Vec<Token<'a>>,
        index: usize,
    }

    impl<'a> MockLexer<'a> {
        fn new(tokens: Vec<Token<'a>>) -> Self {
            Self {
                scan_results: tokens,
                index: 0,
            }
        }
    }

    impl<'a> Scan<'a> for MockLexer<'a> {
        fn scan(&mut self) -> Result<Token<'a>, LexerError> {
            if self.index < self.scan_results.len() {
                let tok = self.scan_results[self.index];
                self.index += 1;
                Ok(tok)
            } else {
                Err(LexerError::Eof)
            }
        }
    }

    fn parse_number(bytes: &[u8]) -> (u64, f64) {
        let integer = {
            let mut res = 0;
            for (i, &b) in bytes.iter().enumerate() {
                res |= (b as u64) << (i as u64 * 8);
            }
            res
        };
        let float = f64::from_bits(integer);
        (integer, float)
    }

    #[test]
    fn test_single_number() {
        let mut lexer = MockLexer::new(vec![Token::Number(b"1")]);
        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::Number.into());
        assert!(compiler.chunk.len() >= 9);
        let (_, float) = parse_number(&compiler.chunk[1..=8]);
        assert_eq!(float, 1.0);
    }

    #[test]
    fn test_single_negative_number() {
        let mut lexer = MockLexer::new(vec![Token::Minus, Token::Number(b"1")]);
        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[1..=8]);
        assert_eq!(float, 1.0);
        assert_eq!(compiler.chunk[9], Op::Negate.into());
    }

    #[test]
    fn test_sum_of_two_numbers() {
        let mut lexer = MockLexer::new(vec![Token::Number(b"1"), Token::Plus, Token::Number(b"2")]);
        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[1..=8]);
        assert_eq!(float, 1.0);

        assert_eq!(compiler.chunk[9], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[10..=17]);
        assert_eq!(float, 2.0);

        assert_eq!(compiler.chunk[18], Op::Plus.into());
    }

    #[test]
    fn test_grouping() {
        let mut lexer = MockLexer::new(vec![
            Token::Number(b"2"),
            Token::Mult,
            Token::LeftParen,
            Token::Number(b"1"),
            Token::Plus,
            Token::Number(b"1.5"),
            Token::RightParen,
        ]);
        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[1..=8]);
        assert_eq!(float, 2.0);

        assert_eq!(compiler.chunk[9], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[10..=17]);
        assert_eq!(float, 1.0);

        assert_eq!(compiler.chunk[18], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[19..=26]);
        assert_eq!(float, 1.5);

        assert_eq!(compiler.chunk[27], Op::Plus.into());
    }

    #[test]
    fn test_long_complex_expression() {
        // 1 + (2e-3 / 4 + 2) * 2 - 1
        let mut lexer = MockLexer::new(vec![
            Token::Number(b"1"),
            Token::Plus,
            Token::LeftParen,
            Token::Number(b"2e-3"),
            Token::Div,
            Token::Number(b"4"),
            Token::Plus,
            Token::Number(b"2"),
            Token::RightParen,
            Token::Mult,
            Token::Number(b"2"),
            Token::Minus,
            Token::Number(b"1"),
        ]);

        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());

        assert_eq!(compiler.chunk[0], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[1..=8]);
        assert_eq!(float, 1.0);

        assert_eq!(compiler.chunk[9], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[10..=17]);
        assert_eq!(float, 2e-3);

        assert_eq!(compiler.chunk[18], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[19..=26]);
        assert_eq!(float, 4.0);

        assert_eq!(compiler.chunk[27], Op::Div.into());

        assert_eq!(compiler.chunk[28], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[29..=36]);
        assert_eq!(float, 2.0);

        assert_eq!(compiler.chunk[37], Op::Plus.into());

        assert_eq!(compiler.chunk[38], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[39..=46]);
        assert_eq!(float, 2.0);

        assert_eq!(compiler.chunk[47], Op::Mult.into());

        assert_eq!(compiler.chunk[48], Op::Plus.into());

        assert_eq!(compiler.chunk[49], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[50..=57]);
        assert_eq!(float, 1.0);

        assert_eq!(compiler.chunk[58], Op::Minus.into());
    }
}
