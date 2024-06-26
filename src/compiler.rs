use crate::highlighting::wrapping_parse;
use crate::instruction_set::{
    get_expected_operand_types_string, AcceptedOperandTypes, InstructionOperand,
    InstructionOperands, ADDR_INC_MASK, ADDR_MASK, INSTRUCTION_SET, NUMBER_MASK,
    NUMBER_OPERAND_CODE, PORT_MASK, REG_MASK,
};
use eframe::egui::ahash::{HashSet, HashSetExt};
use eframe::epaint::ahash::{HashMap, HashMapExt};
use lazy_regex::{regex_captures, regex_is_match};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Range;

pub const MAX_PROGRAM_SIZE: usize = 0x1000;

pub struct Compiler {
    instruction_codes: HashMap<&'static str, u8>,
    pub program: [u8; MAX_PROGRAM_SIZE],
    label_mentions_in_program: Vec<(String, (usize, usize))>,
    line_addresses: Vec<usize>,
    line_i: usize,
    pub errors: ErrorsHighlightInfo,
}

#[derive(Debug, Hash, Clone)]
pub enum CompilationError {
    UnknownInstruction {
        line: usize,
        instruction: String,
    },
    NoLabelWithSuchName {
        line: usize,
        name: String,
    },
    InvalidOperand {
        line: usize,
        operand: String,
    },
    WrongNumberOfOperands {
        line: usize,
        expected: usize,
        found: usize,
    },
    WrongOperandType {
        line: usize,
        expected: String,
        found: String,
    },
    OutOfMemory {
        line: usize,
    },
    LabelAlreadyExists {
        line: usize,
        name: String,
    },
    InvalidLabelName {
        line: usize,
        name: String,
    },
}

pub type CompilationResult<T> = Result<T, CompilationError>;

impl Display for CompilationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilationError::UnknownInstruction { line, instruction } => {
                write!(f, "line {}: Unknown instruction: `{instruction}`", line + 1)
            }
            CompilationError::NoLabelWithSuchName { line, name } => {
                write!(f, "line {}: No label with such name: `{name}`", line + 1)
            }
            CompilationError::InvalidOperand { line, operand } => {
                write!(f, "line {}: Invalid operand: `{operand}`", line + 1)
            }
            CompilationError::WrongNumberOfOperands {
                line,
                expected,
                found,
            } => write!(
                f,
                "line {}: Wrong number of operands: expected {expected}, found {found}",
                line + 1,
            ),
            CompilationError::WrongOperandType {
                line,
                expected,
                found,
            } => write!(
                f,
                "line {}: Wrong operand type: expected {expected}, found {found}",
                line + 1
            ),
            CompilationError::OutOfMemory { line } => {
                write!(f, "line {}: Program doesn't fit in memory", line + 1)
            }
            CompilationError::LabelAlreadyExists { line, name } => {
                write!(
                    f,
                    "line {}: A label with such name already exists: `{name}`",
                    line + 1
                )
            }
            CompilationError::InvalidLabelName { line, name } => {
                write!(f, "line {}: `{name}` is not a correct label name", line + 1)
            }
        }
    }
}

impl Error for CompilationError {}

pub type ErrorsHighlightInfo = Vec<(Range<usize>, CompilationError)>;

impl Compiler {
    pub fn build() -> Self {
        let mut instructions = HashMap::with_capacity(256);
        for (i, instruction_info) in INSTRUCTION_SET.iter().enumerate() {
            instructions.insert(instruction_info.name, i as u8);
        }
        Self {
            instruction_codes: instructions,
            program: [0; MAX_PROGRAM_SIZE],
            label_mentions_in_program: vec![],
            line_addresses: vec![],
            line_i: 0,
            errors: vec![],
        }
    }

    fn process_operand(
        operand: InstructionOperand,
        accepted_mask: u8,
        line_i: usize,
    ) -> CompilationResult<(u8, Option<u16>)> {
        type Operand = InstructionOperand;
        let mut operand_byte = 0;
        let mut number = None;
        match operand {
            Operand::Reg(r) if (accepted_mask & REG_MASK) != 0 => operand_byte |= r,
            Operand::Addr(r) if (accepted_mask & ADDR_MASK) != 0 => operand_byte |= r + 5,
            Operand::AddrInc(r) if (accepted_mask & ADDR_INC_MASK) != 0 => operand_byte |= r + 10,
            Operand::Port(r) if (accepted_mask & PORT_MASK) != 0 => operand_byte |= r,
            Operand::Number(n) if (accepted_mask & NUMBER_MASK) != 0 => {
                number = Some(n);
                operand_byte |= NUMBER_OPERAND_CODE;
            }
            _ => {
                return Err(CompilationError::WrongOperandType {
                    line: line_i,
                    expected: get_expected_operand_types_string(accepted_mask),
                    found: operand.to_string(),
                })
            }
        }
        Ok((operand_byte, number))
    }

    fn convert_operands_to_binary(
        &self,
        operands: InstructionOperands,
        accepted: AcceptedOperandTypes,
    ) -> CompilationResult<(u8, Option<u16>)> {
        let expected_operands_num = accepted.count();
        match operands {
            InstructionOperands::Two(a, b) => {
                if expected_operands_num != 2 {
                    return Err(CompilationError::WrongNumberOfOperands {
                        line: self.line_i,
                        expected: expected_operands_num,
                        found: 2,
                    });
                }
                let (operand1, mut number) = Self::process_operand(a, accepted.0, self.line_i)?;
                let (operand2, number1) = Self::process_operand(b, accepted.1, self.line_i)?;
                if number1.is_some() {
                    assert_eq!(number, None);
                    number = number1;
                }
                let operands = (operand1 << 4) | operand2;
                Ok((operands, number))
            }
            InstructionOperands::One(a) => {
                if expected_operands_num != 1 {
                    return Err(CompilationError::WrongNumberOfOperands {
                        line: self.line_i,
                        expected: expected_operands_num,
                        found: 1,
                    });
                }
                let (operand, number) = Self::process_operand(a, accepted.0, self.line_i)?;
                Ok((operand << 4, number))
            }
            InstructionOperands::Zero => {
                if expected_operands_num != 0 {
                    return Err(CompilationError::WrongNumberOfOperands {
                        line: self.line_i,
                        expected: expected_operands_num,
                        found: 0,
                    });
                }
                Ok((0, None))
            }
        }
    }

    fn str_reg_to_num(r: &str) -> u8 {
        match r {
            "r0" => 0,
            "r1" => 1,
            "r2" => 2,
            "r3" => 3,
            "sp" => 4,
            _ => unreachable!(),
        }
    }

    fn parse_operand(&mut self, string: &str) -> CompilationResult<InstructionOperand> {
        let string = string.trim();
        // Register
        if let Some((_, r)) = regex_captures!(r"^(r0|r1|r2|r3|sp)$", string) {
            return Ok(InstructionOperand::Reg(Self::str_reg_to_num(r)));
        }
        // Address in register
        if let Some((_, r)) = regex_captures!(r"^\((r0|r1|r2|r3|sp)\)$", string) {
            return Ok(InstructionOperand::Addr(Self::str_reg_to_num(r)));
        }
        // Address in register with increment
        if let Some((_, r)) = regex_captures!(r"^\((r0|r1|r2|r3|sp)\)\+$", string) {
            return Ok(InstructionOperand::AddrInc(Self::str_reg_to_num(r)));
        }
        // Port
        if let Some((_, r)) = regex_captures!(r"^p([0-9]|1[0-5])$", string) {
            return Ok(InstructionOperand::Port(r.parse::<u8>().unwrap()));
        }
        // Number
        if let Some(num) = wrapping_parse(string) {
            return Ok(InstructionOperand::Number(num));
        }
        // Label
        if let Some((_, label_name)) = regex_captures!(r"^@(\w+)$", string) {
            self.label_mentions_in_program.push((
                label_name.to_string(),
                (self.line_i, self.line_addresses.last().unwrap() + 2),
            ));
            return Ok(InstructionOperand::Number(0));
        }
        Err(CompilationError::InvalidOperand {
            line: self.line_i,
            operand: string.to_string(),
        })
    }

    // Compiles a single assembly instruction and returns its binary code
    fn process_instruction(&mut self, text: &str) -> CompilationResult<Option<(u16, Option<u16>)>> {
        let words = text.splitn(2, ' ').collect::<Vec<&str>>();
        if words.is_empty() {
            return Ok(None);
        }
        let name = words[0];
        if name.is_empty() {
            return Ok(None);
        }
        let code =
            *self
                .instruction_codes
                .get(name)
                .ok_or(CompilationError::UnknownInstruction {
                    line: self.line_i,
                    instruction: name.to_string(),
                })?;
        let info = &INSTRUCTION_SET[code as usize];
        let operands = if let Some(operands) = words.get(1) {
            operands.split(',').collect::<Vec<&str>>()
        } else {
            vec![]
        };
        if operands.len() != info.accepted_operands.count() {
            return Err(CompilationError::WrongNumberOfOperands {
                line: self.line_i,
                expected: info.accepted_operands.count(),
                found: operands.len(),
            });
        }
        let operands = match operands[..] {
            [] => InstructionOperands::Zero,
            [a] => InstructionOperands::One(self.parse_operand(a)?),
            [a, b] => InstructionOperands::Two(self.parse_operand(a)?, self.parse_operand(b)?),
            _ => unreachable!(),
        };
        let (operands, number) =
            self.convert_operands_to_binary(operands, info.accepted_operands)?;
        Ok(Some((((code as u16) << 8) | operands as u16, number)))
    }

    fn preprocess_line(line: &str) -> &str {
        line.trim().split(';').next().unwrap()
    }

    pub fn compile_code(&mut self, asm_code: &str) {
        self.program = [0; MAX_PROGRAM_SIZE];
        let asm_code = asm_code.to_lowercase();
        let lines: Vec<(usize, &str)> = asm_code.split('\n').enumerate().collect();
        let mut label_names = HashSet::new();
        let mut errors = vec![];

        // Saving names of created labels.
        let mut curr_symbol = 0;
        for &(i, raw_line) in &lines {
            let raw_line_len = raw_line.chars().count() + 1;
            let line = Self::preprocess_line(raw_line);
            if let Some(label_name) = line.strip_suffix(':') {
                if regex_is_match!(r"^(?:\w)+$", label_name) {
                    if label_names.contains(label_name) {
                        errors.push((
                            curr_symbol..(curr_symbol + raw_line_len),
                            CompilationError::LabelAlreadyExists {
                                line: i,
                                name: label_name.to_string(),
                            },
                        ));
                    }
                    label_names.insert(label_name);
                } else {
                    errors.push((
                        curr_symbol..(curr_symbol + raw_line_len),
                        CompilationError::InvalidLabelName {
                            line: i,
                            name: label_name.to_string(),
                        },
                    ));
                }
            }
            curr_symbol += raw_line_len;
        }
        // Compiling the program
        let mut curr_symbol = 0;
        let mut label_addresses = HashMap::new();
        let mut line_start_symbol_indexes = vec![];
        self.label_mentions_in_program.clear();
        self.line_addresses = vec![0];
        for &(i, line) in &lines {
            line_start_symbol_indexes.push(curr_symbol);
            self.line_i = i;
            let mut instruction_size = 0;
            let line_len_raw = line.len() + 1;
            let line = Self::preprocess_line(line);
            if line.ends_with(':') {
                // Label
                let label_name = &line[..(line.len() - 1)];
                if let Some(&addr) = self.line_addresses.last() {
                    label_addresses.insert(label_name, addr);
                }
            } else {
                // Instruction
                match self.process_instruction(line) {
                    Ok(binary) => {
                        let addr = *self.line_addresses.last().unwrap();
                        if let Some((instruction, number)) = binary {
                            self.program[addr] = (instruction >> 8) as u8;
                            self.program[addr + 1] = instruction as u8;
                            match number {
                                Some(number) => {
                                    if addr + 1 >= self.program.len() {
                                        errors.push((
                                            curr_symbol..(curr_symbol + line_len_raw),
                                            CompilationError::OutOfMemory { line: i },
                                        ));
                                    }
                                    self.program[addr + 2] = (number >> 8) as u8;
                                    self.program[addr + 3] = number as u8;
                                    instruction_size = 4;
                                }
                                None => instruction_size = 2,
                            }
                        }
                    }
                    Err(e) => errors.push((curr_symbol..(curr_symbol + line_len_raw), e)),
                }
            }
            self.line_addresses
                .push(self.line_addresses.last().unwrap() + instruction_size);
            curr_symbol += line_len_raw;
        }
        line_start_symbol_indexes.push(asm_code.chars().count());
        // Replacing currently uninitialized label @mentions in code with the right addresses.
        for (label, (label_line, mention_addr)) in self.label_mentions_in_program.clone() {
            if let Some(&addr) = label_addresses.get(label.as_str()) {
                assert_eq!(self.program[mention_addr], 0);
                assert_eq!(self.program[mention_addr + 1], 0);
                self.program[mention_addr] = (addr >> 8) as u8;
                self.program[mention_addr + 1] = addr as u8;
            } else {
                errors.push((
                    line_start_symbol_indexes[label_line]
                        ..line_start_symbol_indexes[label_line + 1],
                    CompilationError::NoLabelWithSuchName {
                        line: label_line,
                        name: label,
                    },
                ));
            }
        }
        self.errors = errors;
    }
}
