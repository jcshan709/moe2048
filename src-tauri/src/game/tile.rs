//! 块

use std::fmt;

use super::geometry::SIZE;

/// Tile id
pub type TileId = u32;

pub const MIN_EXPONENT: u8 = 1;
pub const WINNING_EXPONENT: u8 = 11;
pub const WINNING_VALUE: u32 = 1 << WINNING_EXPONENT;

/// 程序能生成的最大数
/// 不是指2048的理论最大
pub const MAX_EXPONENT: u8 = 30;

/// Tile
/// 0表示空位
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tile {
    id: TileId,
    exponent: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileError {
    value: u32,
}

impl TileError {
    pub fn value(self) -> u32 {
        self.value
    }
}

impl fmt::Display for TileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} is not a tile value: expected 0 or a power of two between 2 and 2^{MAX_EXPONENT}",
            self.value
        )
    }
}

impl std::error::Error for TileError {}

impl fmt::Debug for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tile")
            .field("id", &self.id)
            .field("value", &self.value())
            .finish()
    }
}

impl Tile {
    pub const EMPTY: Self = Self { id: 0, exponent: 0 };

    pub const fn from_exponent(id: TileId, exponent: u8) -> Option<Self> {
        if exponent == 0 || exponent > MAX_EXPONENT {
            return None;
        }
        Some(Self { id, exponent })
    }

    pub fn from_value(id: TileId, value: u32) -> Result<Self, TileError> {
        if value == 0 {
            return Ok(Self::EMPTY);
        }
        let exponent = value.trailing_zeros();
        if value.is_power_of_two() && exponent <= u32::from(MAX_EXPONENT) {

            return Ok(Self {
                id,
                exponent: exponent as u8,
            });
        }
        Err(TileError { value })
    }

    pub const fn id(self) -> TileId {
        self.id
    }

    pub const fn exponent(self) -> u8 {
        self.exponent
    }

    pub const fn value(self) -> u32 {
        if self.exponent == 0 {
            0
        } else {
            1u32 << self.exponent
        }
    }

    pub const fn is_empty(self) -> bool {
        self.exponent == 0
    }

    pub const fn same_value(self, other: Self) -> bool {
        self.exponent == other.exponent
    }

    pub const fn doubled(self, id: TileId) -> Self {
        let exponent = if self.exponent >= MAX_EXPONENT {
            MAX_EXPONENT
        } else {
            self.exponent + 1
        };
        Self { id, exponent }
    }

    pub const fn is_winning(self) -> bool {
        self.exponent >= WINNING_EXPONENT
    }

    /// 测试用
    pub const fn max_value() -> u32 {
        1u32 << MAX_EXPONENT
    }
}

pub const SPAWN_EXPONENTS: [u8; 2] = [MIN_EXPONENT, MIN_EXPONENT + 1];

/// 生成最大值不能小于理论最大值
const _: () = assert!(
    (MAX_EXPONENT as usize) > SIZE * SIZE,
    "MAX_EXPONENT must exceed the reachable exponent, otherwise merged tiles saturate"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tile_has_no_value() {
        assert!(Tile::EMPTY.is_empty());
        assert_eq!(Tile::EMPTY.value(), 0);
        assert_eq!(Tile::EMPTY.id(), 0);
    }

    #[test]
    fn value_and_exponent_round_trip() {
        for exponent in MIN_EXPONENT..=MAX_EXPONENT {
            let tile = Tile::from_exponent(1, exponent).expect("in range");
            assert_eq!(tile.value(), 1u32 << exponent);
            assert_eq!(Tile::from_value(1, tile.value()), Ok(tile));
        }
    }

    #[test]
    fn from_value_accepts_zero_as_the_empty_cell() {
        assert_eq!(Tile::from_value(7, 0), Ok(Tile::EMPTY));
    }

    #[test]
    fn from_value_rejects_non_powers_of_two() {
        for value in [3, 6, 100, 2047] {
            assert_eq!(Tile::from_value(1, value), Err(TileError { value }));
        }
    }

    #[test]
    fn from_value_rejects_values_above_the_cap() {
        assert!(Tile::from_value(1, Tile::max_value() * 2).is_err());
        assert!(Tile::from_exponent(1, MAX_EXPONENT + 1).is_none());
    }

    #[test]
    fn same_value_ignores_identity() {
        let a = Tile::from_value(1, 4).expect("valid");
        let b = Tile::from_value(9, 4).expect("valid");
        assert_ne!(a, b);
        assert!(a.same_value(b));
        assert!(!a.same_value(Tile::from_value(2, 8).expect("valid")));
    }

    #[test]
    fn doubling_saturates_instead_of_overflowing() {
        let capped = Tile::from_exponent(1, MAX_EXPONENT).expect("in range");
        assert_eq!(capped.doubled(2).value(), Tile::max_value());
    }

    #[test]
    fn winning_tile_is_2048() {
        assert!(Tile::from_value(1, 2048).expect("valid").is_winning());
        assert!(!Tile::from_value(1, 1024).expect("valid").is_winning());
    }

    #[test]
    fn spawn_exponents_are_two_and_four() {
        let values: Vec<u32> = SPAWN_EXPONENTS
            .iter()
            .map(|&e| Tile::from_exponent(1, e).expect("in range").value())
            .collect();
        assert_eq!(values, vec![2, 4]);
    }
}
