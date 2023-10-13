use crate::compiler::MAX_PROGRAM_SIZE;
use crate::instruction_set::INSTRUCTION_SET;
use std::fmt::{Display, Formatter};

pub struct ProgramState {
    pub registers: [u16; 5],
    pub state_register: u16,
    pub memory: [u8; MAX_PROGRAM_SIZE],
    pub display: [u16; 16],
    pub has_finished: bool,
    pub curr_addr: usize,
}

impl ProgramState {
    pub fn new() -> Self {
        Self {
            registers: [0, 0, 0, 0, (MAX_PROGRAM_SIZE - 1) as u16],
            state_register: 0,
            memory: [0; MAX_PROGRAM_SIZE],
            display: [0; 16],
            has_finished: false,
            curr_addr: 0,
        }
    }

    pub fn read_u8(&self, addr: u16) -> RuntimeResult<u8> {
        let addr = addr as usize;
        Ok(*self
            .memory
            .get(addr)
            .ok_or(RuntimeError::InvalidAddress(self.curr_addr, addr))?)
    }

    pub fn write_u8(&mut self, addr: u16, new_val: u8) -> RuntimeResult<()> {
        let addr = addr as usize;
        *self
            .memory
            .get_mut(addr)
            .ok_or(RuntimeError::InvalidAddress(self.curr_addr, addr))? = new_val;
        Ok(())
    }

    pub fn read_u16(&self, addr: u16) -> RuntimeResult<u16> {
        Ok(u16::from_be_bytes([
            self.read_u8(addr)?,
            self.read_u8(addr + 1)?,
        ]))
    }

    pub fn write_u16(&mut self, addr: u16, new_val: u16) -> RuntimeResult<()> {
        self.write_u8(addr, (new_val >> 8) as u8)?;
        self.write_u8(addr, new_val as u8)?;
        Ok(())
    }

    pub fn execute_next_instruction(&mut self) -> RuntimeResult<()> {
        if self.has_finished {
            return Ok(());
        }
        let instruction_code = self.read_u8(self.curr_addr as u16)?;
        let executor = &INSTRUCTION_SET[instruction_code as usize].executor;
        executor(self)
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    InvalidOperand(usize, u8),
    InvalidAddress(usize, usize),
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::InvalidOperand(line, operand) => {
                write!(f, "{line:x}: invalid operand: `{operand:x}`")
            }
            RuntimeError::InvalidAddress(line, address) => {
                write!(f, "{line:x}: invalid address: `{address:x}`")
            }
        }
    }
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;
