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
        executor: |executor, accepted_operand_types| {
            match executor.get_current_instruction_operand_types(accepted_operand_types)? {
                InstructionOperands::Zero => {}
                _ => unreachable!(),
            }
            executor.add_to_pc(2);
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
            let InstructionOperands::Two(op1, op2) =
                executor.get_current_instruction_operand_types(accepted_operands)?
            else {
                unreachable!();
            };
            // Getting the moved value
            let num = match op2 {
                InstructionOperand::Reg(reg) => executor.registers[reg as usize],
                InstructionOperand::Addr(reg) => {
                    executor.read_u16(executor.registers[reg as usize])?
                }
                InstructionOperand::AddrInc(reg) => {
                    let num = executor.read_u16(executor.registers[reg as usize])?;
                    executor.registers[reg as usize] =
                        executor.registers[reg as usize].wrapping_add(2);
                    num
                }
                InstructionOperand::Number(num) => {
                    executor.add_to_pc(2);
                    num
                }
                _ => unreachable!(),
            };
            match op1 {
                InstructionOperand::Reg(reg) => executor.registers[reg as usize] = num,
                InstructionOperand::Addr(reg) => {
                    executor.write_u16(executor.registers[reg as usize], num)?
                }
                InstructionOperand::AddrInc(reg) => {
                    executor.write_u16(executor.registers[reg as usize], num)?;
                    executor.registers[reg as usize] =
                        executor.registers[reg as usize].wrapping_add(2);
                }
                _ => unreachable!(),
            }
            executor.add_to_pc(2);
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
            let InstructionOperands::Two(op1, op2) =
                executor.get_current_instruction_operand_types(accepted_operands)?
            else {
                unreachable!();
            };
            // Getting the moved value
            let num2 = match op2 {
                InstructionOperand::Reg(reg) => executor.registers[reg as usize],
                InstructionOperand::Addr(reg) => {
                    executor.read_u16(executor.registers[reg as usize])?
                }
                InstructionOperand::AddrInc(reg) => {
                    let num = executor.read_u16(executor.registers[reg as usize])?;
                    executor.registers[reg as usize] =
                        executor.registers[reg as usize].wrapping_add(2);
                    num
                }
                InstructionOperand::Number(num) => {
                    executor.add_to_pc(2);
                    num
                }
                _ => unreachable!(),
            };
            // Getting the moved value
            let num1 = match op1 {
                InstructionOperand::Reg(reg) => executor.registers[reg as usize],
                InstructionOperand::Addr(reg) | InstructionOperand::AddrInc(reg) => {
                    executor.read_u16(executor.registers[reg as usize])?
                }
                _ => unreachable!(),
            };
            let res = num1.wrapping_add(num2);
            match op1 {
                InstructionOperand::Reg(reg) => executor.registers[reg as usize] = res,
                InstructionOperand::Addr(reg) => {
                    executor.write_u16(executor.registers[reg as usize], res)?
                }
                InstructionOperand::AddrInc(reg) => {
                    executor.write_u16(executor.registers[reg as usize], res)?;
                    executor.registers[reg as usize] =
                        executor.registers[reg as usize].wrapping_add(2);
                }
                _ => unreachable!(),
            }
            executor.add_to_pc(2);
            Ok(())
        },
    },
    InstructionInfo {
        name: "stop",
        accepted_operands: AcceptedOperandTypes(0, 0),
        executor: |executor, accepted_operand_types| {
            match executor.get_current_instruction_operand_types(accepted_operand_types)? {
                InstructionOperands::Zero => {}
                _ => unreachable!(),
            }
            executor.has_finished = true;
            Ok(())
        },
    },
];
