use std::fmt::Display;
use crate::compiler::MAX_PROGRAM_SIZE;
use crate::executor::{ProgramExecutor, RuntimeError, RuntimeResult};

pub const NUMBER_OPERAND_CODE: u8 = 0xF;
pub const REG_MASK: u8 = 0b00001;
pub const ADDR_MASK: u8 = 0b00010;
pub const ADDR_INC_MASK: u8 = 0b00100;
pub const PORT_MASK: u8 = 0b01000;
pub const NUMBER_MASK: u8 = 0b10000;

#[derive(Clone, Copy)]
pub struct AcceptedOperandTypes(pub u8, pub u8);

impl AcceptedOperandTypes {
    pub fn count(&self) -> usize {
        if self.0 == 0 {
            0
        } else if self.1 == 0 {
            1
        } else {
            2
        }
    }
}

pub fn get_expected_operand_types_string(mask: u8) -> String {
    let mut expected = vec![];
    const OPERAND_TYPES: [(u8, &'static str); 5] = [
        (REG_MASK, "register"),
        (ADDR_MASK, "address"),
        (ADDR_INC_MASK, "address++"),
        (PORT_MASK, "port"),
        (NUMBER_MASK, "number"),
    ];
    for (curr_mask, name) in OPERAND_TYPES {
        if mask & curr_mask != 0 {
            expected.push(name);
        }
    }
    if expected.is_empty() {
        return "nothing".into();
    }
    if expected.len() == 1 {
        return expected[0].into();
    }
    let mut res = String::new();
    for i in 0..(expected.len() - 1) {
        res += expected[i];
        if i + 2 != expected.len() {
            res += ", ";
        }
    }
    res += " or ";
    res += expected.last().unwrap();
    res
}

#[derive(Clone, Copy, Debug)]
pub enum InstructionOperand {
    Reg(u8),
    Addr(u8),
    AddrInc(u8),
    Port(u8),
    Number(u16),
}

impl Display for InstructionOperand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            InstructionOperand::Reg(4) => "SP".to_string(),
            InstructionOperand::Reg(r) => format!("R{r}"),
            InstructionOperand::Addr(r) => format!("(R{r})"),
            InstructionOperand::AddrInc(r) => format!("(R{r})+"),
            InstructionOperand::Port(p) => format!("P{p}"),
            InstructionOperand::Number(_) => "number".to_string(),
        };
        write!(f, "{}", str)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InstructionOperands {
    Zero,
    One(InstructionOperand),
    Two(InstructionOperand, InstructionOperand),
}

impl InstructionOperands {
    pub fn count(&self) -> usize {
        match self {
            InstructionOperands::Zero => 0,
            InstructionOperands::One(_) => 1,
            InstructionOperands::Two(_, _) => 2,
        }
    }

    pub fn instruction_size(&self) -> usize {
        match self {
            InstructionOperands::Zero => 2,
            InstructionOperands::One(op1) => {
                if let InstructionOperand::Number(_) = op1 {
                    4
                } else {
                    2
                }
            }
            InstructionOperands::Two(op1, op2) => {
                if let InstructionOperand::Number(_) = op1 {
                    4
                } else if let InstructionOperand::Number(_) = op2 {
                    4
                } else {
                    2
                }
            }
        }
    }

    pub fn zero(&self) -> usize {
        match self {
            InstructionOperands::Zero => 2,
            _ => panic!("Expected 0 operands, found {}", self.count()),
        }
    }

    pub fn one(&self) -> (InstructionOperand, usize) {
        match self {
            &InstructionOperands::One(op) => (
                op,
                match op {
                    InstructionOperand::Number(_) => 4,
                    _ => 2,
                },
            ),
            _ => panic!("Expected 1 operand, found {}", self.count()),
        }
    }

    pub fn two(&self) -> (InstructionOperand, InstructionOperand, usize) {
        match self {
            &InstructionOperands::Two(op1, op2) => (
                op1,
                op2,
                match (op1, op2) {
                    (InstructionOperand::Number(_), _) => 4,
                    (_, InstructionOperand::Number(_)) => 4,
                    _ => 2,
                },
            ),
            _ => panic!("Expected 2 operands, found {}", self.count()),
        }
    }
}

// Executes a binary instruction
pub type InstructionExecutor = fn(&mut ProgramExecutor, InstructionOperands) -> RuntimeResult<()>;

pub struct InstructionInfo {
    pub name: &'static str,
    pub accepted_operands: AcceptedOperandTypes,
    pub executor: InstructionExecutor,
}

macro_rules! one_operand_instruction {
    ($f:expr) => {
        |executor: &mut ProgramExecutor, operands: InstructionOperands| {
            let (op1, size) = operands.one();
            let res = ($f)(executor.read_from(op1)?);
            executor.write_to(op1, res)?;
            executor.add_to_pc(size);
            Ok(())
        }
    };
}

macro_rules! two_operands_instruction {
    ($f:expr) => {
        |executor: &mut ProgramExecutor, operands: InstructionOperands| {
            let (op1, op2, size) = operands.two();
            let res = ($f)(executor.read_from(op1)?, executor.read_from(op2)?);
            executor.write_to(op1, res)?;
            executor.add_to_pc(size);
            Ok(())
        }
    };
}

pub const INSTRUCTION_SET: [InstructionInfo; 14] = [
    InstructionInfo {
        name: "nop",
        accepted_operands: AcceptedOperandTypes(0, 0),
        executor: |executor, operands| {
            let size = operands.zero();
            executor.add_to_pc(size);
            Ok(())
        },
    },
    InstructionInfo {
        name: "mov",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
        ),
        executor: two_operands_instruction!(|_a, b| b),
    },
    InstructionInfo {
        name: "add",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
        ),
        executor: two_operands_instruction!(u16::wrapping_add),
    },
    InstructionInfo {
        name: "sub",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
        ),
        executor: two_operands_instruction!(u16::wrapping_sub),
    },
    InstructionInfo {
        name: "mul",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
        ),
        executor: two_operands_instruction!(u16::wrapping_mul),
    },
    InstructionInfo {
        name: "div",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
        ),
        executor: two_operands_instruction!(u16::wrapping_div),
    },
    InstructionInfo {
        name: "and",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
        ),
        executor: two_operands_instruction!(|a, b| a & b),
    },
    InstructionInfo {
        name: "or",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
        ),
        executor: two_operands_instruction!(|a, b| a | b),
    },
    InstructionInfo {
        name: "xor",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
        ),
        executor: two_operands_instruction!(|a, b| a ^ b),
    },
    InstructionInfo {
        name: "not",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            0,
        ),
        executor: one_operand_instruction!(|a: u16| !a),
    },
    InstructionInfo {
        name: "jmp",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
            0,
        ),
        executor: |executor, operands| {
            let (op, _) = operands.one();
            let addr = executor.read_from(op)? as usize;
            if addr >= MAX_PROGRAM_SIZE {
                return Err(RuntimeError::InvalidAddress(executor.curr_addr, addr));
            }
            executor.curr_addr = addr;
            Ok(())
        },
    },
    InstructionInfo {
        name: "wrt",
        accepted_operands: AcceptedOperandTypes(
            PORT_MASK,
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
        ),
        executor: |executor, operands| {
            let (port, data, size) = operands.two();
            let data = executor.read_from(data)?;
            executor.write_to(port, data)?;
            executor.add_to_pc(size);
            Ok(())
        },
    },
    InstructionInfo {
        name: "read",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            PORT_MASK,
        ),
        executor: |executor, operands| {
            let (place, data, size) = operands.two();
            let data = executor.read_from(data)?;
            executor.write_to(place, data)?;
            executor.add_to_pc(size);
            Ok(())
        },
    },
    InstructionInfo {
        name: "stop",
        accepted_operands: AcceptedOperandTypes(0, 0),
        executor: |executor, _operands| {
            executor.has_finished = true;
            Ok(())
        },
    },
];
