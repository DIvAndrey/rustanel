use crate::compiler::MAX_PROGRAM_SIZE;
use crate::instruction_set::{
    AcceptedOperandTypes, InstructionInfo, InstructionOperand, InstructionOperands, ADDR_INC_MASK,
    ADDR_MASK, INSTRUCTION_SET, NUMBER_MASK, NUMBER_OPERAND_CODE, PORT_MASK, REG_MASK,
};
use std::fmt::{Display, Formatter};

pub struct ProgramExecutor {
    pub registers: [u16; 5],
    pub program_state_reg: u16,
    pub memory: [u8; MAX_PROGRAM_SIZE],
    pub display: [u16; 16],
    pub has_finished: bool,
    pub is_in_debug_mode: bool,
    // pub
    pub curr_addr: usize,
}

impl Default for ProgramExecutor {
    fn default() -> Self {
        Self {
            registers: [0, 0, 0, 0, (MAX_PROGRAM_SIZE - 1) as u16],
            program_state_reg: 0,
            memory: [0; MAX_PROGRAM_SIZE],
            display: [0; 16],
            has_finished: true,
            is_in_debug_mode: false,
            curr_addr: 0,
        }
    }
}

impl ProgramExecutor {
    pub fn set_overflow(&mut self, overflow: bool) {
        self.program_state_reg = (1 << 3) * (overflow as u16);
    }

    pub fn prepare_for_a_new_run(&mut self) {
        self.curr_addr = 0;
        self.registers[4] = (MAX_PROGRAM_SIZE - 1) as u16;
        self.has_finished = false;
    }

    pub fn read_u8(&self, addr: u16) -> RuntimeResult<u8> {
        let addr = addr as usize;
        Ok(*self.memory.get(addr).ok_or(RuntimeError::InvalidAddress {
            line: self.curr_addr,
            address: addr,
        })?)
    }

    pub fn write_u8(&mut self, addr: u16, new_val: u8) -> RuntimeResult<()> {
        let addr = addr as usize;
        *self
            .memory
            .get_mut(addr)
            .ok_or(RuntimeError::InvalidAddress {
                line: self.curr_addr,
                address: addr,
            })? = new_val;
        Ok(())
    }

    pub fn read_u16(&self, addr: u16) -> RuntimeResult<u16> {
        Ok(u16::from_be_bytes([
            self.read_u8(addr)?,
            self.read_u8(addr.wrapping_add(1))?,
        ]))
    }

    pub fn write_u16(&mut self, addr: u16, new_val: u16) -> RuntimeResult<()> {
        self.write_u8(addr, (new_val >> 8) as u8)?;
        self.write_u8(addr.wrapping_add(1), new_val as u8)?;
        Ok(())
    }

    fn get_instruction_operand(
        &self,
        accepted_operand_types: u8,
        operand: u8,
    ) -> RuntimeResult<InstructionOperand> {
        Ok(if (accepted_operand_types & PORT_MASK) != 0 {
            InstructionOperand::Port(operand)
        } else if (accepted_operand_types & NUMBER_MASK) != 0 && operand == NUMBER_OPERAND_CODE {
            InstructionOperand::Number(u16::from_be_bytes([
                self.memory[self.curr_addr + 2],
                self.memory[self.curr_addr + 3],
            ]))
        } else if (accepted_operand_types & ADDR_INC_MASK) != 0 && (10..15).contains(&operand) {
            InstructionOperand::AddrInc(operand - 10)
        } else if (accepted_operand_types & ADDR_MASK) != 0 && (5..10).contains(&operand) {
            InstructionOperand::Addr(operand - 5)
        } else if (accepted_operand_types & REG_MASK) != 0 && (0..5).contains(&operand) {
            InstructionOperand::Reg(operand)
        } else {
            return Err(RuntimeError::InvalidOperand {
                line: self.curr_addr,
                operand,
            });
        })
    }

    pub fn get_instruction_operands(
        &self,
        accepted_operand_types: AcceptedOperandTypes,
    ) -> RuntimeResult<InstructionOperands> {
        assert!(!(accepted_operand_types.0 == 0 && accepted_operand_types.1 != 0));
        let operands_byte = self.memory[self.curr_addr + 1];
        let operand1 = (operands_byte >> 4) & 0xF;
        let operand2 = operands_byte & 0xF;
        Ok(if accepted_operand_types.0 == 0 {
            InstructionOperands::Zero
        } else if accepted_operand_types.1 == 0 {
            InstructionOperands::One(
                self.get_instruction_operand(accepted_operand_types.0, operand1)?,
            )
        } else {
            InstructionOperands::Two(
                self.get_instruction_operand(accepted_operand_types.0, operand1)?,
                self.get_instruction_operand(accepted_operand_types.1, operand2)?,
            )
        })
    }

    pub fn execute_next_instruction(&mut self) -> RuntimeResult<()> {
        if self.has_finished {
            return Ok(());
        }
        let instruction_code = self.read_u8(self.curr_addr as u16)?;
        let Some(InstructionInfo {
            accepted_operands,
            executor,
            ..
        }) = &INSTRUCTION_SET.get(instruction_code as usize)
        else {
            return Err(RuntimeError::InvalidInstruction {
                line: self.curr_addr,
                instruction: instruction_code,
            });
        };
        executor(self, self.get_instruction_operands(*accepted_operands)?)
    }

    pub fn read_from(&mut self, place_to_read_from: InstructionOperand) -> RuntimeResult<u16> {
        Ok(match place_to_read_from {
            InstructionOperand::Reg(reg) => self.registers[reg as usize],
            InstructionOperand::Addr(reg) => self.read_u16(self.registers[reg as usize])?,
            InstructionOperand::AddrInc(reg) => {
                let num = self.read_u16(self.registers[reg as usize])?;
                self.registers[reg as usize] = self.registers[reg as usize].wrapping_add(2);
                num
            }
            InstructionOperand::Number(num) => num,
            InstructionOperand::Port(port) => self.display[port as usize],
        })
    }

    pub fn write_to(
        &mut self,
        place_to_write_to: InstructionOperand,
        num: u16,
    ) -> RuntimeResult<()> {
        match place_to_write_to {
            InstructionOperand::Reg(reg) => self.registers[reg as usize] = num,
            InstructionOperand::Addr(reg) | InstructionOperand::AddrInc(reg) => {
                self.write_u16(self.registers[reg as usize], num)?
            }
            InstructionOperand::Port(port) => self.display[port as usize] = num,
            InstructionOperand::Number(_) => panic!("Cannot write to a number"),
        }
        Ok(())
    }

    pub fn add_to_pc(&mut self, n: usize) {
        self.curr_addr = self.curr_addr.wrapping_add(n);
        if self.curr_addr >= MAX_PROGRAM_SIZE {
            self.curr_addr -= MAX_PROGRAM_SIZE;
        }
    }
}

#[derive(Debug, Clone)]
pub enum RuntimeError {
    InvalidInstruction { line: usize, instruction: u8 },
    InvalidOperand { line: usize, operand: u8 },
    InvalidAddress { line: usize, address: usize },
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::InvalidInstruction { line, instruction } => {
                write!(f, "line {}: Invalid instruction: `{instruction:x}`", line + 1)
            }
            RuntimeError::InvalidOperand { line, operand } => {
                write!(f, "line {}: Invalid operand: `{operand:x}`", line + 1)
            }
            RuntimeError::InvalidAddress { line, address } => {
                write!(f, "line {}: Invalid address: `{address:x}`", line + 1)
            }
        }
    }
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;
