//! 坐标相关

use serde::{Deserialize, Serialize};

/// Board 的边长
pub const SIZE: usize = 4;

/// 格子数
pub const CELL_COUNT: usize = SIZE * SIZE;

/// 坐标
/// 采取命名字段而非匿名字段(usize, usize)来避免混淆
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

impl Pos {
    /// 创建一个坐标
    /// 超出范围会panic
    /// 以此保证坐标始终可以作为合法下标
    pub fn new(row: usize, col: usize) -> Self {
        assert!(
            row < SIZE && col < SIZE,
            "position ({row}, {col}) is outside a {SIZE}x{SIZE} board"
        );
        Self { row, col }
    }

    /// 返回偏移坐标
    /// 若在范围外则返回 None
    pub fn offset(self, (delta_row, delta_col): (isize, isize)) -> Option<Self> {
        let row = self.row.checked_add_signed(delta_row)?;
        let col = self.col.checked_add_signed(delta_col)?;
        (row < SIZE && col < SIZE).then_some(Self { row, col })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 偏移坐标时保持在范围内
    #[test]
    fn offset_stays_inside_the_board() {
        let origin = Pos::new(0, 0);
        assert_eq!(origin.offset((1, 1)), Some(Pos::new(1, 1)));
        assert_eq!(origin.offset((0, 0)), Some(origin));
    }

    /// 偏移坐标时超出范围返回 None
    /// 偏移量为 1
    #[test]
    fn offset_reports_leaving_the_board() {
        let origin = Pos::new(0, 0);
        assert_eq!(origin.offset((-1, 0)), None);
        assert_eq!(origin.offset((0, -1)), None);
        assert_eq!(Pos::new(3, 3).offset((1, 0)), None);
        assert_eq!(Pos::new(3, 3).offset((0, 1)), None);
    }

    /// 偏移量超出范围时返回 None
    /// 偏移量不为 1
    #[test]
    fn far_offsets_do_not_wrap() {
        assert_eq!(Pos::new(0, 0).offset((-1000, 0)), None);
        assert_eq!(Pos::new(0, 0).offset((0, -1000)), None);
    }

    /// 创建一个坐标时超出范围会panic
    #[test]
    #[should_panic(expected = "outside a 4x4 board")]
    fn new_rejects_out_of_range_rows() {
        Pos::new(4, 0);
    }
}
