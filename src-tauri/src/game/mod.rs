//! 底层2048逻辑代码，与前端完全分离
//!
//! - `geometry` — 坐标相关
//! - `tile` — 块
//! - `direction` — 方向
//! - `rng` — 带种子的随机数生成器，用于测试复现
//! - `board` — 棋盘与规则
//! - `state` — 游戏状态机

mod board;
mod direction;
mod geometry;
mod rng;
mod state;
mod tile;

pub use board::{Board, MergeEvent, ShiftReport, SlideEvent, SpawnEvent};
pub use direction::Direction;
pub use geometry::{Pos, CELL_COUNT, SIZE};
pub use rng::Rng;
pub use state::{Game, GameState, GameStatus, MoveOutcome, TileView};
pub use tile::{Tile, TileError, TileId, WINNING_VALUE};
