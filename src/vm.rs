use std::fmt::Display;

use crate::{compiler::Op, lexer::FuncType};

const STACK_INITIAL_CAPACITY: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FuncArgs {
    Arg1(f64),
    Arg2(f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Error {
    DivisionByZero,
    EmptyStack,
    InvalidFunctionArgs {
        func_type: FuncType,
        func_args: FuncArgs,
    },
    AnsNotAvailable,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

pub struct VirtualMachine {
    instruction_pointer: usize,
    stack: Vec<f64>,
    ans: Option<f64>,
}

impl Default for VirtualMachine {
    fn default() -> Self {
        Self {
            instruction_pointer: 0,
            stack: Vec::with_capacity(STACK_INITIAL_CAPACITY),
            ans: None,
        }
    }
}

impl VirtualMachine {
    pub fn reset(&mut self, ans: Option<f64>) {
        self.instruction_pointer = 0;
        self.stack.clear();
        self.ans = ans;
    }

    pub fn new(ans: Option<f64>) -> Self {
        Self {
            ans,
            ..Self::default()
        }
    }

    pub fn interpret(&mut self, opcodes: &[u8]) -> Result<f64, Error> {
        while self.instruction_pointer < opcodes.len() {
            let byte = self.advance_instruction(opcodes);
            let op = Op::try_from(byte)
                .unwrap_or_else(|e| panic!("Invalid opcode {}, error: {:?}", byte, e));
            match op {
                Op::Number => self.number(opcodes),
                Op::NumberI8 => self.number_i8(opcodes),
                Op::Negate => self.negate(),
                Op::Minus | Op::Plus | Op::Mult | Op::Div => self.binary(op)?,
                Op::Func => self.function(opcodes)?,
                Op::Ans => self.load_ans()?,
            };
        }
        // reset for further calls
        self.stack.pop().ok_or(Error::EmptyStack)
    }

    fn load_ans(&mut self) -> Result<(), Error> {
        match self.ans {
            Some(ans) => {
                self.stack.push(ans);
                Ok(())
            }
            None => Err(Error::AnsNotAvailable),
        }
    }

    fn function(&mut self, opcodes: &[u8]) -> Result<(), Error> {
        let func_type = self.advance_instruction(opcodes);
        let func_type = FuncType::try_from(func_type)
            .unwrap_or_else(|e| panic!("Invalid byte function code {:?}", e));
        match func_type {
            FuncType::Log => {
                let arg = self.stack_pop("Missing function argument (Log)");
                let val = arg.ln();
                if !val.is_finite() {
                    return Err(Error::InvalidFunctionArgs {
                        func_type,
                        func_args: FuncArgs::Arg1(arg),
                    });
                }
                self.stack.push(val);
            }
            FuncType::Sin => {
                let arg = self.stack_pop("Missing function argument (Log)");
                self.stack.push(arg.sin());
            }
            FuncType::Cos => {
                let arg = self.stack_pop("Missing function argument (Log)");
                self.stack.push(arg.cos());
            }
            FuncType::Sqrt => {
                let arg = self.stack_pop("Missing function argument (Log)");
                let val = arg.sqrt();
                if val.is_nan() {
                    return Err(Error::InvalidFunctionArgs {
                        func_type,
                        func_args: FuncArgs::Arg1(arg),
                    });
                }
                self.stack.push(val);
            }
            FuncType::Pow => {
                let exponent = self.stack_pop("Missing exponent in pow");
                let base = self.stack_pop("Missing base in pow");
                self.stack.push(base.powf(exponent));
            }
        };
        Ok(())
    }

    #[inline(always)]
    fn stack_pop(&mut self, msg: &'static str) -> f64 {
        self.stack.pop().expect(msg)
    }

    fn advance_instruction(&mut self, opcodes: &[u8]) -> u8 {
        self.instruction_pointer += 1;
        opcodes[self.instruction_pointer - 1]
    }

    fn advance_instruction_by<'a>(&mut self, opcodes: &'a [u8], n_bytes: usize) -> &'a [u8] {
        let bytes = &opcodes[self.instruction_pointer..self.instruction_pointer + n_bytes];
        self.instruction_pointer += n_bytes;
        bytes
    }

    fn number(&mut self, opcodes: &[u8]) {
        let bytes = self.advance_instruction_by(opcodes, 8);
        let num = parse_number(bytes);
        self.stack.push(num);
    }

    fn number_i8(&mut self, opcodes: &[u8]) {
        let byte = self.advance_instruction(opcodes);
        self.stack
            .push(unsafe { std::mem::transmute::<u8, i8>(byte) } as f64);
    }

    fn negate(&mut self) {
        let n = self.stack_pop("Empty stack in negate function");
        self.stack.push(-n);
    }

    fn binary(&mut self, op: Op) -> Result<(), Error> {
        let a = self.stack_pop("Empty stack in binary. First operand");
        let b = self.stack_pop("Empty stack in binary. Second operand");
        let n = match op {
            Op::Div => {
                if a == 0.0 {
                    return Err(Error::DivisionByZero);
                }
                b / a
            }
            Op::Plus => b + a,
            Op::Mult => b * a,
            Op::Minus => b - a,
            _ => panic!("Invalid binary operation {:?}", op),
        };
        self.stack.push(n);
        Ok(())
    }
}

fn parse_number(bytes: &[u8]) -> f64 {
    let integer = {
        let mut res = 0;
        for (i, &b) in bytes.iter().enumerate() {
            res |= (b as u64) << (i as u64 * 8);
        }
        res
    };
    f64::from_bits(integer)
}

#[cfg(test)]
mod vm_tests {
    use crate::{compiler::Op, lexer::FuncType};

    use super::VirtualMachine;

    macro_rules! assert_float_eq {
        ($a:expr, $b:expr) => {
            assert!($a.abs() >= $b.abs() - 1e-6 && $a.abs() <= $b.abs() + 1e-6)
        };
        ($a:expr, $b:expr, $delta:expr) => {
            assert!($a.abs() >= $b.abs() - $delta && $a.abs() <= $b.abs() + $delta)
        };
    }

    fn number_to_bytes(n: f64) -> Vec<u8> {
        let as_u64 = n.to_bits();
        let mut bytes = Vec::with_capacity(8);
        for i in 0u64..8u64 {
            bytes.push(((as_u64 >> (i * 8)) & 0xFF) as u8);
        }
        bytes
    }

    #[test]
    fn test_single_number() {
        let mut vm = VirtualMachine::default();
        let mut opcodes: Vec<_> = vec![Op::Number.into()];
        let n = 1.0f64;
        opcodes.append(&mut number_to_bytes(n));
        let res = vm.interpret(&opcodes);
        assert!(res.is_ok());
        assert_float_eq!(res.unwrap(), n);
    }

    #[test]
    fn test_negation() {
        let mut vm = VirtualMachine::default();
        let mut opcodes = vec![Op::Number.into()];
        let n = 1.0f64;
        opcodes.append(&mut number_to_bytes(n));
        opcodes.push(Op::Negate.into());
        let res = vm.interpret(&opcodes);
        assert!(res.is_ok());
        assert_float_eq!(res.unwrap(), -n);
    }

    #[test]
    fn test_addition() {
        let mut vm = VirtualMachine::default();

        let mut opcodes = vec![Op::Number.into()];
        let a = 1.0f64;
        opcodes.append(&mut number_to_bytes(a));

        opcodes.push(Op::Number.into());
        let b = 3.0f64;
        opcodes.append(&mut number_to_bytes(b));

        opcodes.push(Op::Plus.into());

        let res = vm.interpret(&opcodes);
        assert!(res.is_ok());
        assert_float_eq!(res.unwrap(), a + b);
    }

    #[test]
    fn test_complex_expression() {
        // -(1 + 2) * 3 / (2 * 3 - (1 / 2)) + 1 = -0.6363
        let mut vm = VirtualMachine::default();
        let mut opcodes = vec![Op::Number.into()];
        opcodes.append(&mut number_to_bytes(1.0));
        opcodes.push(Op::Number.into());
        opcodes.append(&mut number_to_bytes(2.0));

        opcodes.push(Op::Plus.into());

        opcodes.push(Op::Negate.into());

        opcodes.push(Op::Number.into());
        opcodes.append(&mut number_to_bytes(3.0));

        opcodes.push(Op::Mult.into());

        opcodes.push(Op::Number.into());
        opcodes.append(&mut number_to_bytes(2.0));
        opcodes.push(Op::Number.into());
        opcodes.append(&mut number_to_bytes(3.0));

        opcodes.push(Op::Mult.into());

        opcodes.push(Op::Number.into());
        opcodes.append(&mut number_to_bytes(1.0));
        opcodes.push(Op::Number.into());
        opcodes.append(&mut number_to_bytes(2.0));

        opcodes.push(Op::Div.into());

        opcodes.push(Op::Minus.into());

        opcodes.push(Op::Div.into());

        opcodes.push(Op::Number.into());
        opcodes.append(&mut number_to_bytes(1.0));

        opcodes.push(Op::Plus.into());

        let res = vm.interpret(&opcodes);
        assert!(res.is_ok());
        assert_float_eq!(res.unwrap(), -0.6363f64, 1e-4f64);
    }

    #[test]
    fn test_function_sin() {
        let mut vm = VirtualMachine::default();

        let n = 10f64;
        let mut opcodes = vec![Op::Number.into()];
        opcodes.append(&mut number_to_bytes(n));
        opcodes.push(Op::Func.into());
        opcodes.push(FuncType::Sin.into());

        let res = vm.interpret(&opcodes);
        assert!(res.is_ok());
        assert_float_eq!(res.unwrap(), n.sin());
    }

    #[test]
    fn test_function_cos() {
        let mut vm = VirtualMachine::default();

        let n = 10f64;
        let mut opcodes = vec![Op::Number.into()];
        opcodes.append(&mut number_to_bytes(n));
        opcodes.push(Op::Func.into());
        opcodes.push(FuncType::Cos.into());

        let res = vm.interpret(&opcodes);
        assert!(res.is_ok());
        assert_float_eq!(res.unwrap(), n.cos());
    }

    #[test]
    fn test_function_log() {
        let mut vm = VirtualMachine::default();

        let n = 10f64;
        let mut opcodes = vec![Op::Number.into()];
        opcodes.append(&mut number_to_bytes(n));
        opcodes.push(Op::Func.into());
        opcodes.push(FuncType::Log.into());

        let res = vm.interpret(&opcodes);
        assert!(res.is_ok());
        assert_float_eq!(res.unwrap(), n.ln());
    }

    #[test]
    fn test_function_log_invalid() {
        let mut vm = VirtualMachine::default();

        let n = -10f64;
        let mut opcodes = vec![Op::Number.into()];
        opcodes.append(&mut number_to_bytes(n));
        opcodes.push(Op::Func.into());
        opcodes.push(FuncType::Log.into());

        let res = vm.interpret(&opcodes);
        assert!(res.is_err());
    }

    #[test]
    fn test_function_sqrt() {
        let mut vm = VirtualMachine::default();

        let n = 10f64;
        let mut opcodes = vec![Op::Number.into()];
        opcodes.append(&mut number_to_bytes(n));
        opcodes.push(Op::Func.into());
        opcodes.push(FuncType::Sqrt.into());

        let res = vm.interpret(&opcodes);
        assert!(res.is_ok());
        assert_float_eq!(res.unwrap(), n.sqrt());
    }

    #[test]
    fn test_function_sqrt_invalid() {
        let mut vm = VirtualMachine::default();

        let n = -10f64;
        let mut opcodes = vec![Op::Number.into()];
        opcodes.append(&mut number_to_bytes(n));
        opcodes.push(Op::Func.into());
        opcodes.push(FuncType::Sqrt.into());

        let res = vm.interpret(&opcodes);
        assert!(res.is_err());
    }

    #[test]
    fn test_function_pow() {
        let mut vm = VirtualMachine::default();

        let base = 2f64;
        let exponent = 3f64;
        let mut opcodes = vec![Op::Number.into()];
        opcodes.append(&mut number_to_bytes(base));
        opcodes.push(Op::Number.into());
        opcodes.append(&mut number_to_bytes(exponent));
        opcodes.push(Op::Func.into());
        opcodes.push(FuncType::Pow.into());

        let res = vm.interpret(&opcodes);
        assert!(res.is_ok());
        assert_float_eq!(res.unwrap(), base.powf(exponent));
    }

    #[test]
    fn test_i8_opcode() {
        let mut vm = VirtualMachine::default();

        let mut opcodes = vec![Op::NumberI8.into()];
        let n = -15;
        opcodes.push(unsafe { std::mem::transmute::<i8, u8>(n) });
        let res = vm.interpret(&opcodes);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), n as f64);
    }
}
