/// A cell on the battle grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Grid (Manhattan) distance between two cells.
    pub fn distance(self, other: Pos) -> u32 {
        self.x.abs_diff(other.x) + self.y.abs_diff(other.y)
    }

    /// True if the cells share an edge.
    pub fn is_adjacent(self, other: Pos) -> bool {
        self.distance(other) == 1
    }

    /// Candidate single steps toward `target`, best first.
    ///
    /// The axis with the larger gap is tried first; ties prefer the x axis.
    /// The order is fixed so movement stays deterministic.
    pub fn steps_toward(self, target: Pos) -> impl Iterator<Item = Pos> {
        let dx = (target.x - self.x).signum();
        let dy = (target.y - self.y).signum();
        let horizontal = (dx != 0).then(|| Pos::new(self.x + dx, self.y));
        let vertical = (dy != 0).then(|| Pos::new(self.x, self.y + dy));
        let (first, second) = if self.x.abs_diff(target.x) >= self.y.abs_diff(target.y) {
            (horizontal, vertical)
        } else {
            (vertical, horizontal)
        };
        first.into_iter().chain(second)
    }

    /// The four edge-adjacent cells, in a fixed order (up, left, right, down).
    pub fn neighbours(self) -> [Pos; 4] {
        [
            Pos::new(self.x, self.y - 1),
            Pos::new(self.x - 1, self.y),
            Pos::new(self.x + 1, self.y),
            Pos::new(self.x, self.y + 1),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_is_manhattan() {
        assert_eq!(Pos::new(0, 0).distance(Pos::new(3, -2)), 5);
    }

    #[test]
    fn steps_prefer_the_longer_axis() {
        let steps: Vec<_> = Pos::new(0, 0).steps_toward(Pos::new(1, 3)).collect();
        assert_eq!(steps, vec![Pos::new(0, 1), Pos::new(1, 0)]);
    }

    #[test]
    fn no_steps_when_already_there() {
        assert_eq!(Pos::new(2, 2).steps_toward(Pos::new(2, 2)).count(), 0);
    }
}
