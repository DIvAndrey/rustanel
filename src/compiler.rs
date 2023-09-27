use eframe::epaint::ahash::HashMap;

pub(crate) struct Compiler {
    instruction_codes: HashMap<&'static str, u16>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            instruction_codes: HashMap::from_iter([
                ("nop", 0x0000),
                ("mov", 0x0100),
                ("mov", 0x0100),
                ("mov", 0x0100),
            ]),
        }
    }

    pub fn compile(code: &str) {
        for line in code.split('\n') {
            let line = line.trim();
            if let [instruction, ..] = line.split(' ') {

            }
        }
    }
}
