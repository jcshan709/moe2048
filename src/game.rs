//! [`Board`] 之上的可变游戏状态：分数、阶段，以及“只能撤回一步”的撤回槽。

use std::time::{Duration, Instant};

use crate::board::{Board, Direction, SlideOutcome};
use crate::rng::GameRng;

/// 临时提示消息在屏幕上停留的时间。
const STATUS_TTL: Duration = Duration::from_millis(1800);

/// 出现这个数字就算通关（之后仍可继续玩）。
const WIN_TILE: u32 = 2048;

/// 游戏当前所处的阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// 还能继续滑动。
    Playing,
    /// 已经出现 [`WIN_TILE`]。
    Won,
    /// 任何方向都滑不动了。
    Over,
}

/// 提示消息的语气，用来决定颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Warning,
}

/// 尝试滑动棋盘的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcome {
    /// 棋盘发生了变化，并已补充新数字。
    Moved,
    /// 这个方向上什么都动不了。
    Blocked,
    /// 游戏已经结束，本次操作被忽略。
    Finished,
}

/// 恢复游戏所需的全部状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Snapshot {
    board: Board,
    score: u64,
    moves: u64,
    phase: Phase,
}

/// 一条临时提示消息。
#[derive(Debug, Clone)]
struct Status {
    text: String,
    kind: StatusKind,
    expires_at: Instant,
}

/// 一整局游戏。
pub struct Game {
    board: Board,
    rng: GameRng,
    score: u64,
    moves: u64,
    phase: Phase,
    /// 每次成功的滑动都会写入这里，被 [`Game::undo`] 取走。
    undo: Option<Snapshot>,
    /// 被撤回操作清零——这正是“不能连续撤回”的实现方式。
    undo_armed: bool,
    status: Option<Status>,
}

impl Game {
    /// 开一局新游戏，棋盘上先摆好两个数字。
    pub fn new(rng: GameRng) -> Self {
        let mut game = Self {
            board: Board::default(),
            rng,
            score: 0,
            moves: 0,
            phase: Phase::Playing,
            undo: None,
            undo_armed: false,
            status: None,
        };
        game.deal_opening_tiles();
        game
    }

    /// 当前棋盘。
    pub fn board(&self) -> &Board {
        &self.board
    }

    /// 已获得的分数。
    pub fn score(&self) -> u64 {
        self.score
    }

    /// 已成功滑动的步数。
    pub fn moves(&self) -> u64 {
        self.moves
    }

    /// 当前阶段。
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// 此刻按 `Z` 是否有作用。
    pub fn undo_ready(&self) -> bool {
        self.undo_armed
    }

    /// 仍在显示的临时提示消息。
    pub fn status(&self) -> Option<(&str, StatusKind)> {
        self.status.as_ref().map(|status| (status.text.as_str(), status.kind))
    }

    /// 朝 `direction` 滑动一步。
    pub fn play(&mut self, direction: Direction) -> MoveOutcome {
        if self.phase == Phase::Over {
            self.set_status("游戏结束：Z 撤回上一步，R 重开，Q 退出", StatusKind::Warning);
            return MoveOutcome::Finished;
        }

        let before = self.snapshot();
        let outcome: SlideOutcome = self.board.slide(direction);
        if !outcome.moved {
            // 走不动的一步不算一步，撤回槽保持原样。
            return MoveOutcome::Blocked;
        }

        self.score = self.score.saturating_add(outcome.gained);
        self.moves += 1;
        // 只有真正走出一步才会重新装填撤回槽。
        self.undo = Some(before);
        self.undo_armed = true;

        self.spawn_tile();
        self.refresh_phase();
        MoveOutcome::Moved
    }

    /// 撤销上一步。
    ///
    /// 没有可撤回的步骤时返回 `false`；刚刚撤回过的下一步同样返回 `false`，
    /// 这就是“不能连续撤回”。
    pub fn undo(&mut self) -> bool {
        if !self.undo_armed {
            self.set_status("不能连续撤回：先走一步再按 Z", StatusKind::Warning);
            return false;
        }
        let Some(snapshot) = self.undo.take() else {
            // 不可达：`undo_armed` 与 `undo` 永远同时设置。
            self.set_status("没有可以撤回的步骤", StatusKind::Warning);
            return false;
        };

        self.board = snapshot.board;
        self.score = snapshot.score;
        self.moves = snapshot.moves;
        self.phase = snapshot.phase;
        self.undo_armed = false;
        self.set_status("已撤回上一步", StatusKind::Info);
        true
    }

    /// 重开一局。
    pub fn restart(&mut self) {
        self.board = Board::default();
        self.score = 0;
        self.moves = 0;
        self.phase = Phase::Playing;
        self.undo = None;
        self.undo_armed = false;
        self.deal_opening_tiles();
        self.set_status("新的一局", StatusKind::Info);
    }

    /// 清掉已经过期的提示消息；返回 `true` 表示需要重画。
    pub fn expire_status(&mut self, now: Instant) -> bool {
        match &self.status {
            Some(status) if now >= status.expires_at => {
                self.status = None;
                true
            }
            _ => false,
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot { board: self.board, score: self.score, moves: self.moves, phase: self.phase }
    }

    fn deal_opening_tiles(&mut self) {
        self.spawn_tile();
        self.spawn_tile();
    }

    /// 在所有空格里均匀随机地选一个，放上 `2`（90%）或 `4`（10%）。
    fn spawn_tile(&mut self) -> bool {
        let empty = self.board.empty_cells();
        let Some(&(row, col)) = self.rng.pick(&empty) else {
            return false;
        };
        let value = if self.rng.chance(9, 10) { 2 } else { 4 };
        self.board.set(row, col, value);
        true
    }

    fn refresh_phase(&mut self) {
        if !self.board.has_moves() {
            self.phase = Phase::Over;
            self.set_status("无处可走，游戏结束", StatusKind::Warning);
            return;
        }
        if self.phase == Phase::Playing && self.board.max_tile() >= WIN_TILE {
            self.phase = Phase::Won;
            self.set_status("达成 2048！可以继续冲更高分", StatusKind::Info);
        }
    }

    fn set_status(&mut self, text: impl Into<String>, kind: StatusKind) {
        self.status =
            Some(Status { text: text.into(), kind, expires_at: Instant::now() + STATUS_TTL });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRECTIONS: [Direction; 4] =
        [Direction::Up, Direction::Right, Direction::Down, Direction::Left];

    fn game() -> Game {
        Game::new(GameRng::from_seed([7; 32]))
    }

    /// 棋盘上可被观察到的全部状态。
    #[derive(Debug, PartialEq, Eq)]
    struct State {
        board: Board,
        score: u64,
        moves: u64,
    }

    impl Game {
        fn state(&self) -> State {
            State { board: *self.board(), score: self.score(), moves: self.moves() }
        }
    }

    /// 走一步必定能成功的方向（棋盘上还有空格，总有一个方向动得了）。
    fn play_one_step(game: &mut Game) -> Direction {
        for direction in DIRECTIONS {
            if game.play(direction) == MoveOutcome::Moved {
                return direction;
            }
        }
        panic!("棋盘上至少还有两个数字，不可能四个方向都走不动");
    }

    #[test]
    fn a_fresh_game_has_two_tiles_and_no_undo() {
        let game = game();
        let tiles: Vec<_> = (0..4)
            .flat_map(|row| (0..4).map(move |col| (row, col)))
            .map(|(row, col)| game.board().get(row, col))
            .filter(|value| *value != 0)
            .collect();

        assert_eq!(tiles.len(), 2);
        assert!(tiles.iter().all(|value| *value == 2 || *value == 4));
        assert!(!game.undo_ready());
        assert_eq!(game.phase(), Phase::Playing);
        assert_eq!(game.score(), 0);
        assert_eq!(game.moves(), 0);
    }

    #[test]
    fn sliding_invariants_hold_across_a_long_deterministic_game() {
        let mut game = game();
        let mut moved_steps = 0;

        for (index, direction) in DIRECTIONS.into_iter().cycle().take(400).enumerate() {
            let before = game.state();
            let outcome = game.play(direction);

            match outcome {
                MoveOutcome::Moved => {
                    moved_steps += 1;
                    assert_eq!(game.moves(), before.moves + 1, "第 {index} 步");
                    assert!(game.score() >= before.score, "第 {index} 步");
                    assert_ne!(game.board(), &before.board, "第 {index} 步");
                    assert!(game.undo_ready(), "第 {index} 步之后应可撤回");
                }
                MoveOutcome::Blocked => {
                    assert_eq!(game.state(), before, "第 {index} 步走不动就不该算一步");
                }
                MoveOutcome::Finished => {
                    assert_eq!(game.phase(), Phase::Over, "第 {index} 步");
                    assert!(!game.board().has_moves());
                }
            }
        }

        assert!(moved_steps > 0);
    }

    #[test]
    fn undo_restores_exactly_the_previous_step() {
        let mut game = game();
        let opening = game.state();
        play_one_step(&mut game);

        let after_move = game.state();
        assert_ne!(after_move, opening);
        assert!(game.undo_ready());

        assert!(game.undo());
        assert_eq!(game.state(), opening);

        // 撤回之后紧接着再按 Z 必须无效。
        assert!(!game.undo_ready());
        assert!(!game.undo());
        assert_eq!(game.state(), opening);

        // 再走一步就又能撤回了，而且撤回后回到的正是这一步之前。
        let before_second_move = game.state();
        play_one_step(&mut game);
        assert_ne!(game.state(), before_second_move);
        assert!(game.undo());
        assert_eq!(game.state(), before_second_move);
    }

    #[test]
    fn a_blocked_move_does_not_rearm_the_undo_slot() {
        let mut game = game();
        // 先把棋盘全部推到左边，此时再往左推必定走不动。
        let mut pushed = game.state();
        for _ in 0..4 {
            game.play(Direction::Left);
            if game.state() == pushed {
                break;
            }
            pushed = game.state();
        }
        assert_eq!(game.play(Direction::Left), MoveOutcome::Blocked);

        assert!(game.undo());
        assert!(!game.undo_ready());
        // 连续撤回被拒绝，棋盘保持不动。
        let snapshot = game.state();
        assert!(!game.undo());
        assert_eq!(game.state(), snapshot);
        assert_eq!(game.status().map(|(text, _)| text), Some("不能连续撤回：先走一步再按 Z"));
    }

    #[test]
    fn undo_on_a_fresh_game_is_rejected() {
        let mut game = game();
        assert!(!game.undo());
        assert_eq!(game.status().map(|(text, _)| text), Some("不能连续撤回：先走一步再按 Z"));
    }

    #[test]
    fn restart_clears_everything() {
        let mut game = game();
        play_one_step(&mut game);
        game.restart();

        assert_eq!(game.moves(), 0);
        assert_eq!(game.score(), 0);
        assert_eq!(game.phase(), Phase::Playing);
        assert!(!game.undo_ready());
        assert_eq!(game.board().empty_cells().len(), 14);
    }

    #[test]
    fn status_messages_expire() {
        let mut game = game();
        assert!(game.status().is_none());
        game.restart();
        assert!(game.status().is_some());

        let later = Instant::now() + STATUS_TTL + Duration::from_millis(1);
        assert!(game.expire_status(later));
        assert!(game.status().is_none());
        // 已经清掉之后再检查不会重复触发重画。
        assert!(!game.expire_status(later));
    }
}
