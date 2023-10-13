use crate::executor::{ProgramState, RuntimeError, RuntimeResult};

pub const STACK_POINTER_OPERAND_CODE: u8 = 0xC;
pub const NUMBER_OPERAND_CODE: u8 = 0xD;

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

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
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
}

// Executes a binary instruction
pub type InstructionExecutor = fn(&mut ProgramState) -> RuntimeResult<()>;

pub struct InstructionInfo {
    pub name: &'static str,
    pub accepted_operands: AcceptedOperandTypes,
    pub executor: InstructionExecutor,
}

pub const INSTRUCTION_SET: [InstructionInfo; 3] = [
    InstructionInfo {
        name: "nop",
        accepted_operands: AcceptedOperandTypes(0, 0),
        executor: |_| Ok(()),
    },
    InstructionInfo {
        name: "mov",
        accepted_operands: AcceptedOperandTypes(
            REG_MASK | ADDR_MASK | ADDR_INC_MASK,
            REG_MASK | ADDR_MASK | ADDR_INC_MASK | NUMBER_MASK,
        ),
        executor: |state| {
            let i = state.curr_addr;
            let argument1 = (state.memory[i] & 0xF0) >> 4;
            let argument2 = state.memory[i] & 0x0F;
            let num;
            // Getting the moved value
            if argument2 == NUMBER_OPERAND_CODE {
                num = u16::from_be_bytes([state.memory[i + 1], state.memory[i + 2]]);
            } else {
                if argument2 == STACK_POINTER_OPERAND_CODE {
                    num = state.registers[4];
                } else if argument2 >= 12 {
                    return Err(RuntimeError::InvalidOperand(i, argument2));
                } else if argument2 >= 8 {
                    num = state.read_u16(state.registers[argument2 as usize - 8])?;
                    state.registers[argument2 as usize - 8] += 1;
                } else if argument2 >= 4 {
                    num = state.read_u16(state.registers[argument2 as usize - 4])?;
                } else {
                    num = state.registers[argument2 as usize];
                };
            };
            // Assigning value
            if argument2 == STACK_POINTER_OPERAND_CODE {
                state.registers[4] = num;
            } else if argument1 >= 12 {
                return Err(RuntimeError::InvalidOperand(i, argument1));
            } else if argument2 >= 8 {
                state.write_u16(state.registers[argument1 as usize - 8], num)?;
                state.registers[argument1 as usize - 8] += 1;
            } else if argument1 >= 4 {
                state.write_u16(state.registers[argument1 as usize - 4], num)?;
            } else {
                state.registers[argument1 as usize] = num
            };
            Ok(())
        },
    },
    InstructionInfo {
        name: "stop",
        accepted_operands: AcceptedOperandTypes(0, 0),
        executor: |state| {
            state.has_finished = true;
            Ok(())
        },
    },
];
