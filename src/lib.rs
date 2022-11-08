use std::collections::HashMap;

/// All values in any program's memory are of this type.
pub type Number = i64;

#[derive(Debug)]
enum ParameterMode {
    Position,
    Immediate,
    Relative,
}

impl ParameterMode {
    fn from(i: Number) -> ParameterMode {
        match i {
            0 => ParameterMode::Position,
            1 => ParameterMode::Immediate,
            2 => ParameterMode::Relative,
            _ => panic!("Unknown parameter mode: {}", i)
        }
    }
}

#[derive(Debug)]
enum Opcode {
    Add,
    Multiply,
    Input,
    Output,
    JumpIfTrue,
    JumpIfFalse,
    LessThan,
    Equals,
    RelativeBaseOffset,
    Halt,
}

impl Opcode {
    fn from(i: Number) -> Opcode {
        match i {
            1 => Opcode::Add,
            2 => Opcode::Multiply,
            3 => Opcode::Input,
            4 => Opcode::Output,
            5 => Opcode::JumpIfTrue,
            6 => Opcode::JumpIfFalse,
            7 => Opcode::LessThan,
            8 => Opcode::Equals,
            9 => Opcode::RelativeBaseOffset,
            99 => Opcode::Halt,
            _ => { panic!("Unknown opcode: {}", i) },
        }
    }
}

#[derive(Debug)]
struct Instruction {
    opcode: Opcode,
    param1: ParameterMode,
    param2: ParameterMode,
    param3: ParameterMode,
}

impl Instruction {
    fn from(mut i: Number) -> Instruction {
        let opcode = Opcode::from(i % 100);
        i /= 100;
        let param1 = ParameterMode::from(i % 10);
        i /= 10;
        let param2 = ParameterMode::from(i % 10);
        i /= 10;
        let param3 = ParameterMode::from(i % 10);

        Instruction {
            opcode,
            param1,
            param2,
            param3
        }
    }
}

#[derive(Debug)]
enum ProgramState {
    Running,
    WaitingForInput,
    Halted,
}

/// Contains an Intcode program.
pub struct Program {
    program: Vec<Number>,
    sp: usize,
    input: Vec<Number>,
    input_pos: usize,
    output: Vec<Number>,
    output_pos: usize,
    state: ProgramState,
    extra_memory: HashMap<usize, Number>,
    relative_base: Number,
}

impl Program {
    /// Creates a new Intcode program.
    ///
    /// The `Program` returned will start out as Running.
    pub fn new(program_vec: Vec<Number>) -> Program {
        Program {
            program: program_vec,
            sp: 0,
            input: Vec::new(),
            input_pos: 0,
            output: Vec::new(),
            output_pos: 0,
            state: ProgramState::Running,
            extra_memory: HashMap::new(),
            relative_base: 0,
        }
    }

    /// Adds a value to the program's input queue.
    pub fn push_input(&mut self, i: Number) {
        self.input.push(i);
        if let ProgramState::WaitingForInput = self.state {
            self.state = ProgramState::Running;
        }
    }

    fn push_output(&mut self, i: Number) {
        self.output.push(i);
    }

    fn get_input(&mut self) -> Number {
        let result = self.input[self.input_pos];
        self.input_pos += 1;
        result
    }

    /// Return `true` if and only if this program's output queue is not empty.
    pub fn has_output(&mut self) -> bool {
        self.output_pos < self.output.len()
    }

    /// Returns the last output generated, even if it has already been consumed.
    /// If no outputs have been generated yet, this will return `None`.
    ///
    /// # Example
    /// ```
    /// let mut program = intcode::Program::new(&vec![4, 3, 99, 1]);
    /// program.run_till_halted_or_blocked();
    /// assert_eq!(program.get_output(), Some(1));
    /// assert_eq!(program.get_output(), None);
    /// assert_eq!(program.last_output(), Some(1));
    /// ```
    pub fn last_output(&mut self) -> Option<Number> {
        if self.output.len() > 0 {
            Some(self.output[self.output.len() - 1])
        } else {
            None
        }
    }

    /// Consumes and returns the first output in the output queue that has not
    /// been consumed. If no outputs have been generated yet, this will return
    /// `None`.
    ///
    /// # Example
    /// ```
    /// let mut program = intcode::Program::new(&vec![4, 5, 4, 6, 99, 1, 2]);
    /// program.run_till_halted_or_blocked();
    /// assert_eq!(program.get_output(), Some(1));
    /// assert_eq!(program.get_output(), Some(2));
    /// assert_eq!(program.get_output(), None);
    /// ```
    pub fn get_output(&mut self) -> Option<Number> {
        if self.has_output() {
            self.output_pos += 1;
            Some(self.output[self.output_pos - 1])
        } else {
            None
        }
    }

    fn increase_sp(&mut self) {
        let instruction = Instruction::from(self.get_mem(self.sp));
        self.sp += match instruction.opcode {
            Opcode::Add => 4,
            Opcode::Multiply => 4,
            Opcode::Input => 2,
            Opcode::Output => 2,
            Opcode::JumpIfTrue => 3,
            Opcode::JumpIfFalse => 3,
            Opcode::LessThan => 4,
            Opcode::Equals => 4,
            Opcode::RelativeBaseOffset => 2,
            Opcode::Halt => 0,
        }
    }

    fn param(&self, param: usize) -> Number {
        let instruction = Instruction::from(self.get_mem(self.sp));
        let value = self.get_mem(self.sp + param);

        let mode = match param {
            1 => instruction.param1,
            2 => instruction.param2,
            3 => instruction.param3,
            _ => unreachable!()
        };

        match mode {
            ParameterMode::Position => { self.get_mem(value as usize) },
            ParameterMode::Immediate => { value },
            ParameterMode::Relative => { self.get_mem((self.relative_base + value) as usize) },
        }
    }

    /// Returns a position to write to or read from, taking into account the
    /// parameter mode. The number passed in is the parameter that needs to be
    /// converted into the appropriate position (so 3 for opcode 1, etc).
    fn get_pos(&self, param: usize) -> usize {
        let instruction = Instruction::from(self.get_mem(self.sp));
        let pos = self.get_mem(self.sp + param);

        let mode = match param {
            1 => instruction.param1,
            2 => instruction.param2,
            3 => instruction.param3,
            _ => unreachable!()
        };

        match mode {
            ParameterMode::Position => { pos as usize },
            ParameterMode::Immediate => { panic!("Can't get an immediate position!") },
            ParameterMode::Relative => { (self.relative_base + pos) as usize },
        }
    }

    fn get_mem(&self, pos: usize) -> Number {
        if pos < self.program.len() {
            self.program[pos]
        } else if self.extra_memory.contains_key(&pos) {
            *self.extra_memory.get(&pos).unwrap()
        } else {
            0
        }
    }

    fn set_mem(&mut self, pos: usize, val: Number) {
        if pos < self.program.len() {
            self.program[pos] = val;
        } else {
            self.extra_memory.insert(pos, val);
        }
    }

    fn execute_instruction(&mut self) {
        let instruction = Instruction::from(self.get_mem(self.sp));
        let mut bump_sp = true;

        if let ProgramState::Halted = self.state {
            panic!("Attempted to run a halted program.");
        }

        match instruction.opcode {
            Opcode::Add => {
                let pos = self.get_pos(3);
                self.set_mem(pos as usize, self.param(1) + self.param(2));
            }
            Opcode::Multiply => {
                let pos = self.get_pos(3);
                self.set_mem(pos as usize, self.param(1) * self.param(2));
            }
            Opcode::Input => {
                if self.input.len() > self.input_pos {
                    let input = self.get_input();
                    self.set_mem(self.get_pos(1) as usize, input);
                } else {
                    bump_sp = false;
                    self.state = ProgramState::WaitingForInput;
                }
            }
            Opcode::Output => {
                self.push_output(self.param(1));
            }
            Opcode::JumpIfTrue => {
                if self.param(1) != 0 {
                    bump_sp = false;
                    self.sp = self.param(2) as usize;
                }
            }
            Opcode::JumpIfFalse => {
                if self.param(1) == 0 {
                    bump_sp = false;
                    self.sp = self.param(2) as usize;
                }
            }
            Opcode::LessThan => {
                let pos = self.get_pos(3);
                let result = if self.param(1) < self.param(2) { 1 } else { 0 };
                self.set_mem(pos as usize, result);
            }
            Opcode::Equals => {
                let pos = self.get_pos(3);
                let result = if self.param(1) == self.param(2) { 1 } else { 0 };
                self.set_mem(pos as usize, result);
            }
            Opcode::RelativeBaseOffset => {
                self.relative_base += self.param(1);
            }
            Opcode::Halt => {
                self.state = ProgramState::Halted;
            }
        }
        if bump_sp {
            self.increase_sp();
        }
    }

    /// Returns `true` if and only if the program is in the "halted" state. This
    /// can only happen if the appropriate opcode has been executed.
    pub fn halted(&mut self) -> bool {
        match self.state {
            ProgramState::Running => false,
            ProgramState::Halted => true,
            ProgramState::WaitingForInput => false,
        }
    }

    /// Returns `true` if and only if the program is in the "halted" state, or
    /// is waiting for input.
    pub fn halted_or_blocked(&mut self) -> bool {
        match self.state {
            ProgramState::Running => false,
            ProgramState::Halted => true,
            ProgramState::WaitingForInput => true,
        }
    }

    /// Starts running the program until it can't run any further.
    ///
    /// This will go through the instructions of the program until it halts, or
    /// encounters an "input" opcode but has no input. If the latter happens,
    /// then you can call this method again after supplying input to make the
    /// program resume execution.
    ///
    /// # Panics
    ///
    /// Panics if the program is in "halted" state when the method is called, or
    /// if an unknown opcode is encountered.
    ///
    /// # Example
    /// ```
    /// let mut program = intcode::Program::new(&vec![3, 5, 4, 5, 99, 0]);
    /// program.run_till_halted_or_blocked();
    ///
    /// assert_eq!(program.get_output(), None);
    /// assert!(!program.halted());
    ///
    /// program.push_input(123);
    /// program.run_till_halted_or_blocked();
    ///
    /// assert_eq!(program.get_output(), Some(123));
    /// assert!(program.halted());
    /// ```
    pub fn run_till_halted_or_blocked(&mut self) {
        while !self.halted_or_blocked() {
            self.execute_instruction();
        }
    }
}

#[test]
fn test_get_set_mem() {
    let mut p = Program::new(&vec![1, 1, 1, 1]);
    assert_eq!(p.get_mem(0), 1);
    p.set_mem(0, 2);
    assert_eq!(p.get_mem(0), 2);
}

#[test]
fn test_get_set_extra_memory() {
    let mut p = Program::new(&vec![1, 1, 1, 1]);
    p.set_mem(100, 2);
    assert_eq!(p.get_mem(100), 2);
}

#[test]
fn test_relative_mode() {
    let v = vec![109,1,204,-1,1001,100,1,100,1008,100,16,101,1006,101,0,99];
    let mut p = Program::new(&v);
    p.run_till_halted_or_blocked();
    let mut v2 = Vec::new();
    while let Some(i) = p.get_output() {
        v2.push(i);
    }
    assert_eq!(v, v2);
}

#[test]
fn test_large_numbers() {
    let v = vec![104,1125899906842624,99];
    let mut p = Program::new(&v);
    p.run_till_halted_or_blocked();
    assert_eq!(p.get_output().unwrap(), 1125899906842624);
}

#[test]
fn test_relative_mode2() {
    let v = vec![109,19,204,-34,99];

    let mut p = Program::new(&v);
    p.relative_base = 2000;
    p.set_mem(1985, 333333);
    p.run_till_halted_or_blocked();

    assert_eq!(p.get_output().unwrap(), 333333);
    assert_eq!(p.relative_base, 2019);
}
