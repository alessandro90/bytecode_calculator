use crate::{
    lexer::{Error as LexerError, FuncType, Priority, Scan, Token},
    misc::i8_as_u8,
};

pub trait Compile {
    fn compile(&mut self, lexer: &mut impl Scan) -> Result<(), Error>;
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
    InvalidToken(String),
    MissingFunctionParen,
    MissingCommaInFunctionCall,
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
    Number = 0,
    Plus = 1,
    Minus = 2,
    Mult = 3,
    Div = 4,
    Negate = 5,
    Func = 6,
    Ans = 7,
    NumberI8 = 8,
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
            6 => Ok(Op::Func),
            7 => Ok(Op::Ans),
            8 => Ok(Op::NumberI8),
            x => Err(InvalidOpcode(x)),
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
        self.advance(lexer)?;
        self.expression(lexer, Priority::Term)?;

        match self.current_token {
            Some(t) => Err(Error::InvalidToken(t.into())),
            None => Ok(()),
        }
    }
}

const INITIAL_CHUNK_SIZE: usize = 100;

impl Default for Compiler {
    fn default() -> Self {
        let chunk = Vec::with_capacity(INITIAL_CHUNK_SIZE);
        Self {
            chunk,
            prev_token: None,
            current_token: None,
        }
    }
}

impl Compiler {
    pub fn opcodes(&self) -> &[u8] {
        &self.chunk
    }

    pub fn reset(&mut self) {
        self.chunk.clear();
        self.prev_token = None;
        self.current_token = None;
    }

    fn expression(&mut self, lexer: &mut impl Scan, priority: Priority) -> CompilerResult {
        self.advance(lexer)?;
        if let Some(prev) = self.prev_token {
            match prev {
                Token::Minus => self.emit_unary(lexer),
                Token::Number(num_str) => self.emit_number(num_str.into()),
                Token::LeftParen => self.parse_group(lexer),
                Token::Func(func_type) => self.parse_fn(lexer, func_type),
                Token::Ans => {
                    self.chunk.push(Op::Ans.into());
                    Ok(())
                }
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

    fn parse_binary(&mut self, lexer: &mut impl Scan, tok: Token) -> CompilerResult {
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

    fn advance(&mut self, lexer: &mut impl Scan) -> CompilerResult {
        self.prev_token = self.current_token;
        let tok = lexer.scan();
        self.current_token = tok.ok();
        tok.map_or_else(
            |e| {
                if e != LexerError::Eof {
                    Err(e.into())
                } else {
                    Ok(())
                }
            },
            |_| Ok(()),
        )
    }

    fn consume(&mut self, lexer: &mut impl Scan, target: Token, err: Error) -> CompilerResult {
        if self.current_token.is_some_and(|tok| tok == target) {
            self.advance(lexer)
        } else {
            Err(err)
        }
    }

    fn parse_group(&mut self, lexer: &mut impl Scan) -> CompilerResult {
        self.expression(lexer, Priority::Term)?;
        self.consume(lexer, Token::RightParen, Error::UnterminedGroup)?;
        Ok(())
    }

    fn parse_fn(&mut self, lexer: &mut impl Scan, func_type: FuncType) -> CompilerResult {
        self.consume(lexer, Token::LeftParen, Error::MissingFunctionParen)?;
        let arity = func_type.arity();
        if arity > 0 {
            for _ in 0..arity - 1 {
                self.expression(lexer, Priority::Term)?;
                self.consume(lexer, Token::Comma, Error::MissingCommaInFunctionCall)?;
            }
            self.expression(lexer, Priority::Term)?;
        }
        self.consume(lexer, Token::RightParen, Error::MissingFunctionParen)?;
        self.chunk.push(Op::Func.into());
        self.chunk.push(func_type.into());
        Ok(())
    }

    fn emit_unary(&mut self, lexer: &mut impl Scan) -> CompilerResult {
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
                if (i8::MIN as f64..=i8::MAX as f64).contains(&n) && n.fract() == 0.0 {
                    self.chunk.push(Op::NumberI8.into());
                    self.chunk.push(i8_as_u8(n as i8));
                } else {
                    self.chunk.push(Op::Number.into());
                    let old_len = self.chunk.len();
                    let f64_bytes = 8;
                    self.chunk.resize(old_len + f64_bytes, 0);
                    let p = self.chunk[old_len..].as_mut_ptr();
                    let pn = std::ptr::from_ref(&n) as *const u8;
                    unsafe { pn.copy_to_nonoverlapping(p, f64_bytes) };
                }
                Ok(())
            }
            None => Err(Error::InvalidNumber(Vec::from(digits))),
        }
    }
}

#[cfg(test)]
mod compiler_tests {
    use crate::misc::u8_as_i8;

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

    fn parse_i8(n: u8) -> f64 {
        u8_as_i8(n) as f64
    }

    fn eight_bytes_num(start: usize) -> std::ops::RangeInclusive<usize> {
        start..=start + 7
    }

    #[test]
    fn test_single_number() {
        let mut lexer = MockLexer::new(vec![Token::Number(b"1".as_slice().into())]);
        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[1]);
        assert_eq!(float, 1.0);
    }

    #[test]
    fn test_single_negative_number() {
        let mut lexer = MockLexer::new(vec![Token::Minus, Token::Number(b"1".as_slice().into())]);
        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[1]);
        assert_eq!(float, 1.0);
        assert_eq!(compiler.chunk[2], Op::Negate.into());
    }

    #[test]
    fn test_sum_of_two_numbers() {
        let mut lexer = MockLexer::new(vec![
            Token::Number(b"1".as_slice().into()),
            Token::Plus,
            Token::Number(b"2".as_slice().into()),
        ]);
        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[1]);
        assert_eq!(float, 1.0);

        assert_eq!(compiler.chunk[2], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[3]);
        assert_eq!(float, 2.0);

        assert_eq!(compiler.chunk[4], Op::Plus.into());
    }

    #[test]
    fn test_grouping() {
        let mut lexer = MockLexer::new(vec![
            Token::Number(b"2".as_slice().into()),
            Token::Mult,
            Token::LeftParen,
            Token::Number(b"1".as_slice().into()),
            Token::Plus,
            Token::Number(b"1.5".as_slice().into()),
            Token::RightParen,
        ]);
        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[1]);
        assert_eq!(float, 2.0);

        assert_eq!(compiler.chunk[2], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[3]);
        assert_eq!(float, 1.0);

        assert_eq!(compiler.chunk[4], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[eight_bytes_num(5)]);
        assert_eq!(float, 1.5);

        assert_eq!(compiler.chunk[13], Op::Plus.into());
    }

    #[test]
    fn test_long_complex_expression() {
        // 1 + (2e-3 / 4 + 2) * 2 - 1
        let mut lexer = MockLexer::new(vec![
            Token::Number(b"1".as_slice().into()),
            Token::Plus,
            Token::LeftParen,
            Token::Number(b"2e-3".as_slice().into()),
            Token::Div,
            Token::Number(b"4".as_slice().into()),
            Token::Plus,
            Token::Number(b"2".as_slice().into()),
            Token::RightParen,
            Token::Mult,
            Token::Number(b"2".as_slice().into()),
            Token::Minus,
            Token::Number(b"1".as_slice().into()),
        ]);

        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());

        assert_eq!(compiler.chunk[0], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[1]);
        assert_eq!(float, 1.0);

        assert_eq!(compiler.chunk[2], Op::Number.into());
        let (_, float) = parse_number(&compiler.chunk[eight_bytes_num(3)]);
        assert_eq!(float, 2e-3);

        assert_eq!(compiler.chunk[11], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[12]);
        assert_eq!(float, 4.0);

        assert_eq!(compiler.chunk[13], Op::Div.into());

        assert_eq!(compiler.chunk[14], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[15]);
        assert_eq!(float, 2.0);

        assert_eq!(compiler.chunk[16], Op::Plus.into());

        assert_eq!(compiler.chunk[17], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[18]);
        assert_eq!(float, 2.0);

        assert_eq!(compiler.chunk[19], Op::Mult.into());

        assert_eq!(compiler.chunk[20], Op::Plus.into());

        assert_eq!(compiler.chunk[21], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[22]);
        assert_eq!(float, 1.0);

        assert_eq!(compiler.chunk[23], Op::Minus.into());
    }

    #[test]
    fn test_sin() {
        let mut lexer = MockLexer::new(vec![
            Token::Func(FuncType::Sin),
            Token::LeftParen,
            Token::Number(b"4".as_slice().into()),
            Token::RightParen,
        ]);

        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[1]);
        assert_eq!(float, 4.0);
        assert_eq!(compiler.chunk[2], Op::Func.into());
        assert_eq!(compiler.chunk[3], FuncType::Sin.into());
    }

    #[test]
    fn test_cos() {
        let mut lexer = MockLexer::new(vec![
            Token::Func(FuncType::Cos),
            Token::LeftParen,
            Token::Number(b"4".as_slice().into()),
            Token::RightParen,
        ]);

        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[1]);
        assert_eq!(float, 4.0);
        assert_eq!(compiler.chunk[2], Op::Func.into());
        assert_eq!(compiler.chunk[3], FuncType::Cos.into());
    }

    #[test]
    fn test_log() {
        let mut lexer = MockLexer::new(vec![
            Token::Func(FuncType::Log),
            Token::LeftParen,
            Token::Number(b"4".as_slice().into()),
            Token::RightParen,
        ]);

        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[1]);
        assert_eq!(float, 4.0);
        assert_eq!(compiler.chunk[2], Op::Func.into());
        assert_eq!(compiler.chunk[3], FuncType::Log.into());
    }

    #[test]
    fn test_sqrt() {
        let mut lexer = MockLexer::new(vec![
            Token::Func(FuncType::Sqrt),
            Token::LeftParen,
            Token::Number(b"4".as_slice().into()),
            Token::RightParen,
        ]);

        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.chunk[0], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[1]);
        assert_eq!(float, 4.0);
        assert_eq!(compiler.chunk[2], Op::Func.into());
        assert_eq!(compiler.chunk[3], FuncType::Sqrt.into());
    }

    #[test]
    fn test_pow() {
        let mut lexer = MockLexer::new(vec![
            Token::Func(FuncType::Pow),
            Token::LeftParen,
            Token::Number(b"3".as_slice().into()),
            Token::Comma,
            Token::Number(b"2".as_slice().into()),
            Token::RightParen,
        ]);

        let mut compiler = Compiler::default();
        let res = compiler.compile(&mut lexer);
        assert!(res.is_ok());
        assert_eq!(compiler.opcodes()[0], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[1]);
        assert_eq!(float, 3.0);

        assert_eq!(compiler.chunk[2], Op::NumberI8.into());
        let float = parse_i8(compiler.chunk[3]);
        assert_eq!(float, 2.0);
        assert_eq!(compiler.chunk[4], Op::Func.into());
        assert_eq!(compiler.chunk[5], FuncType::Pow.into());
    }
}
