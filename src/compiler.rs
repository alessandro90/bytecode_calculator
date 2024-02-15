use crate::lexer::{Error as LexerError, Priority, Scan, Token};

pub trait Compile {
    fn compile(&mut self, lexer: &mut impl Scan) -> Result<(), Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    FromLexer(LexerError),
    InvalidOpCode(u8),
    InvalidNumber(Vec<u8>),
    InvalidTokenBefore { prev: Token, current: Option<Token> },
    UnterminedGroup,
    EmptyGroup,
    MissingExpression,
    InvalidToken(Token),
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
pub enum Op {
    // NOTE: a number is a 64 bit float/u64. Maybe if
    // it is an integer and for e.g. in [0, 255] could
    // make a Op::SmallNumber that just needs an extra byte
    Number = 0,
    Add = 1,
    Subtract = 2,
    Multiply = 3,
    Divide = 4,
    UnaryMinus = 6,
}

impl From<Op> for u8 {
    fn from(value: Op) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for Op {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Op::Number),
            1 => Ok(Op::Add),
            2 => Ok(Op::Subtract),
            3 => Ok(Op::Multiply),
            4 => Ok(Op::Divide),
            5 => Ok(Op::UnaryMinus),
            x => Err(Error::InvalidOpCode(x)),
        }
    }
}

pub struct Compiler {
    prev_token: Option<Token>,
    current_token: Option<Token>,
    chunk: Vec<u8>,
}

pub type CompilerResult = Result<(), Error>;

impl Compile for Compiler {
    fn compile(&mut self, lexer: &mut impl Scan) -> CompilerResult {
        self.advance(lexer);
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

impl Compiler {
    pub fn new() -> Self {
        let chunk = Vec::with_capacity(INITIAL_CHUNK_SIZE);
        Self {
            chunk,
            prev_token: None,
            current_token: None,
        }
    }

    pub fn opcodes(&self) -> &[u8] {
        &self.chunk
    }

    pub fn expression(&mut self, lexer: &mut impl Scan, priority: Priority) -> CompilerResult {
        self.advance(lexer);
        if let Some(prev) = self.prev_token {
            match prev {
                Token::Minus => self.emit_unary(lexer),
                Token::Number(num_str) => self.emit_number(num_str),
                Token::LeftParen => self.parse_group(lexer),
                t => Err(Error::InvalidTokenBefore {
                    prev: t,
                    current: self.current_token,
                }),
            }?;
        }
        while self.current_token.is_some_and(|t| t.priority() >= priority) {
            self.advance(lexer);
            if let Some(prev) = self.prev_token {
                match prev {
                    Token::Div | Token::Plus | Token::Mult | Token::Minus => {
                        self.parse_binary(lexer, prev)
                    }
                    t => Err(Error::InvalidToken(t)),
                }?;
            }
        }
        Ok(())
    }

    fn parse_binary(&mut self, lexer: &mut impl Scan, tok: Token) -> CompilerResult {
        self.expression(lexer, tok.priority().next())?;
        match tok {
            Token::Minus => {
                self.chunk.push(Op::Subtract.into());
                Ok(())
            }
            Token::Plus => {
                self.chunk.push(Op::Add.into());
                Ok(())
            }
            Token::Div => {
                self.chunk.push(Op::Divide.into());
                Ok(())
            }
            Token::Mult => {
                self.chunk.push(Op::Multiply.into());
                Ok(())
            }
            t => Err(Error::InvalidToken(t)),
        }
    }

    fn advance(&mut self, lexer: &mut impl Scan) {
        self.prev_token = self.current_token;
        let tok = lexer.scan();
        self.current_token = tok.ok();
    }

    fn consume(&mut self, lexer: &mut impl Scan, target: Token, err: Error) -> CompilerResult {
        let tok = lexer.scan()?;
        if tok != target {
            Err(err)
        } else {
            Ok(())
        }
    }

    fn parse_group(&mut self, lexer: &mut impl Scan) -> CompilerResult {
        let chunk_len_before = self.chunk.len();
        self.expression(lexer, Priority::Term)?;
        self.consume(lexer, Token::RightParen, Error::UnterminedGroup)?;
        let chunk_len_after = self.chunk.len();
        if chunk_len_before == chunk_len_after {
            Err(Error::EmptyGroup)
        } else {
            Ok(())
        }
    }

    fn emit_unary(&mut self, lexer: &mut impl Scan) -> CompilerResult {
        self.expression(lexer, Priority::Unary)?;
        self.chunk.push(Op::UnaryMinus.into());
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

    struct MockLexer {
        scan_results: Vec<Token>,
        index: usize,
    }

    impl MockLexer {
        fn new(tokens: Vec<Token>) -> Self {
            Self {
                scan_results: tokens,
                index: 0,
            }
        }
    }

    impl Scan for MockLexer {
        fn scan(&mut self) -> Result<Token, LexerError> {
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
        let mut compiler = Compiler::new();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::Number.into());
        assert!(compiler.chunk.len() >= 9);
        let (_, float) = parse_number(&compiler.chunk[1..=8]);
        assert_eq!(float, 1.0);
    }

    #[test]
    fn test_sum_of_two_numbers() {
        let mut lexer = MockLexer::new(vec![Token::Number(b"1"), Token::Plus, Token::Number(b"2")]);
        let mut compiler = Compiler::new();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[1..=8]);
        assert_eq!(float, 1.0);

        assert_eq!(compiler.chunk[9], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[10..=17]);
        assert_eq!(float, 2.0);

        assert_eq!(compiler.chunk[18], Op::Add.into());
    }
}
