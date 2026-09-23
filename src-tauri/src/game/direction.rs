//! 移动方向

use serde::{Deserialize, Serialize};

use super::geometry::{Pos, CELL_COUNT, SIZE};

/// 移动方向
/// 以lowercase序列化，方便前端直接调用
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// 便于遍历所有方向
    pub const ALL: [Self; 4] = [Self::Up, Self::Down, Self::Left, Self::Right];

    /// 将方向转换为系数
    pub const fn delta(self) -> (isize, isize) {
        match self {
            Self::Up => (-1, 0),
            Self::Down => (1, 0),
            Self::Left => (0, -1),
            Self::Right => (0, 1),
        }
    }

    /// 返回毗邻格子的坐标或 None（超范围）
    pub fn step(self, pos: Pos) -> Option<Pos> {
        pos.offset(self.delta())
    }

    /// 移动时的遍历顺序
    pub fn traversal_order(self) -> [Pos; CELL_COUNT] {
        std::array::from_fn(|index| {
            // index = row * SIZE + col
            // 注：这里的row和col仅为名称，在Left和Right情况下意义应交换
            let (row, col) = (index / SIZE, index % SIZE);
            match self {
                Self::Up => Pos::new(row, col),
                Self::Down => Pos::new(SIZE - 1 - row, col),
                Self::Left => Pos::new(col, row),
                Self::Right => Pos::new(col, SIZE - 1 - row),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 方向的系数化检查
    #[test]
    fn delta_matches_the_visual_direction() {
        assert_eq!(Direction::Up.delta(), (-1, 0));
        assert_eq!(Direction::Down.delta(), (1, 0));
        assert_eq!(Direction::Left.delta(), (0, -1));
        assert_eq!(Direction::Right.delta(), (0, 1));
    }

    /// 方向的移动检查
    #[test]
    fn step_moves_along_the_delta() {
        let centre = Pos::new(1, 1);
        assert_eq!(Direction::Up.step(centre), Some(Pos::new(0, 1)));
        assert_eq!(Direction::Down.step(centre), Some(Pos::new(2, 1)));
        assert_eq!(Direction::Left.step(centre), Some(Pos::new(1, 0)));
        assert_eq!(Direction::Right.step(centre), Some(Pos::new(1, 2)));
    }

    /// 方向的移动检查（边界处理）
    #[test]
    fn step_stops_at_the_edges() {
        assert_eq!(Direction::Up.step(Pos::new(0, 2)), None);
        assert_eq!(Direction::Down.step(Pos::new(3, 2)), None);
        assert_eq!(Direction::Left.step(Pos::new(2, 0)), None);
        assert_eq!(Direction::Right.step(Pos::new(2, 3)), None);
    }

    /// 遍历顺序检查（起始位）
    #[test]
    fn traversal_starts_at_the_edge_tiles_move_towards() {
        assert_eq!(Direction::Up.traversal_order()[0], Pos::new(0, 0));
        assert_eq!(Direction::Down.traversal_order()[0], Pos::new(3, 0));
        assert_eq!(Direction::Left.traversal_order()[0], Pos::new(0, 0));
        assert_eq!(Direction::Right.traversal_order()[0], Pos::new(0, 3));
    }

    /// 遍历顺序检查（1、4关键位）
    #[test]
    fn traversal_walks_rows_for_vertical_and_columns_for_horizontal() {
        // Sliding up walks row 0 across, then row 1, and so on.
        assert_eq!(Direction::Up.traversal_order()[1], Pos::new(0, 1));
        assert_eq!(Direction::Up.traversal_order()[4], Pos::new(1, 0));
        assert_eq!(Direction::Down.traversal_order()[1], Pos::new(3, 1));
        assert_eq!(Direction::Down.traversal_order()[4], Pos::new(2, 0));

        // Sliding left walks column 0 down, then column 1, and so on.
        assert_eq!(Direction::Left.traversal_order()[1], Pos::new(1, 0));
        assert_eq!(Direction::Left.traversal_order()[4], Pos::new(0, 1));
        assert_eq!(Direction::Right.traversal_order()[1], Pos::new(1, 3));
        assert_eq!(Direction::Right.traversal_order()[4], Pos::new(0, 2));
    }

    /// 遍历唯一性检查
    #[test]
    fn traversal_visits_every_cell_exactly_once() {
        for direction in Direction::ALL {
            let mut seen = std::collections::HashSet::new();
            for pos in direction.traversal_order() {
                assert!(seen.insert(pos), "{direction:?} revisited {pos:?}");
            }
            assert_eq!(seen.len(), CELL_COUNT);
        }
    }

    /// 遍历顺序模拟检查
    /// 原理：
    /// 遍历到某格时检查其移动方向上的毗邻格是否已遍历
    /// 如 A B C D 左移，A必定在B前遍历
    /// 即A的index要小于B
    #[test]
    fn traversal_covers_cells_in_decreasing_reachability() {
        for direction in Direction::ALL {
            let order = direction.traversal_order();
            for (index, pos) in order.iter().enumerate() {
                if let Some(ahead) = direction.step(*pos) {
                    let ahead_index = order
                        .iter()
                        .position(|candidate| *candidate == ahead)
                        .expect("every cell is in the order");
                    assert!(
                        ahead_index < index,
                        "{direction:?}: {pos:?} is visited before the cell it slides into ({ahead:?})"
                    );
                }
            }
        }
    }

    /// 反序列化测试
    #[test]
    fn directions_deserialize_from_lowercase_names() {
        for (name, direction) in [
            ("\"up\"", Direction::Up),
            ("\"down\"", Direction::Down),
            ("\"left\"", Direction::Left),
            ("\"right\"", Direction::Right),
        ] {
            let parsed: Direction = serde_json::from_str(name).expect("valid direction");
            assert_eq!(parsed, direction);
        }
    }
}
