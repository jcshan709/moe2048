//! 纯 2048 棋盘逻辑：只关心滑动与合并，不涉及终端、异步与随机数，因此极易测试。

/// 棋盘的边长（4x4）。
pub const SIZE: usize = 4;

/// 一个格子上的数字，`0` 表示空格。
pub type Tile = u32;

/// 玩家可以滑动的四个方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    /// `line` 这一条线上、从“数字将要滑向的那一侧”数起第 `step` 个格子的坐标。
    const fn cell(self, line: usize, step: usize) -> (usize, usize) {
        let forward = match self {
            Direction::Left | Direction::Up => step,
            Direction::Right | Direction::Down => SIZE - 1 - step,
        };
        match self {
            Direction::Left | Direction::Right => (line, forward),
            Direction::Up | Direction::Down => (forward, line),
        }
    }
}

/// 4x4 的棋盘。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Board {
    cells: [[Tile; SIZE]; SIZE],
}

/// [`Board::slide`] 的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SlideOutcome {
    /// 是否有格子发生了移动或合并。
    pub moved: bool,
    /// 本次滑动因合并得到的分数。
    pub gained: u64,
}

impl Board {
    /// 直接用行优先的二维数组构造棋盘（测试与推演时很方便）。
    #[cfg(test)]
    pub const fn from_rows(cells: [[Tile; SIZE]; SIZE]) -> Self {
        Self { cells }
    }

    /// 读取 `(row, col)` 上的数字。
    pub const fn get(&self, row: usize, col: usize) -> Tile {
        self.cells[row][col]
    }

    /// 写入 `(row, col)` 上的数字。
    pub fn set(&mut self, row: usize, col: usize, value: Tile) {
        self.cells[row][col] = value;
    }

    /// 所有空格，行优先顺序。
    pub fn empty_cells(&self) -> Vec<(usize, usize)> {
        let mut empty = Vec::new();
        for row in 0..SIZE {
            for col in 0..SIZE {
                if self.cells[row][col] == 0 {
                    empty.push((row, col));
                }
            }
        }
        empty
    }

    /// 棋盘上最大的数字。
    pub fn max_tile(&self) -> Tile {
        self.cells.iter().flatten().copied().max().unwrap_or(0)
    }

    /// 是否还存在能让棋盘发生变化的滑动。
    pub fn has_moves(&self) -> bool {
        if !self.empty_cells().is_empty() {
            return true;
        }
        for row in 0..SIZE {
            for col in 0..SIZE {
                let value = self.cells[row][col];
                if col + 1 < SIZE && self.cells[row][col + 1] == value {
                    return true;
                }
                if row + 1 < SIZE && self.cells[row + 1][col] == value {
                    return true;
                }
            }
        }
        false
    }

    /// 朝 `direction` 方向滑动整个棋盘，相邻且相等的数字合并一次。
    pub fn slide(&mut self, direction: Direction) -> SlideOutcome {
        let before = self.cells;
        let mut gained = 0;

        for line in 0..SIZE {
            // 先按滑动方向把这一条线读成一维数组，压紧合并后再写回去。
            let values: [Tile; SIZE] = std::array::from_fn(|step| {
                let (row, col) = direction.cell(line, step);
                self.cells[row][col]
            });

            let (collapsed, earned) = collapse(values);
            gained += earned;

            for (step, value) in collapsed.into_iter().enumerate() {
                let (row, col) = direction.cell(line, step);
                self.cells[row][col] = value;
            }
        }

        SlideOutcome { moved: before != self.cells, gained }
    }
}

/// 把一条线上的数字压向靠前一侧并合并相邻的相同数字。
///
/// `[2, 0, 2, 4]` 变成 `[4, 4, 0, 0]`；同一次滑动中刚刚合并出来的数字不会再被
/// 合并，所以 `[2, 2, 2, 2]` 只得到 `[4, 4, 0, 0]`。
fn collapse(line: [Tile; SIZE]) -> ([Tile; SIZE], u64) {
    let mut collapsed = [0; SIZE];
    let mut gained = 0;
    let mut write = 0;
    // 等待与下一个数字配对的前一个数字。
    let mut pending: Option<Tile> = None;

    for value in line.into_iter().filter(|value| *value != 0) {
        match pending.take() {
            Some(previous) if previous == value => {
                let merged = previous.saturating_mul(2);
                collapsed[write] = merged;
                write += 1;
                gained += u64::from(merged);
            }
            Some(previous) => {
                collapsed[write] = previous;
                write += 1;
                pending = Some(value);
            }
            None => pending = Some(value),
        }
    }

    if let Some(last) = pending {
        collapsed[write] = last;
    }

    (collapsed, gained)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board_of(rows: [[Tile; SIZE]; SIZE]) -> Board {
        Board::from_rows(rows)
    }

    fn row(board: &Board, row: usize) -> [Tile; SIZE] {
        [0, 1, 2, 3].map(|col| board.get(row, col))
    }

    fn column(board: &Board, col: usize) -> [Tile; SIZE] {
        [0, 1, 2, 3].map(|row| board.get(row, col))
    }

    #[test]
    fn merges_a_single_pair() {
        let (collapsed, gained) = collapse([2, 2, 4, 4]);
        assert_eq!(collapsed, [4, 8, 0, 0]);
        assert_eq!(gained, 12);
    }

    #[test]
    fn never_merges_a_tile_twice_in_one_slide() {
        assert_eq!(collapse([2, 2, 2, 2]), ([4, 4, 0, 0], 8));
        assert_eq!(collapse([4, 4, 4, 0]), ([8, 4, 0, 0], 8));
    }

    #[test]
    fn keeps_distinct_tiles_in_order() {
        assert_eq!(collapse([2, 4, 2, 4]), ([2, 4, 2, 4], 0));
        assert_eq!(collapse([0, 0, 0, 2]), ([2, 0, 0, 0], 0));
    }

    #[test]
    fn slides_left_and_reports_the_gain() {
        let mut board = board_of([[2, 2, 4, 4], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let outcome = board.slide(Direction::Left);
        assert!(outcome.moved);
        assert_eq!(outcome.gained, 12);
        assert_eq!(row(&board, 0), [4, 8, 0, 0]);
    }

    #[test]
    fn slides_right_mirrored() {
        let mut board = board_of([[2, 2, 4, 4], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        board.slide(Direction::Right);
        assert_eq!(row(&board, 0), [0, 0, 4, 8]);
    }

    #[test]
    fn slides_up_and_down_on_columns() {
        let mut board = board_of([[2, 0, 0, 0], [2, 0, 0, 0], [4, 0, 0, 0], [4, 0, 0, 0]]);
        board.slide(Direction::Up);
        assert_eq!(column(&board, 0), [4, 8, 0, 0]);

        let mut board = board_of([[2, 0, 0, 0], [2, 0, 0, 0], [4, 0, 0, 0], [4, 0, 0, 0]]);
        board.slide(Direction::Down);
        assert_eq!(column(&board, 0), [0, 0, 4, 8]);
    }

    #[test]
    fn a_blocked_slide_changes_nothing() {
        let before = board_of([[2, 4, 2, 4], [4, 2, 4, 2], [2, 4, 2, 4], [4, 2, 4, 2]]);
        let mut board = before;
        let outcome = board.slide(Direction::Left);
        assert!(!outcome.moved);
        assert_eq!(outcome.gained, 0);
        assert_eq!(board, before);
    }

    #[test]
    fn detects_a_locked_board() {
        let locked = board_of([[2, 4, 2, 4], [4, 2, 4, 2], [2, 4, 2, 4], [4, 2, 4, 2]]);
        assert!(!locked.has_moves());
        // 只要存在一个空格，就还能继续走。
        let mut almost = locked;
        almost.set(3, 3, 0);
        assert!(almost.has_moves());
        // 空格被填满、但存在相邻相同数字时同样能继续走。
        let mut mergeable = locked;
        mergeable.set(0, 1, 2);
        assert!(mergeable.has_moves());
    }

    #[test]
    fn tracks_empty_cells_and_the_largest_tile() {
        let mut board = Board::default();
        assert_eq!(board.empty_cells().len(), SIZE * SIZE);
        assert_eq!(board.max_tile(), 0);
        board.set(1, 2, 64);
        assert_eq!(board.max_tile(), 64);
        assert!(!board.empty_cells().contains(&(1, 2)));
    }
}
