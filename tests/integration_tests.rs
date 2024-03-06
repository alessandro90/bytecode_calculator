extern crate vm_calculator;

use vm_calculator::{
    compiler::{self, Compile, Compiler},
    lexer::Lexer,
    vm::{self, VirtualMachine},
};

macro_rules! assert_float_eq {
    ($a:expr, $b:expr) => {
        assert!($a.abs() >= $b.abs() - 1e-6 && $a.abs() <= $b.abs() + 1e-6)
    };
    ($a:expr, $b:expr, $delta:expr) => {
        assert!($a.abs() >= $b.abs() - $delta && $a.abs() <= $b.abs() + $delta)
    };
}

#[test]
fn test_addition() {
    let mut lexer = Lexer::new(b"1 + 2");
    let mut compiler = Compiler::default();
    let compiled = compiler.compile(&mut lexer);
    assert!(compiled.is_ok());
    let mut vm = VirtualMachine::default();
    let res = vm.interpret(compiler.opcodes());
    assert!(res.is_ok());
    assert_float_eq!(res.unwrap(), 3.0f64);
}

#[test]
fn test_empty() {
    let mut lexer = Lexer::new(b"");
    let mut compiler = Compiler::default();
    let compiled = compiler.compile(&mut lexer);
    assert!(compiled.is_ok());
    let mut vm = VirtualMachine::default();
    let res = vm.interpret(compiler.opcodes());
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), vm::Error::EmptyStack);
}

#[test]
fn test_complex_expression() {
    let mut lexer = Lexer::new(b"1.5 * (4 - 10 / 2 - (-1 * 4e-1))"); // -0.899999
    let mut compiler = Compiler::default();
    let compiled = compiler.compile(&mut lexer);
    assert!(compiled.is_ok());
    let mut vm = VirtualMachine::default();
    let res = vm.interpret(compiler.opcodes());
    assert!(res.is_ok());
    assert_float_eq!(res.unwrap(), -0.89999f64, 1e-4f64);
}

#[test]
fn test_unterminated_group() {
    let mut lexer = Lexer::new(b"1 + (2 + 1 * (1 - 3)"); // -0.899999
    let mut compiler = Compiler::default();
    let compiled = compiler.compile(&mut lexer);
    assert!(compiled.is_err());
    assert_eq!(compiled.unwrap_err(), compiler::Error::UnterminedGroup);
}

#[test]
fn test_empty_group() {
    let mut lexer = Lexer::new(b"1 + ()"); // -0.899999
    let mut compiler = Compiler::default();
    let compiled = compiler.compile(&mut lexer);
    assert!(compiled.is_err());
    assert_eq!(
        compiled.unwrap_err(),
        compiler::Error::InvalidTokenBefore {
            prev: ")".to_string(),
            current: None
        }
    );
}

#[test]
fn test_expression_multiple_functions_invalid_pow() {
    let mut lexer =
        Lexer::new(b"-(cos(sqrt(144) * sin(1 + pow(-1, -1.5))) * 1 / sqrt(44) * 0.005e2)"); // -0.899999
    let mut compiler = Compiler::default();
    let compiled = compiler.compile(&mut lexer);
    assert!(compiled.is_ok());
    let mut vm = VirtualMachine::default();
    let res = vm.interpret(compiler.opcodes());
    assert!(res.is_err());
}

#[test]
fn test_expression_multiple_functions() {
    let mut lexer =
        Lexer::new(b"-(cos(sqrt(144) * sin(1 + pow(-1, -2))) * 1 / sqrt(44) * 0.005e2)"); // -0.899999
    let mut compiler = Compiler::default();
    let compiled = compiler.compile(&mut lexer);
    assert!(compiled.is_ok());
    let mut vm = VirtualMachine::default();
    let res = vm.interpret(compiler.opcodes());
    assert!(res.is_ok());
    println!("{}; {}", res.unwrap(), 0.00632468f64);
    assert_float_eq!(res.unwrap(), 0.00632468f64, 1e-8);
}
