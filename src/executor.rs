use std::fmt::{Display, Formatter};
use crate::compiler::MAX_PROGRAM_SIZE;

pub struct ProgramState {
    pub registers: [u16; 4],
    pub memory: [u8; MAX_PROGRAM_SIZE],
    pub display: [u16; 8],
    pub is_running: bool,
}

#[derive(Debug)]
pub enum RuntimeError {
    InvalidOperand(usize, u8),
    InvalidAddress(usize, usize),
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::InvalidOperand(line, operand) => write!(f, "{line:x}: invalid operand: `{operand:x}`"),
            RuntimeError::InvalidAddress(line, address) => write!(f, "{line:x}: invalid address: `{address:x}`")
        }
    }
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;
