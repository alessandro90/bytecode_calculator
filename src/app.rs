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

pub fn run(src: &'static [u8]) -> Result<f64, ApplicationError> {
    let mut lexer = Lexer::new(src);
    let mut compiler = Compiler::default();
    compiler.compile(&mut lexer)?;
    let mut vm = VirtualMachine::default();
    vm.interpret(compiler.opcodes()).map_err(|e| e.into())
}
