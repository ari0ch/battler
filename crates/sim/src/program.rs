//! Unit programs: a short stack of rule cards ("nodes").
//!
//! Each tick a unit checks its cards top to bottom and runs the first one whose
//! condition holds *and* whose action can actually be carried out (fall-through).
//! If no card fires, a locked final rule runs: attack the nearest enemy,
//! advancing toward it when out of range. It can't be removed, so a unit never
//! just stands idle.

use std::fmt;

use crate::{Ability, Class, Level};

/// A percentage threshold picked from a dropdown in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Threshold {
    P25,
    P50,
    P75,
}

impl Threshold {
    pub fn percent(self) -> u32 {
        match self {
            Threshold::P25 => 25,
            Threshold::P50 => 50,
            Threshold::P75 => 75,
        }
    }
}

/// A team-wide signal colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signal {
    Red,
    Blue,
}

impl Signal {
    /// Bit used when storing signals in a `u8` set.
    pub(crate) fn bit(self) -> u8 {
        match self {
            Signal::Red => 0b01,
            Signal::Blue => 0b10,
        }
    }
}

/// The condition half of a rule card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Condition {
    /// Always true; the usual condition for a last, catch-all card.
    Always,
    EnemyAdjacent,
    /// An enemy is within this unit's basic attack range.
    EnemyInRange,
    MyHpBelow(Threshold),
    /// Any living ally (not counting this unit) is below the threshold.
    AllyHpBelow(Threshold),
    /// More enemies than allies (this unit included) within 2 cells.
    Outnumbered,
    AbilityReady(Ability),
    /// True on ticks divisible by N (N from 1 to 9).
    EveryNTicks(u8),
    /// A teammate sent this signal on the previous tick.
    SignalReceived(Signal),
}

/// How an action picks its target from the eligible units.
///
/// Ties are always broken by distance, then by unit id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    Nearest,
    /// Enemies: the least current HP. Allies: the most hurt by percentage.
    LowestHp,
    HighestHp,
    /// The highest attack.
    Strongest,
    /// The nearest unit with range above 1. Fails if there is none.
    Ranged,
}

/// The action half of a rule card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Shared actions.
    /// Step toward the target, unless it is already within attack range.
    Advance(Target),
    /// Basic attack on the target, if it is within range.
    Attack(Target),
    /// Take half damage until this unit's next turn.
    Guard,
    /// Step away from the nearest enemy.
    Retreat,
    /// Deliberately do nothing (still ends this tick's card evaluation).
    Hold,
    /// Move next to the targeted ally.
    ProtectAlly(Target),
    /// Broadcast a signal to teammates for the next tick.
    SendSignal(Signal),

    // Knight.
    /// Enemies within 2 cells must target this unit for 3 ticks.
    Taunt,
    /// This unit and adjacent allies take half damage for 2 ticks.
    ShieldWall,
    // Ranger.
    /// Double-damage shot with +1 range.
    AimedShot(Target),
    /// Basic attack on the target, then step away from the nearest enemy.
    Kite(Target),
    // Mage.
    /// Hits the target, and enemies adjacent to it for half damage.
    Fireball(Target),
    /// Teleport up to 3 cells to the spot furthest from enemies.
    Blink,
    // Cleric.
    /// Restore HP to an ally (or self) within range + 1. Fails on full HP.
    Heal(Target),
    /// Make an ally within range + 1 immune to damage for 2 ticks.
    Sanctuary(Target),
    // Rogue.
    /// Triple damage on an adjacent enemy that is also adjacent to one of our allies.
    Backstab(Target),
    /// Teleport next to an enemy within 4 cells, on its far side.
    Shadowstep(Target),
    // Berserker.
    /// For 4 ticks, basic attacks strike twice.
    Frenzy,
    /// Hit every adjacent enemy.
    Cleave,
}

impl Action {
    /// The signature ability this action uses, if any.
    pub fn ability(self) -> Option<Ability> {
        match self {
            Action::Taunt => Some(Ability::Taunt),
            Action::ShieldWall => Some(Ability::ShieldWall),
            Action::AimedShot(_) => Some(Ability::AimedShot),
            Action::Kite(_) => Some(Ability::Kite),
            Action::Fireball(_) => Some(Ability::Fireball),
            Action::Blink => Some(Ability::Blink),
            Action::Heal(_) => Some(Ability::Heal),
            Action::Sanctuary(_) => Some(Ability::Sanctuary),
            Action::Backstab(_) => Some(Ability::Backstab),
            Action::Shadowstep(_) => Some(Ability::Shadowstep),
            Action::Frenzy => Some(Ability::Frenzy),
            Action::Cleave => Some(Ability::Cleave),
            Action::Advance(_)
            | Action::Attack(_)
            | Action::Guard
            | Action::Retreat
            | Action::Hold
            | Action::ProtectAlly(_)
            | Action::SendSignal(_) => None,
        }
    }

    /// The target dropdown, for actions that have one.
    pub fn target(self) -> Option<Target> {
        match self {
            Action::Advance(t)
            | Action::Attack(t)
            | Action::ProtectAlly(t)
            | Action::AimedShot(t)
            | Action::Kite(t)
            | Action::Fireball(t)
            | Action::Heal(t)
            | Action::Sanctuary(t)
            | Action::Backstab(t)
            | Action::Shadowstep(t) => Some(t),
            _ => None,
        }
    }

    /// True if the target dropdown picks among allies rather than enemies.
    pub fn targets_allies(self) -> bool {
        matches!(self, Action::ProtectAlly(_) | Action::Heal(_) | Action::Sanctuary(_))
    }
}

/// One rule card: when `condition` holds, try `action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Node {
    pub condition: Condition,
    pub action: Action,
}

impl Node {
    pub const fn new(condition: Condition, action: Action) -> Self {
        Self { condition, action }
    }
}

/// The card that fired on a unit's turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardRef {
    /// A card from the unit's program, by position (0 = top).
    Node(usize),
    /// The locked final rule: attack the nearest enemy, advancing if needed.
    Default,
}

/// A unit's program: its rule cards, top to bottom.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Program {
    nodes: Vec<Node>,
}

impl Program {
    pub fn new(nodes: impl Into<Vec<Node>>) -> Self {
        Self { nodes: nodes.into() }
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Checks that a unit of this class and level may run this program.
    pub fn validate(&self, class: Class, level: Level) -> Result<(), ProgramError> {
        if self.nodes.len() > level.max_nodes() {
            return Err(ProgramError::TooManyNodes {
                max: level.max_nodes(),
                got: self.nodes.len(),
            });
        }
        for (index, node) in self.nodes.iter().enumerate() {
            if let Condition::AbilityReady(ability) = node.condition
                && !class.has_ability(ability)
            {
                return Err(ProgramError::AbilityNotAvailable { index, ability });
            }
            if let Condition::EveryNTicks(n) = node.condition
                && !(1..=9).contains(&n)
            {
                return Err(ProgramError::BadTickInterval { index, n });
            }
            if let Some(ability) = node.action.ability()
                && !class.has_ability(ability)
            {
                return Err(ProgramError::AbilityNotAvailable { index, ability });
            }
            // Focusing the weakest enemy is the Ranger's speciality.
            let focuses_weakest_enemy =
                node.action.target() == Some(Target::LowestHp) && !node.action.targets_allies();
            if focuses_weakest_enemy && class != Class::Ranger {
                return Err(ProgramError::TargetNotAvailable { index, target: Target::LowestHp });
            }
        }
        Ok(())
    }
}

/// Why a program can't be used by a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramError {
    TooManyNodes { max: usize, got: usize },
    AbilityNotAvailable { index: usize, ability: Ability },
    TargetNotAvailable { index: usize, target: Target },
    BadTickInterval { index: usize, n: u8 },
}

// Implementing `Display` + `Error` lets callers use `?` and print friendly messages.
impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProgramError::TooManyNodes { max, got } => {
                write!(f, "program has {got} cards but this unit's level allows {max}")
            }
            ProgramError::AbilityNotAvailable { index, ability } => {
                write!(f, "card {}: this class can't use {ability:?}", index + 1)
            }
            ProgramError::TargetNotAvailable { index, target } => {
                write!(f, "card {}: this class can't target {target:?} enemies", index + 1)
            }
            ProgramError::BadTickInterval { index, n } => {
                write!(f, "card {}: tick interval must be 1-9, got {n}", index + 1)
            }
        }
    }
}

impl std::error::Error for ProgramError {}
