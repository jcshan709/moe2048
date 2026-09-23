//! 前端调用的后端命令
//!
//! 每一个指令都对底层操作进行浅包装
//! 具体操作为：
//! - 加互斥锁
//! - 调用底层操作
//! - 返回序列化后的结果

use std::fmt;
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::game::{Direction, Game, GameState, MoveOutcome};

/// 一次游戏的 Session
///
/// Tauri 的命令是并发执行的，所以需要互斥锁阻止竞争访问
#[derive(Debug)]
pub struct GameSession(Mutex<Game>);

impl GameSession {
    pub fn new() -> Self {
        Self(Mutex::new(Game::new()))
    }

    /// 对 action 调用的包装
    /// Rust的线程在运行时如果触发了panic，会将锁标记为poisoned，后续的调用会返回错误
    /// 所以需要在包装层处理这种情况
    fn with<T>(&self, action: impl FnOnce(&mut Game) -> T) -> Result<T, ApiError> {
        let mut game = self.0.lock().map_err(|_| {
            ApiError::new("the game state was left inconsistent by an earlier panic")
        })?;
        Ok(action(&mut game))
    }
}

impl Default for GameSession {
    fn default() -> Self {
        Self::new()
    }
}

/// 前端错误包装器
#[derive(Debug, Serialize)]
pub struct ApiError {
    message: String,
}

impl ApiError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

/// 开始新游戏
#[tauri::command]
pub fn new_game(session: State<'_, GameSession>) -> Result<GameState, ApiError> {
    session.with(|game| game.reset())
}

/// 获取当前状态
#[tauri::command]
pub fn game_state(session: State<'_, GameSession>) -> Result<GameState, ApiError> {
    session.with(|game| game.state())
}

/// 移动操作
#[tauri::command]
pub fn make_move(
    direction: Direction,
    session: State<'_, GameSession>,
) -> Result<MoveOutcome, ApiError> {
    session.with(|game| game.apply(direction))
}

/// 回退操作
#[tauri::command]
pub fn undo_move(session: State<'_, GameSession>) -> Result<Option<GameState>, ApiError> {
    session.with(|game| game.undo())
}

/// 关闭win提示
#[tauri::command]
pub fn keep_playing(session: State<'_, GameSession>) -> Result<GameState, ApiError> {
    session.with(|game| game.acknowledge_win())
}
