//! 棋盘和规则

use std::fmt;

use serde::Serialize;

use super::direction::Direction;
use super::geometry::{Pos, SIZE};
use super::rng::Rng;
use super::tile::{Tile, TileError, TileId, SPAWN_EXPONENTS};

/// 2的出现概率（9/10）
const SPAWN_TWO_ODDS: (u32, u32) = (9, 10);

/// 第一个Tile应分配的id
const FIRST_TILE_ID: TileId = 1;

/// 移动事件
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SlideEvent {
    pub id: TileId,
    pub from: Pos,
    pub to: Pos,
}

/// 合并事件
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MergeEvent {
    /// 合并处
    pub at: Pos,
    /// 合并后的Tile id
    pub id: TileId,
    /// 合并后的显示值
    pub value: u32,
    /// 合并的两个Tile的移动事件
    pub consumed: [SlideEvent; 2],
}

/// 产生事件
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SpawnEvent {
    pub id: TileId,
    pub value: u32,
    pub at: Pos,
}

/// 一次移动的状态
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShiftReport {
    /// 是否成功移动
    pub moved: bool,
    /// 加分
    pub gained: u32,
    /// 仅发生了移动的Tile
    pub slides: Vec<SlideEvent>,
    /// 发生了合并的Tile
    pub merges: Vec<MergeEvent>,
}

/// 棋盘
///
/// 应大肥鱼的要求，为每个Tile分配一个唯一的id，便于动画效果实现 owo
#[derive(Clone, Copy)]
pub struct Board {
    cells: [[Tile; SIZE]; SIZE],
    next_id: TileId,
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    /// 空棋盘
    pub fn new() -> Self {
        Self {
            cells: [[Tile::EMPTY; SIZE]; SIZE],
            next_id: FIRST_TILE_ID,
        }
    }

    /// 从二维数组创建棋盘，便于测试与恢复
    pub fn from_values(values: [[u32; SIZE]; SIZE]) -> Result<Self, TileError> {
        let mut board = Self::new();
        for (row, cells) in values.iter().enumerate() {
            for (col, &value) in cells.iter().enumerate() {
                if value == 0 {
                    continue;
                }
                let id = board.alloc_id();
                board.cells[row][col] = Tile::from_value(id, value)?;
            }
        }
        Ok(board)
    }

    /// 获取某个位置的Tile（Tile实现了Copy所以不会被取得所有权）
    pub fn get(&self, pos: Pos) -> Tile {
        self.cells[pos.row][pos.col]
    }

    /// 从棋盘构造二维数组，用于测试
    pub fn values(&self) -> [[u32; SIZE]; SIZE] {
        std::array::from_fn(|row| std::array::from_fn(|col| self.cells[row][col].value()))
    }

    /// 以(Pos, Tile)形式枚举
    pub fn tiles(&self) -> impl Iterator<Item = (Pos, Tile)> + '_ {
        self.cells.iter().enumerate().flat_map(|(row, cells)| {
            cells
                .iter()
                .enumerate()
                .map(move |(col, &tile)| (Pos::new(row, col), tile))
        })
    }

    /// 获取空格
    pub fn empty_positions(&self) -> Vec<Pos> {
        self.tiles()
            .filter(|(_, tile)| tile.is_empty())
            .map(|(pos, _)| pos)
            .collect()
    }

    /// 是否全满
    pub fn is_full(&self) -> bool {
        self.tiles().all(|(_, tile)| !tile.is_empty())
    }

    /// 是否全空
    pub fn is_empty(&self) -> bool {
        self.tiles().all(|(_, tile)| tile.is_empty())
    }

    /// 毗邻格是否有相同值
    /// 最坏时间复杂度：O(SIZE^2)
    fn has_equal_neighbors(&self) -> bool {
        self.tiles().any(|(pos, tile)| {
            [Direction::Right, Direction::Down]
                .iter()
                .filter_map(|&direction| direction.step(pos))
                .any(|neighbor| tile.same_value(self.get(neighbor)))
        })
    }

    /// 是否可移动
    /// 空棋盘视为无法移动
    pub fn can_move(&self) -> bool {
        if self.is_empty() {
            return false;
        }
        !self.is_full() || self.has_equal_neighbors()
    }

    /// 核心：移动操作
    pub fn shift(&mut self, direction: Direction) -> ShiftReport {
        let mut report = ShiftReport::default();
        // 合并标记器，防止二次合并
        let mut absorbed = [[false; SIZE]; SIZE];

        for from in direction.traversal_order() {
            let tile = self.get(from);
            if tile.is_empty() {
                continue;
            }

            // 向前遍历，直到触边或遇到Tile
            let mut landing = from;
            let mut merged = false;
            while let Some(next) = direction.step(landing) {
                let target = self.get(next);
                // 空格直接移动
                if target.is_empty() {
                    landing = next;
                    continue;
                }
                // 相同值触发合并
                if target.same_value(tile) && !absorbed[next.row][next.col] {
                    // 合并逻辑
                    let id = self.alloc_id();
                    let result = tile.doubled(id);
                    absorbed[next.row][next.col] = true;
                    self.cells[next.row][next.col] = result;
                    self.cells[from.row][from.col] = Tile::EMPTY;
                    // 事件记录
                    report.moved = true;
                    report.gained += result.value();
                    report.merges.push(MergeEvent {
                        at: next,
                        id,
                        value: result.value(),
                        consumed: [
                            SlideEvent {
                                id: tile.id(),
                                from,
                                to: next,
                            },
                            SlideEvent {
                                id: target.id(),
                                from: next,
                                to: next,
                            },
                        ],
                    });
                    merged = true;
                }
                break; // 不同值退出
            }

            // 对于不同值的移动处理
            if !merged && landing != from {
                // 移动逻辑
                self.cells[landing.row][landing.col] = tile;
                self.cells[from.row][from.col] = Tile::EMPTY;
                // 事件记录
                report.moved = true;
                report.slides.push(SlideEvent {
                    id: tile.id(),
                    from,
                    to: landing,
                });
            }
        }

        report
    }

    /// 随机产生一个块
    /// 无法产生时返回 None
    pub fn spawn_random(&mut self, rng: &mut Rng) -> Option<SpawnEvent> {
        // 检查
        let empty = self.empty_positions();
        if empty.is_empty() {
            return None;
        }
        // 随机选择一个空格
        let at = empty[rng.below(empty.len())];
        // 随机出2或4的块
        let exponent = if rng.chance(SPAWN_TWO_ODDS.0, SPAWN_TWO_ODDS.1) {
            SPAWN_EXPONENTS[0]
        } else {
            SPAWN_EXPONENTS[1]
        };
        // 产生块
        let id = self.alloc_id();
        let tile =
            Tile::from_exponent(id, exponent).expect("both spawn exponents are below the tile cap");
        self.cells[at.row][at.col] = tile;
        // 事件记录
        Some(SpawnEvent {
            id,
            value: tile.value(),
            at,
        })
    }

    /// id分配
    fn alloc_id(&mut self) -> TileId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(FIRST_TILE_ID);
        id
    }
}

/// 所有块数字相等视为Board相等
impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        self.values() == other.values()
    }
}

/// 显然Board的二元比较满足反身性，即 self == self
impl Eq for Board {}

/// Debug的输出格式
impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in self.values() {
            writeln!(f, "{row:?}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::geometry::CELL_COUNT;

    /// 建棋盘
    fn board_of(values: [[u32; SIZE]; SIZE]) -> Board {
        Board::from_values(values).expect("test board uses valid tile values")
    }

    /// 棋盘转原数据
    fn values(board: &Board) -> [[u32; SIZE]; SIZE] {
        board.values()
    }

    /// 空棋盘相关检测
    #[test]
    fn new_board_is_empty() {
        let board = Board::new();
        assert_eq!(board.values(), [[0; SIZE]; SIZE]);
        assert_eq!(board.empty_positions().len(), 16);
        assert!(board.is_empty());
        assert!(!board.is_full());
        assert!(!board.can_move(), "an empty board has nothing to slide");
    }

    /// 当数据异常（不为2的幂）时抛出错误
    #[test]
    fn from_values_rejects_a_non_power_of_two() {
        let mut grid = [[0u32; SIZE]; SIZE];
        grid[0][0] = 3;
        assert!(Board::from_values(grid).is_err());
    }

    /// 左移相关检测
    #[test]
    fn slide_left_compacts_a_row() {
        let mut board = board_of([[0, 2, 0, 4], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let report = board.shift(Direction::Left);
        assert!(report.moved);
        assert_eq!(values(&board)[0], [2, 4, 0, 0]);
        assert_eq!(report.gained, 0);
        assert_eq!(report.merges.len(), 0);
        assert_eq!(report.slides.len(), 2);
    }

    /// 右移相关检测
    #[test]
    fn slide_right_compacts_a_row() {
        let mut board = board_of([[2, 0, 4, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        assert!(board.shift(Direction::Right).moved);
        assert_eq!(values(&board)[0], [0, 0, 2, 4]);
    }

    /// 上移相关检测
    #[test]
    fn slide_up_compacts_a_column() {
        // Regression guard: the original `tuple()` table was transposed, so this
        // moved the tiles sideways instead.
        let mut board = board_of([[0, 0, 0, 0], [2, 0, 0, 0], [0, 0, 0, 0], [4, 0, 0, 0]]);
        assert!(board.shift(Direction::Up).moved);
        let grid = values(&board);
        assert_eq!(
            [grid[0][0], grid[1][0], grid[2][0], grid[3][0]],
            [2, 4, 0, 0]
        );
        assert_eq!(grid[0][1], 0, "nothing may leak into another column");
    }

    /// 下移相关检测
    #[test]
    fn slide_down_compacts_a_column() {
        let mut board = board_of([[2, 0, 0, 0], [0, 0, 0, 0], [4, 0, 0, 0], [0, 0, 0, 0]]);
        assert!(board.shift(Direction::Down).moved);
        let grid = values(&board);
        assert_eq!(
            [grid[0][0], grid[1][0], grid[2][0], grid[3][0]],
            [0, 0, 2, 4]
        );
    }

    /// 合并检测与分数验证
    #[test]
    fn equal_neighbors_merge_and_score_their_sum() {
        let mut board = board_of([[2, 2, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let report = board.shift(Direction::Left);
        assert_eq!(values(&board)[0], [4, 0, 0, 0]);
        assert_eq!(report.gained, 4);
        assert_eq!(report.merges.len(), 1);
        assert_eq!(report.merges[0].at, Pos::new(0, 0));
        assert_eq!(report.merges[0].value, 4);
    }

    /// 特殊情况：整排相同，不竞争合并、不额外合并
    #[test]
    fn a_whole_row_of_pairs_merges_into_two_tiles() {
        let mut board = board_of([[2, 2, 2, 2], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let report = board.shift(Direction::Left);
        assert_eq!(values(&board)[0], [4, 4, 0, 0]);
        assert_eq!(report.gained, 8);
    }

    /// 特殊情况：合并后的块不额外合并
    #[test]
    fn tiles_merged_this_move_do_not_merge_again() {
        // [2,2,4] must become [4,4], never [8]: the fresh 4 is spent.
        let mut board = board_of([[2, 2, 4, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let report = board.shift(Direction::Left);
        assert_eq!(values(&board)[0], [4, 4, 0, 0]);
        assert_eq!(report.gained, 4);
    }

    /// 特殊情况：不竞争合并
    #[test]
    fn the_first_of_three_equal_tiles_is_the_one_that_merges() {
        // [2,2,2] becomes [4,2] when sliding left, because the leftmost pair wins.
        let mut board = board_of([[2, 2, 2, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        board.shift(Direction::Left);
        assert_eq!(values(&board)[0], [4, 2, 0, 0]);
    }

    /// 特殊情况：不额外合并
    #[test]
    fn two_pairs_merge_into_two_tiles_not_one_quad() {
        let mut board = board_of([[4, 4, 8, 8], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let report = board.shift(Direction::Left);
        assert_eq!(values(&board)[0], [8, 16, 0, 0]);
        assert_eq!(report.gained, 24);
    }

    /// 非毗邻合并
    #[test]
    fn a_gap_between_equal_tiles_still_merges() {
        let mut board = board_of([[2, 0, 0, 2], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        assert!(board.shift(Direction::Left).moved);
        assert_eq!(values(&board)[0], [4, 0, 0, 0]);
    }

    /// 无法移动情况
    #[test]
    fn a_blocked_slide_reports_no_movement() {
        let mut board = board_of([[2, 4, 8, 16], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let before = board;
        let report = board.shift(Direction::Left);
        assert!(!report.moved);
        assert_eq!(report.gained, 0);
        assert!(report.slides.is_empty());
        assert!(report.merges.is_empty());
        assert_eq!(board, before);
    }

    /// 无法移动检测
    #[test]
    fn a_full_board_with_no_equal_neighbors_cannot_move() {
        let board = board_of([[2, 4, 2, 4], [4, 2, 4, 2], [2, 4, 2, 4], [4, 2, 4, 2]]);
        assert!(board.is_full());
        assert!(!board.can_move());
        for direction in Direction::ALL {
            let mut probe = board;
            assert!(
                !probe.shift(direction).moved,
                "{direction:?} should be blocked"
            );
        }
    }

    /// 可移动检测：满棋盘但有相同块
    #[test]
    fn a_full_board_with_one_equal_pair_can_still_move() {
        let board = board_of([
            [2, 2, 4, 8],
            [4, 8, 16, 32],
            [8, 16, 32, 64],
            [16, 32, 64, 128],
        ]);
        assert!(board.is_full());
        assert!(board.can_move());
    }

    /// can_move与shift结果一致性检查
    #[test]
    fn can_move_agrees_with_actually_being_able_to_move() {
        // 检测函数：shift是否可移动
        let movable = |board: Board| {
            Direction::ALL.iter().any(|&direction| {
                let mut probe = board;
                probe.shift(direction).moved
            })
        };

        // 穷举所有2、4棋盘的情况
        // 共2^16种情况
        // index的每一位代表相应位置是否为4
        for index in 0..(1u32 << CELL_COUNT) {
            let grid = std::array::from_fn(|row| {
                std::array::from_fn(|col| {
                    let bit = row * SIZE + col;
                    if index & (1 << bit) == 0 {
                        2
                    } else {
                        4
                    }
                })
            });
            let board = board_of(grid);
            assert!(board.is_full());
            assert_eq!(board.can_move(), movable(board), "disagreement on {grid:?}");
        }

        // 同上，但是0，2棋盘
        for index in 0..(1u32 << CELL_COUNT) {
            let grid = std::array::from_fn(|row| {
                std::array::from_fn(|col| {
                    let bit = row * SIZE + col;
                    if index & (1 << bit) == 0 {
                        0
                    } else {
                        2
                    }
                })
            });
            let board = board_of(grid);
            assert_eq!(board.can_move(), movable(board), "disagreement on {grid:?}");
        }

        // 模拟随机棋盘
        let mut rng = Rng::from_seed(0xDEADBEEF);
        let palette = [0u32, 2, 4, 8, 16, 32];
        for _ in 0..50_000 {
            let grid =
                std::array::from_fn(|_| std::array::from_fn(|_| palette[rng.below(palette.len())]));
            let board = board_of(grid);
            assert_eq!(board.can_move(), movable(board), "disagreement on {grid:?}");
        }
    }

    /// 检查移动事件是否描述了所有块
    #[test]
    fn slide_events_describe_every_surviving_tile_that_moved() {
        let mut board = board_of([[0, 2, 0, 4], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let before: Vec<(Pos, TileId)> = board
            .tiles()
            .filter(|(_, tile)| !tile.is_empty())
            .map(|(pos, tile)| (pos, tile.id()))
            .collect();
        let report = board.shift(Direction::Left);

        for slide in &report.slides {
            let (original, _) = before
                .iter()
                .find(|(_, id)| *id == slide.id)
                .expect("a slide refers to a tile that existed before the move");
            assert_eq!(slide.from, *original);
            assert_eq!(board.get(slide.to).id(), slide.id);
        }
        assert_eq!(report.slides.len(), before.len());
    }

    /// 检查合并事件是否描述了所有块
    #[test]
    fn merges_report_both_consumed_tiles_at_the_destination() {
        let mut board = board_of([[0, 4, 4, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let report = board.shift(Direction::Left);
        let merge = report.merges[0];

        assert_eq!(merge.at, Pos::new(0, 0));
        for consumed in merge.consumed {
            assert_eq!(
                consumed.to, merge.at,
                "both tiles slide into the merge cell"
            );
        }
        assert_ne!(
            merge.consumed[0].id, merge.consumed[1].id,
            "the two consumed tiles are distinct"
        );
        assert_eq!(board.get(merge.at).id(), merge.id);
        assert_eq!(board.get(merge.at).value(), 8);
    }

    /// 检查随机生成的块是否在空格上
    /// 初步检验概率分布
    #[test]
    fn spawn_lands_on_an_empty_cell_and_respects_the_odds() {
        let mut rng = Rng::from_seed(99);
        let mut seen_two = false;
        let mut seen_four = false;

        for _ in 0..200 {
            // 强制只留出一个块
            let mut board =
                board_of([[0, 2, 4, 8], [2, 4, 8, 16], [4, 8, 16, 32], [8, 16, 32, 64]]);
            let event = board.spawn_random(&mut rng).expect("one cell is free");
            assert_eq!(event.at, Pos::new(0, 0));
            assert!(event.value == 2 || event.value == 4, "got {}", event.value);
            assert_eq!(board.get(event.at).value(), event.value);
            assert_eq!(board.get(event.at).id(), event.id);
            match event.value {
                2 => seen_two = true,
                _ => seen_four = true,
            }
        }

        assert!(seen_two && seen_four, "both spawn values should appear");
    }

    /// 无法生成块返回None测试
    #[test]
    fn spawn_returns_none_when_the_board_is_full() {
        let mut rng = Rng::from_seed(3);
        let mut board = board_of([[2, 4, 2, 4], [4, 2, 4, 2], [2, 4, 2, 4], [4, 2, 4, 2]]);
        assert_eq!(board.spawn_random(&mut rng), None);
    }

    /// 空棋盘不能移动
    /// 起始棋盘可以移动
    #[test]
    fn an_empty_board_cannot_move_but_a_starting_board_can() {
        assert!(!Board::new().can_move());
        assert!(board_of([[2, 0, 0, 0], [0, 0, 0, 0], [0, 0, 2, 0], [0, 0, 0, 0],]).can_move());
    }

    /// Tile id唯一性检查
    #[test]
    fn tile_identities_are_unique_while_alive_and_never_come_back() {
        let mut rng = Rng::from_seed(5);
        let mut board = board_of([[2, 2, 2, 2], [2, 2, 2, 2], [2, 2, 2, 2], [2, 2, 2, 2]]);

        let mut alive: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut retired: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut created_total = 0;

        // 模拟游戏过程
        for step in 0..80 {
            if board
                .shift(Direction::ALL[step % Direction::ALL.len()])
                .moved
            {
                board.spawn_random(&mut rng);
            }

            let mut now = std::collections::HashSet::new();
            for (_, tile) in board.tiles() {
                if tile.is_empty() {
                    continue;
                }
                assert_ne!(tile.id(), 0, "0 is reserved for the empty cell");
                assert!(
                    now.insert(tile.id()),
                    "identity {} appears twice",
                    tile.id()
                );
                assert!(
                    !retired.contains(&tile.id()),
                    "identity {} came back from the dead",
                    tile.id()
                );
            }

            retired.extend(alive.difference(&now).copied());
            created_total += now.difference(&alive).count();
            alive = now;
        }

        assert!(
            created_total > 16,
            "tiles should have been created and merged, saw {created_total}"
        );
    }

    /// 检查相等性是否忽略Tile id
    #[test]
    fn equality_ignores_tile_identity() {
        let first = board_of([[2, 4, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let second = board_of([[2, 4, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        assert_eq!(first, second);
        assert_ne!(first.get(Pos::new(0, 0)).id(), 0);
    }

    /// 检查合并逻辑
    #[test]
    fn a_slide_removes_exactly_one_tile_per_merge_and_scores_what_it_creates() {
        // Invariants that must hold for every move, checked over a long run.
        let mut rng = Rng::from_seed(1234);
        let mut board = Board::new();
        board.spawn_random(&mut rng);
        board.spawn_random(&mut rng);

        let mut total_gained = 0;
        let mut moves = 0;
        for step in 0..400 {
            let before = board.tiles().filter(|(_, tile)| !tile.is_empty()).count();
            let report = board.shift(Direction::ALL[step % Direction::ALL.len()]);
            if !report.moved {
                continue;
            }
            moves += 1;
            total_gained += report.gained;

            assert_eq!(
                board.tiles().filter(|(_, tile)| !tile.is_empty()).count(),
                before - report.merges.len(),
                "each merge consumes two tiles and creates one"
            );
            assert_eq!(
                report.gained,
                report.merges.iter().map(|merge| merge.value).sum::<u32>(),
                "the score is the sum of the tiles that were created"
            );
            board.spawn_random(&mut rng);
        }

        assert!(moves > 0, "the board should have been able to move");
        assert!(
            total_gained > 0,
            "several hundred slides should score something"
        );
    }
}
