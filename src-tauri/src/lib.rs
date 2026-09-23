//! 应用

mod commands;
pub mod game;

pub use game::{Direction, Game, GameState, GameStatus, MoveOutcome};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::GameSession::new())
        .invoke_handler(tauri::generate_handler![
            commands::new_game,
            commands::game_state,
            commands::make_move,
            commands::undo_move,
            commands::keep_playing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
