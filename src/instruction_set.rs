use crate::executor::{ProgramExecutor, RuntimeResult};

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

impl ToString for InstructionOperand {
    fn to_string(&self) -> String {
        match self {
            InstructionOperand::Reg(4) => format!("SP"),
            InstructionOperand::Reg(r) => format!("R{r}"),
            InstructionOperand::Addr(r) => format!("(R{r})"),
            InstructionOperand::AddrInc(r) => format!("R({r})+"),
            InstructionOperand::Port(p) => format!("P{p}"),
            InstructionOperand::Number(_) => format!("number"),
        }
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
pub type InstructionExecutor = fn(&mut ProgramExecutor, AcceptedOperandTypes) -> RuntimeResult<()>;

pub struct InstructionInfo {
    pub name: &'static str,
    pub accepted_operands: AcceptedOperandTypes,
    pub executor: InstructionExecutor,
}

pub const INSTRUCTION_SET: [InstructionInfo; 4] = [
    InstructionInfo {
        name: "nop",
        accepted_operands: AcceptedOperandTypes(0, 0),
        executor: |executor, accepted_operands| {
            let size = executor.get_instruction_operands(accepted_operands)?.zero();
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
        executor: |executor, accepted_operands| {
            let (op1, op2, size) = executor.get_instruction_operands(accepted_operands)?.two();
            let num = executor.read_from(op2)?;
            executor.write_to(op1, num)?;
            executor.add_to_pc(size);
            Ok(())
        },
    },
    InstructionInfo {
        name: "add",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
        ),
        executor: |executor, accepted_operands| {
            let (op1, op2, size) = executor.get_instruction_operands(accepted_operands)?.two();
            let num1 = executor.read_from(op1)?;
            let num2 = executor.read_from(op2)?;
            let res = num1.wrapping_add(num2);
            executor.write_to(op1, res)?;
            executor.add_to_pc(size);
            Ok(())
        },
    },
    InstructionInfo {
        name: "stop",
        accepted_operands: AcceptedOperandTypes(0, 0),
        executor: |executor, accepted_operands| {
            executor.get_instruction_operands(accepted_operands)?.zero();
            executor.has_finished = true;
            Ok(())
        },
    },
];
