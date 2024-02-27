use std::fmt::Display;

use crate::compiler::Op;

const STACK_INITIAL_CAPACITY: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    DivisionByZero,
    EmptyStack,
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
}

impl Default for VirtualMachine {
    fn default() -> Self {
        Self {
            instruction_pointer: 0,
            stack: Vec::with_capacity(STACK_INITIAL_CAPACITY),
        }
    }
}

impl VirtualMachine {
    pub fn interpret(&mut self, opcodes: &[u8]) -> Result<f64, Error> {
        while self.instruction_pointer < opcodes.len() {
            let byte = self.advance_instruction(opcodes);
            let op = Op::try_from(byte)
                .unwrap_or_else(|e| panic!("Invalid opcode {}, error: {:?}", byte, e));
            match op {
                Op::Number => self.number(opcodes),
                Op::Negate => self.negate(),
                Op::Minus | Op::Plus | Op::Mult | Op::Div => self.binary(op)?,
            };
        }
        // reset for further calls
        self.stack.pop().ok_or(Error::EmptyStack)
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
    use crate::compiler::Op;

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
}
