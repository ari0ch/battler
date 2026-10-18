/// One line of a unit's program.
///
/// This is a deliberately tiny placeholder instruction set, enough to prove
/// the interpreter loop. The real language (conditions, signals, a register)
/// is still being designed; see `docs/DESIGN.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    /// Do nothing this tick.
    Wait,
    /// Step one cell toward the nearest enemy, unless already adjacent.
    Advance,
    /// Strike an adjacent enemy (the nearest, lowest id first). Wasted if none is adjacent.
    Attack,
    /// Halve incoming damage until this unit's next turn.
    Guard,
}

/// A unit's program: a short list of instructions that loops forever.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    lines: Vec<Instruction>,
}

impl Program {
    pub fn new(lines: impl Into<Vec<Instruction>>) -> Self {
        Self { lines: lines.into() }
    }

    pub fn lines(&self) -> &[Instruction] {
        &self.lines
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// The instruction at `pc`, wrapping around. An empty program waits.
    pub fn fetch(&self, pc: usize) -> Instruction {
        if self.lines.is_empty() { Instruction::Wait } else { self.lines[pc % self.lines.len()] }
    }
}
