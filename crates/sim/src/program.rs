//! Unit programs: a short stack of rule cards ("nodes").
//!
//! Each tick a unit checks its cards top to bottom and runs the first one whose
//! condition holds *and* whose action can actually be carried out (fall-through).
//! If no card fires, a locked final rule runs: attack the nearest enemy,
//! advancing toward it when out of range. It can't be removed, so a unit never
//! just stands idle.
//!
//! Every block has a [`Requirement`]: available from the start, unlocked at a
//! player level, unlocked by a class synergy, or a character's signature.

use std::fmt;

use crate::synergy::Synergies;
use crate::{Ability, Character, Class, Level};

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

/// HP threshold below which an enemy counts as "weak".
pub const WEAK_PERCENT: u32 = 30;

/// The condition half of a rule card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Condition {
    /// Always true. Shown in the editor as an empty condition slot.
    Always,
    EnemyAdjacent,
    /// An enemy is within this unit's basic attack range.
    EnemyInRange,
    MyHpBelow(Threshold),
    /// Any living ally (not counting this unit) is below the threshold.
    AllyHpBelow(Threshold),
    /// More enemies than allies (this unit included) within 2 cells.
    Outnumbered,
    /// An enemy within range is below [`WEAK_PERCENT`] HP.
    EnemyWeak,
    /// This unit took damage during the previous tick or this one.
    WasHit,
    /// Knights synergy: an adjacent ally took damage during the previous tick or this one.
    AllyAttacked,
    /// Mages synergy: an enemy in range has another enemy next to it.
    EnemiesGrouped,
    /// Rogues synergy: some enemy has none of its allies within 2 cells.
    EnemyIsolated,
    /// Berserkers synergy: two or more enemies are adjacent.
    Surrounded,
    /// Clerics synergy: a teammate sent this signal on the previous tick.
    SignalReceived(Signal),
}

/// How an action picks its target. Ties break by distance, then unit id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    Nearest,
    /// Enemies: the least current HP. Allies: the most hurt by percentage.
    Weakest,
    /// The highest attack.
    Strongest,
    /// The nearest unit with range above 1. Fails if there is none.
    Ranged,
    /// Rangers synergy: the enemy a friendly Ranger hit most recently.
    Marked,
}

/// The action half of a rule card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Shared actions.
    /// Basic attack on the target, if it is within range.
    Attack(Target),
    /// Take half damage until this unit's next turn.
    Guard,
    /// Step away from the nearest enemy.
    Retreat,
    /// Move next to the targeted ally.
    ProtectAlly(Target),
    /// Broadcast a signal to teammates for the next tick (Clerics synergy).
    SendSignal(Signal),

    // Signatures.
    /// Enemies within 2 cells must target this unit for 3 ticks.
    Taunt,
    /// This unit and adjacent allies take half damage for 2 ticks.
    ShieldWall,
    /// Heal self for 30% of max HP. Fails at full HP.
    SecondWind,
    /// Double-damage shot with +1 range.
    AimedShot(Target),
    /// Basic attack on the target, then step away from the nearest enemy.
    Kite(Target),
    /// Half damage to every enemy within range.
    Volley,
    /// A 2.5x damage bolt at a target in range.
    Firebolt(Target),
    /// Half damage to adjacent enemies, which lose their next turn.
    FrostNova,
    /// Hits the target, and enemies adjacent to it for half damage.
    Fireball(Target),
    /// Teleport up to 3 cells to the spot furthest from enemies.
    Blink,
    /// Restore 4x attack HP to an ally (or self) within range + 1. Fails on full HP.
    Heal(Target),
    /// Make an ally within range + 1 immune to damage for 2 ticks.
    Sanctuary(Target),
    /// Restore 2x attack HP to every ally. Fails if nobody is hurt.
    MassHeal,
    /// Triple damage on an adjacent enemy that is also adjacent to one of our allies.
    Backstab(Target),
    /// Teleport next to an enemy within 4 cells, on its far side.
    Shadowstep(Target),
    /// Teleport to an enemy below 25% HP within 3 cells and strike for 3x damage.
    Assassinate,
    /// For 4 ticks, basic attacks strike twice.
    Frenzy,
    /// Hit every adjacent enemy.
    Cleave,
    /// Frenzy and Cleave at once. Needs an adjacent enemy.
    Rampage,
}

/// What unlocks a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Requirement {
    /// Available from the start of a run.
    Start,
    /// Unlocked at this player level.
    PlayerLevel(u32),
    /// Unlocked while the class synergy is active (2+ of the class on the board).
    Synergy(Class, Scope),
    /// Comes with a character's signature ability.
    Signature(Ability),
}

/// Who may use a synergy block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Team,
    ClassOnly,
}

impl Condition {
    pub fn requirement(self) -> Requirement {
        use Condition::*;
        match self {
            Always | EnemyAdjacent | MyHpBelow(_) | AllyHpBelow(_) => Requirement::Start,
            EnemyInRange => Requirement::PlayerLevel(4),
            Outnumbered => Requirement::PlayerLevel(5),
            EnemyWeak | WasHit => Requirement::PlayerLevel(6),
            AllyAttacked => Requirement::Synergy(Class::Knight, Scope::ClassOnly),
            EnemiesGrouped => Requirement::Synergy(Class::Mage, Scope::ClassOnly),
            EnemyIsolated => Requirement::Synergy(Class::Rogue, Scope::ClassOnly),
            Surrounded => Requirement::Synergy(Class::Berserker, Scope::ClassOnly),
            SignalReceived(_) => Requirement::Synergy(Class::Cleric, Scope::Team),
        }
    }
}

impl Target {
    pub fn requirement(self) -> Requirement {
        match self {
            Target::Nearest | Target::Weakest => Requirement::Start,
            Target::Strongest => Requirement::PlayerLevel(4),
            Target::Ranged => Requirement::PlayerLevel(5),
            Target::Marked => Requirement::Synergy(Class::Ranger, Scope::Team),
        }
    }

    pub const ALL: [Target; 5] =
        [Target::Nearest, Target::Weakest, Target::Strongest, Target::Ranged, Target::Marked];
}

impl Action {
    pub fn requirement(self) -> Requirement {
        if let Some(ability) = self.ability() {
            return Requirement::Signature(ability);
        }
        match self {
            Action::Attack(_) | Action::Guard => Requirement::Start,
            Action::ProtectAlly(_) => Requirement::PlayerLevel(4),
            Action::Retreat => Requirement::PlayerLevel(5),
            Action::SendSignal(_) => Requirement::Synergy(Class::Cleric, Scope::Team),
            _ => Requirement::Start,
        }
    }

    /// The signature ability this action uses, if any.
    pub fn ability(self) -> Option<Ability> {
        use Action::*;
        Some(match self {
            Taunt => Ability::Taunt,
            ShieldWall => Ability::ShieldWall,
            SecondWind => Ability::SecondWind,
            AimedShot(_) => Ability::AimedShot,
            Kite(_) => Ability::Kite,
            Volley => Ability::Volley,
            Firebolt(_) => Ability::Firebolt,
            FrostNova => Ability::FrostNova,
            Fireball(_) => Ability::Fireball,
            Blink => Ability::Blink,
            Heal(_) => Ability::Heal,
            Sanctuary(_) => Ability::Sanctuary,
            MassHeal => Ability::MassHeal,
            Backstab(_) => Ability::Backstab,
            Shadowstep(_) => Ability::Shadowstep,
            Assassinate => Ability::Assassinate,
            Frenzy => Ability::Frenzy,
            Cleave => Ability::Cleave,
            Rampage => Ability::Rampage,
            Attack(_) | Guard | Retreat | ProtectAlly(_) | SendSignal(_) => return None,
        })
    }

    /// The action for a signature ability, with a sensible default target.
    pub fn for_ability(ability: Ability) -> Action {
        use Ability as A;
        match ability {
            A::Taunt => Action::Taunt,
            A::ShieldWall => Action::ShieldWall,
            A::SecondWind => Action::SecondWind,
            A::AimedShot => Action::AimedShot(Target::Weakest),
            A::Kite => Action::Kite(Target::Nearest),
            A::Volley => Action::Volley,
            A::Firebolt => Action::Firebolt(Target::Weakest),
            A::FrostNova => Action::FrostNova,
            A::Fireball => Action::Fireball(Target::Nearest),
            A::Blink => Action::Blink,
            A::Heal => Action::Heal(Target::Weakest),
            A::Sanctuary => Action::Sanctuary(Target::Weakest),
            A::MassHeal => Action::MassHeal,
            A::Backstab => Action::Backstab(Target::Nearest),
            A::Shadowstep => Action::Shadowstep(Target::Weakest),
            A::Assassinate => Action::Assassinate,
            A::Frenzy => Action::Frenzy,
            A::Cleave => Action::Cleave,
            A::Rampage => Action::Rampage,
        }
    }

    /// The target dropdown, for actions that have one.
    pub fn target(self) -> Option<Target> {
        use Action::*;
        match self {
            Attack(t) | ProtectAlly(t) | AimedShot(t) | Kite(t) | Firebolt(t) | Fireball(t)
            | Heal(t) | Sanctuary(t) | Backstab(t) | Shadowstep(t) => Some(t),
            _ => None,
        }
    }

    /// The same action with a different target (no change if it has no target).
    pub fn with_target(self, t: Target) -> Action {
        use Action::*;
        match self {
            Attack(_) => Attack(t),
            ProtectAlly(_) => ProtectAlly(t),
            AimedShot(_) => AimedShot(t),
            Kite(_) => Kite(t),
            Firebolt(_) => Firebolt(t),
            Fireball(_) => Fireball(t),
            Heal(_) => Heal(t),
            Sanctuary(_) => Sanctuary(t),
            Backstab(_) => Backstab(t),
            Shadowstep(_) => Shadowstep(t),
            other => other,
        }
    }

    /// True if the target dropdown picks among allies rather than enemies.
    pub fn targets_allies(self) -> bool {
        matches!(self, Action::ProtectAlly(_) | Action::Heal(_) | Action::Sanctuary(_))
    }
}

/// The shared conditions offered by the editor's palette, with default settings.
pub fn palette_conditions() -> Vec<Condition> {
    use Condition::*;
    vec![
        EnemyAdjacent,
        MyHpBelow(Threshold::P50),
        AllyHpBelow(Threshold::P50),
        EnemyInRange,
        Outnumbered,
        EnemyWeak,
        WasHit,
        AllyAttacked,
        EnemiesGrouped,
        EnemyIsolated,
        Surrounded,
        SignalReceived(Signal::Red),
    ]
}

/// The shared actions offered by the editor's palette, with default settings.
pub fn palette_actions() -> Vec<Action> {
    vec![
        Action::Attack(Target::Nearest),
        Action::Guard,
        Action::ProtectAlly(Target::Weakest),
        Action::Retreat,
        Action::SendSignal(Signal::Red),
    ]
}

/// What a player has unlocked: needed to check a program can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unlocks {
    pub player_level: u32,
    pub synergies: Synergies,
}

impl Unlocks {
    /// True if `character` may use a block with this requirement.
    pub fn allows(&self, req: Requirement, character: Character) -> bool {
        match req {
            Requirement::Start => true,
            Requirement::PlayerLevel(n) => self.player_level >= n,
            Requirement::Synergy(class, scope) => self.synergies.allows(class, scope, character),
            Requirement::Signature(a) => character.has_ability(a),
        }
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

    /// Every requirement this card has (condition, action and target).
    pub fn requirements(&self) -> impl Iterator<Item = Requirement> {
        let target = self.action.target().map(Target::requirement);
        [self.condition.requirement(), self.action.requirement()].into_iter().chain(target)
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

/// The most cards any unit can hold: level 3 plus a synergy bonus.
pub const MAX_CARDS: usize = 5;

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

    /// Basic checks that hold regardless of unlocks: card count within the
    /// absolute maximum and only this character's signatures.
    pub fn validate_basic(&self, character: Character) -> Result<(), ProgramError> {
        if self.nodes.len() > MAX_CARDS {
            return Err(ProgramError::TooManyCards { max: MAX_CARDS, got: self.nodes.len() });
        }
        for (index, node) in self.nodes.iter().enumerate() {
            if let Requirement::Signature(ability) = node.action.requirement()
                && !character.has_ability(ability)
            {
                return Err(ProgramError::NotThisCharacter { index, ability });
            }
        }
        Ok(())
    }

    /// Full check for the planning phase: card slots for this level and
    /// synergies, and every block unlocked.
    pub fn validate(
        &self,
        character: Character,
        level: Level,
        unlocks: &Unlocks,
    ) -> Result<(), ProgramError> {
        self.validate_basic(character)?;
        let max = unlocks.synergies.card_slots(character, level);
        if self.nodes.len() > max {
            return Err(ProgramError::TooManyCards { max, got: self.nodes.len() });
        }
        for (index, node) in self.nodes.iter().enumerate() {
            for req in node.requirements() {
                if !unlocks.allows(req, character) {
                    return Err(ProgramError::Locked { index, requirement: req });
                }
            }
        }
        Ok(())
    }
}

/// Why a program can't be used by a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramError {
    TooManyCards { max: usize, got: usize },
    NotThisCharacter { index: usize, ability: Ability },
    Locked { index: usize, requirement: Requirement },
}

impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProgramError::TooManyCards { max, got } => {
                write!(f, "{got} cards, but this unit can hold {max}")
            }
            ProgramError::NotThisCharacter { index, ability } => {
                write!(f, "card {}: {ability:?} belongs to another character", index + 1)
            }
            ProgramError::Locked { index, requirement } => match requirement {
                Requirement::PlayerLevel(n) => {
                    write!(f, "card {}: unlocks at player level {n}", index + 1)
                }
                Requirement::Synergy(class, Scope::Team) => {
                    write!(f, "card {}: needs 2 {} on the board", index + 1, class.plural())
                }
                Requirement::Synergy(class, Scope::ClassOnly) => write!(
                    f,
                    "card {}: needs 2 {} on the board, and only {} can use it",
                    index + 1,
                    class.plural(),
                    class.plural()
                ),
                _ => write!(f, "card {}: not available", index + 1),
            },
        }
    }
}

impl std::error::Error for ProgramError {}
