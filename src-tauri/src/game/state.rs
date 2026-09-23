//! 游戏状态

use serde::Serialize;

use super::board::{Board, MergeEvent, SlideEvent, SpawnEvent};
use super::direction::Direction;
use super::geometry::{Pos, SIZE};
use super::rng::Rng;

/// 最大回退数
const UNDO_LIMIT: usize = 1;

/// 游戏状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GameStatus {
    Playing,
    Won,
    Lost,
}

/// Tile的ViewModel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TileView {
    pub id: u32,
    pub value: u32,
    pub at: Pos,
}

/// 状态机
///
/// 每次操作后均会发送，以同步前端状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub board: [[u32; SIZE]; SIZE],
    pub tiles: Vec<TileView>,
    pub score: u32,
    pub moves: u32,
    pub status: GameStatus,
    pub can_undo: bool,
}

/// 整个操作的结果，用于前端通信
/// 包含了整个 ShiftReport 的同时也记录了产生事件和当前状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveOutcome {
    pub state: GameState,
    pub moved: bool,
    pub gained: u32,
    pub slides: Vec<SlideEvent>,
    pub merges: Vec<MergeEvent>,
    pub spawn: Option<SpawnEvent>,
}

/// 快照，用于恢复
#[derive(Debug, Clone, Copy)]
struct Snapshot {
    board: Board,
    score: u32,
    moves: u32,
    status: GameStatus,
    win_acknowledged: bool,
}

/// 整个游戏的状态
#[derive(Debug)]
pub struct Game {
    board: Board,
    score: u32,
    moves: u32,
    status: GameStatus,
    rng: Rng,
    undo_stack: Vec<Snapshot>,
    /// 当win窗口关闭后置true
    win_acknowledged: bool,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Self::with_rng(Rng::from_entropy())
    }

    pub fn with_seed(seed: u64) -> Self {
        Self::with_rng(Rng::from_seed(seed))
    }

    fn with_rng(rng: Rng) -> Self {
        let mut game = Self {
            board: Board::new(),
            score: 0,
            moves: 0,
            status: GameStatus::Playing,
            rng,
            undo_stack: Vec::new(),
            win_acknowledged: false,
        };

        game.board.spawn_random(&mut game.rng);
        game.board.spawn_random(&mut game.rng);
        game
    }

    pub fn state(&self) -> GameState {
        GameState {
            board: self.board.values(),
            tiles: self
                .board
                .tiles()
                .filter(|(_, tile)| !tile.is_empty())
                .map(|(pos, tile)| TileView {
                    id: tile.id(),
                    value: tile.value(),
                    at: pos,
                })
                .collect(),
            score: self.score,
            moves: self.moves,
            status: self.status,
            can_undo: !self.undo_stack.is_empty(),
        }
    }

    /// 移动并产生块
    pub fn apply(&mut self, direction: Direction) -> MoveOutcome {
        let before = self.snapshot();
        let report = self.board.shift(direction);

        if !report.moved {
            return MoveOutcome {
                state: self.state(),
                moved: false,
                gained: 0,
                slides: Vec::new(),
                merges: Vec::new(),
                spawn: None,
            };
        }

        self.score += report.gained;
        self.moves += 1;
        self.push_undo(before);

        let spawn = self.board.spawn_random(&mut self.rng);
        self.refresh_status();

        MoveOutcome {
            state: self.state(),
            moved: true,
            gained: report.gained,
            slides: report.slides,
            merges: report.merges,
            spawn,
        }
    }

    /// 回退一次移动
    pub fn undo(&mut self) -> Option<GameState> {
        let snapshot = self.undo_stack.pop()?;
        self.board = snapshot.board;
        self.score = snapshot.score;
        self.moves = snapshot.moves;
        self.status = snapshot.status;
        self.win_acknowledged = snapshot.win_acknowledged;
        Some(self.state())
    }

    /// 关闭win窗口时的记录
    pub fn acknowledge_win(&mut self) -> GameState {
        self.win_acknowledged = true;
        // Re-derive the status: the board may also be in a losing position.
        self.refresh_status();
        self.state()
    }

    pub fn reset(&mut self) -> GameState {
        let rng = self.rng;
        *self = Self::with_rng(rng);
        self.state()
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            board: self.board,
            score: self.score,
            moves: self.moves,
            status: self.status,
            win_acknowledged: self.win_acknowledged,
        }
    }

    fn push_undo(&mut self, snapshot: Snapshot) {
        if self.undo_stack.len() == UNDO_LIMIT {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(snapshot);
    }

    /// 刷新游戏状态
    fn refresh_status(&mut self) {
        let won = !self.win_acknowledged && self.board.tiles().any(|(_, tile)| tile.is_winning());

        self.status = if won {
            GameStatus::Won
        } else if self.board.can_move() {
            GameStatus::Playing
        } else {
            GameStatus::Lost
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::tile::WINNING_VALUE;

    /// 一个失败的Board
    const STUCK_GRID: [[u32; SIZE]; SIZE] =
        [[2, 4, 2, 4], [4, 2, 4, 2], [2, 4, 2, 4], [4, 2, 4, 2]];

    fn stuck_board() -> Board {
        Board::from_values(STUCK_GRID).expect("valid board")
    }

    #[test]
    fn a_new_game_starts_with_two_tiles_and_no_score() {
        let game = Game::with_seed(1);
        let state = game.state();
        assert_eq!(state.tiles.len(), 2);
        assert_eq!(state.score, 0);
        assert_eq!(state.moves, 0);
        assert_eq!(state.status, GameStatus::Playing);
        assert!(!state.can_undo);

        for tile in &state.tiles {
            assert!(tile.value == 2 || tile.value == 4, "got {}", tile.value);
        }
        let occupied: Vec<(usize, usize)> = state
            .tiles
            .iter()
            .map(|tile| (tile.at.row, tile.at.col))
            .collect();
        assert_ne!(occupied[0], occupied[1], "the two tiles do not overlap");
    }

    #[test]
    fn the_state_grid_matches_the_tile_list() {
        let game = Game::with_seed(2);
        let state = game.state();
        for tile in &state.tiles {
            assert_eq!(state.board[tile.at.row][tile.at.col], tile.value);
        }
        let from_grid: u32 = state.board.iter().flatten().sum();
        let from_tiles: u32 = state.tiles.iter().map(|tile| tile.value).sum();
        assert_eq!(from_grid, from_tiles);
    }

    #[test]
    fn a_move_updates_score_moves_and_history() {
        let mut game = Game::with_seed(1);
        let outcome = Direction::ALL
            .iter()
            .find_map(|&direction| {
                let outcome = game.apply(direction);
                outcome.moved.then_some(outcome)
            })
            .expect("some direction moves on a fresh board");

        assert!(outcome.moved);
        assert_eq!(outcome.state.moves, 1);
        assert!(outcome.state.can_undo);
        assert_eq!(
            outcome.gained, 0,
            "no merge on the first move of a new board"
        );
        assert!(outcome.spawn.is_some(), "a move spawns a tile");
        assert_eq!(outcome.state.tiles.len(), 3);
    }

    #[test]
    fn a_blocked_move_is_a_complete_no_op() {
        let mut game = Game::with_seed(3);
        // A row already packed to the left with nothing to merge: sliding left
        // cannot change anything.
        game.board = Board::from_values([[2, 4, 8, 16], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]])
            .expect("valid board");
        game.refresh_status();

        let before = game.state();
        for attempt in 0..5 {
            let outcome = game.apply(Direction::Left);
            assert!(!outcome.moved, "attempt {attempt} should be blocked");
            assert_eq!(outcome.gained, 0);
            assert!(outcome.spawn.is_none(), "a blocked move must not spawn");
            assert!(outcome.slides.is_empty());
            assert!(outcome.merges.is_empty());
        }

        let after = game.state();
        assert_eq!(after.board, before.board);
        assert_eq!(after.tiles, before.tiles);
        assert_eq!(after.moves, before.moves);
        assert_eq!(after.score, before.score);
        assert_eq!(after.can_undo, before.can_undo);
    }

    #[test]
    fn undo_restores_the_exact_previous_position() {
        let mut game = Game::with_seed(7);
        game.apply(Direction::Left);
        let expected = game.state();

        let outcome = game.apply(Direction::Down);
        assert!(outcome.moved);
        let restored = game.undo().expect("there is a move to undo");

        assert_eq!(restored.board, expected.board);
        assert_eq!(
            restored.tiles, expected.tiles,
            "tile identities come back too"
        );
        assert_eq!(restored.score, expected.score);
        assert_eq!(restored.moves, expected.moves);
        assert_eq!(restored.status, expected.status);
    }

    #[test]
    fn undo_returns_none_on_a_fresh_game() {
        let mut game = Game::with_seed(1);
        assert_eq!(game.undo(), None);
    }

    #[test]
    fn undo_is_limited_but_never_panics() {
        let mut game = Game::with_seed(11);
        for step in 0..200 {
            game.apply(Direction::ALL[step % Direction::ALL.len()]);
        }
        let mut undone = 0;
        while game.undo().is_some() {
            undone += 1;
            assert!(undone <= UNDO_LIMIT, "undo history grew past its limit");
        }
        assert_eq!(undone, UNDO_LIMIT);
    }

    #[test]
    fn undo_can_be_repeated_to_walk_the_whole_history_back() {
        let mut game = Game::with_seed(13);
        let mut history = vec![game.state()];

        for step in 0..10 {
            if game
                .apply(Direction::ALL[step % Direction::ALL.len()])
                .moved
            {
                history.push(game.state());
            }
        }
        assert!(
            history.len() > 1,
            "the game should have moved at least once"
        );

        for expected in history.iter().rev() {
            assert_eq!(&game.state(), expected);
            game.undo();
        }
        assert_eq!(game.undo(), None, "history is exhausted");
    }

    #[test]
    fn a_stuck_board_is_reported_as_lost() {
        let mut game = Game::with_seed(1);
        game.board = stuck_board();
        game.refresh_status();

        assert_eq!(game.state().status, GameStatus::Lost);
        // Losing must agree with the slide rule, not merely with a heuristic.
        for direction in Direction::ALL {
            assert!(!game.apply(direction).moved);
        }
        assert_eq!(game.state().status, GameStatus::Lost, "still stuck");
    }

    #[test]
    fn a_playable_board_is_reported_as_playing() {
        let mut game = Game::with_seed(1);
        game.refresh_status();
        assert_eq!(game.state().status, GameStatus::Playing);
        assert!(game.board.can_move());
    }

    #[test]
    fn undo_recovers_from_a_lost_position() {
        let before = Board::from_values([[2, 4, 2, 4], [4, 2, 4, 2], [2, 4, 2, 8], [0, 4, 2, 16]])
            .expect("valid board");
        let expected_grid = before.values();

        let mut game = Game::with_seed(1);
        game.board = before;

        let outcome = game.apply(Direction::Left);
        assert!(outcome.moved);
        assert_eq!(
            outcome.state.status,
            GameStatus::Lost,
            "the spawn always lands in a cell that cannot merge"
        );

        let restored = game.undo().expect("the losing move is undoable");
        assert_eq!(restored.status, GameStatus::Playing);
        assert_eq!(restored.board, expected_grid, "the free cell is free again");
        assert!(game.board.can_move());
    }

    #[test]
    fn acknowledging_a_win_on_a_stuck_board_reveals_the_loss() {
        let mut game = Game::with_seed(1);
        let mut grid = STUCK_GRID;
        grid[3][3] = 0;
        grid[0][0] = 2048;
        grid[0][1] = 4;
        game.board = Board::from_values(grid).expect("valid board");

        // A 2048 tile is present, so the first report is a win.
        game.refresh_status();
        assert_eq!(game.state().status, GameStatus::Won);

        // Dismissing it must re-derive the truth rather than assume "playing".
        let state = game.acknowledge_win();
        assert_eq!(state.status, GameStatus::Playing);
        assert!(game.board.can_move(), "the free cell still allows a move");
    }

    #[test]
    fn reaching_2048_wins_once_and_can_be_acknowledged() {
        let mut game = Game::with_seed(1);

        game.board =
            Board::from_values([[1024, 1024, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]])
                .expect("valid board");

        let outcome = game.apply(Direction::Left);
        assert!(outcome.moved);
        assert_eq!(outcome.gained, WINNING_VALUE);
        assert_eq!(outcome.state.status, GameStatus::Won);

        let again = game.apply(Direction::Down);
        assert_eq!(again.state.status, GameStatus::Won);

        let acknowledged = game.acknowledge_win();
        assert_eq!(acknowledged.status, GameStatus::Playing);

        for step in 0..8 {
            let outcome = game.apply(Direction::ALL[step % Direction::ALL.len()]);
            assert_ne!(outcome.state.status, GameStatus::Won);
        }
    }

    #[test]
    fn reset_clears_score_history_and_status() {
        let mut game = Game::with_seed(17);
        for step in 0..20 {
            game.apply(Direction::ALL[step % Direction::ALL.len()]);
        }
        let state = game.reset();

        assert_eq!(state.score, 0);
        assert_eq!(state.moves, 0);
        assert_eq!(state.status, GameStatus::Playing);
        assert!(!state.can_undo);
        assert_eq!(state.tiles.len(), 2);
    }

    #[test]
    fn the_same_seed_produces_the_same_game() {
        let play = |seed| {
            let mut game = Game::with_seed(seed);
            let mut states = vec![game.state()];
            for step in 0..25 {
                game.apply(Direction::ALL[step % Direction::ALL.len()]);
                states.push(game.state());
            }
            states
        };
        assert_eq!(play(2024), play(2024));
        assert_ne!(play(2024), play(2025));
    }

    #[test]
    fn score_only_ever_grows_while_playing() {
        let mut game = Game::with_seed(31);
        let mut previous = game.state().score;
        for step in 0..300 {
            let outcome = game.apply(Direction::ALL[step % Direction::ALL.len()]);
            if outcome.state.status == GameStatus::Lost {
                break;
            }
            assert!(outcome.state.score >= previous);
            assert_eq!(outcome.state.score, previous + outcome.gained);
            previous = outcome.state.score;
        }
    }

    #[test]
    fn the_board_never_loses_a_tile_without_a_merge() {
        let mut game = Game::with_seed(41);
        for step in 0..200 {
            let before = game.state().tiles.len();
            let outcome = game.apply(Direction::ALL[step % Direction::ALL.len()]);
            if !outcome.moved {
                continue;
            }
            let expected = before - outcome.merges.len() + usize::from(outcome.spawn.is_some());
            assert_eq!(outcome.state.tiles.len(), expected);
        }
    }
}
