use std::io::{self, Write};

use crate::{
    compiler::{Compile, Compiler, Error as CompilerError},
    lexer::Lexer,
    vm::{Error as VMError, VirtualMachine},
};

#[derive(Debug, Clone)]
pub enum ApplicationError {
    CompileError(CompilerError),
    VirtualmachineError(VMError),
}

impl std::fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ApplicationError {}

impl From<CompilerError> for ApplicationError {
    fn from(value: CompilerError) -> Self {
        Self::CompileError(value)
    }
}

impl From<VMError> for ApplicationError {
    fn from(value: VMError) -> Self {
        Self::VirtualmachineError(value)
    }
}

pub fn run(src: &[u8]) -> Result<f64, ApplicationError> {
    let mut lexer = Lexer::new(src);
    let mut compiler = Compiler::default();
    compiler.compile(&mut lexer)?;
    let mut vm = VirtualMachine::default();
    vm.interpret(compiler.opcodes()).map_err(|e| e.into())
}

pub fn run_repl() {
    loop {
        print!(">> ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        if input == "\n" || input == "\r\n" {
            continue;
        }
        let bytes = input.as_bytes();
        let mut lexer = Lexer::new(bytes);
        let mut compiler = Compiler::default();
        let mut vm = VirtualMachine::default();
        if let Err(e) = compiler.compile(&mut lexer) {
            eprintln!("Compiler error: {}", e);
            continue;
        }
        match vm.interpret(compiler.opcodes()) {
            Ok(value) => println!("$ {}", value),
            Err(e) => eprintln!("Virtual machine error: {}", e),
        };
    }
}
