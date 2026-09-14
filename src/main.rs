//! ez2048 —— 一个用 Rust 从零写起的异步终端 2048。
//!
//! 结构：
//! * [`board`]：纯粹的棋盘滑动与合并规则；
//! * [`game`]：分数、阶段与“只能撤回一步”的撤回逻辑；
//! * [`rng`]：基于 `rand` 的加密级随机数；
//! * [`input`]：把 `crossterm` 的阻塞按键事件桥接成异步消息；
//! * [`ui`]：`crossterm` 绘制；
//! * 本文件：进入备用屏，并用 `tokio::select!` 驱动主循环。

mod board;
mod game;
mod input;
mod rng;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use tokio::sync::mpsc;
use tokio::time::{MissedTickBehavior, interval};

use crate::game::Game;
use crate::input::Action;
use crate::rng::GameRng;
use crate::ui::{Area, Screen, Ui};

/// 主循环的唤醒间隔，用来让临时提示消息到点后自动消失。
const TICK: Duration = Duration::from_millis(80);

#[tokio::main]
async fn main() -> Result<()> {
    // 先建好游戏再切进备用屏，这样随机数初始化失败时错误信息还能正常打印出来。
    let mut game = Game::new(GameRng::from_os_entropy()?);
    let _terminal = Terminal::enter()?;

    let mut screen = Ui::new(io::stdout());
    let mut actions = input::spawn();

    run(&mut screen, &mut game, &mut actions).await
}

/// 事件主循环：要么处理一个按键，要么等定时器把过期的提示清掉。
async fn run<S: Screen>(
    screen: &mut S,
    game: &mut Game,
    actions: &mut mpsc::Receiver<Action>,
) -> Result<()> {
    let mut ticker = interval(TICK);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut dirty = true;
    loop {
        if dirty {
            screen.draw(game, Area::from_terminal()).context("绘制棋盘失败")?;
            dirty = false;
        }

        tokio::select! {
            // 按键优先于定时器，操作手感更跟手。
            biased;

            action = actions.recv() => match action {
                // `None` 说明输入线程已经收工，直接退出。
                Some(Action::Quit) | None => break,
                Some(Action::Move(direction)) => {
                    game.play(direction);
                    dirty = true;
                }
                Some(Action::Undo) => {
                    game.undo();
                    dirty = true;
                }
                Some(Action::Restart) => {
                    game.restart();
                    dirty = true;
                }
                Some(Action::Resize { .. }) => dirty = true,
            },

            _ = ticker.tick() => {
                if game.expire_status(Instant::now()) {
                    dirty = true;
                }
            }
        }
    }

    Ok(())
}

/// 进入原始模式与备用屏，并在离开作用域（包括 panic 展开）时恢复终端。
struct Terminal;

impl Terminal {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("无法切换到原始模式")?;

        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide) {
            // 备用屏没进去，但原始模式已经开了，得先还原再报错。
            let _ = disable_raw_mode();
            return Err(error).context("无法进入备用屏");
        }

        Ok(Self)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Direction;

    /// 不画终端、只记录每帧摘要的 [`Screen`]，用来观察主循环。
    #[derive(Default)]
    struct Recorder {
        frames: Vec<(u64, u64, bool)>,
    }

    impl Screen for Recorder {
        fn draw(&mut self, game: &Game, _area: Area) -> io::Result<()> {
            self.frames.push((game.score(), game.moves(), game.undo_ready()));
            Ok(())
        }
    }

    #[tokio::test]
    async fn the_loop_applies_keys_and_refuses_a_double_undo() {
        let mut game = Game::new(GameRng::from_seed([3; 32]));
        let mut screen = Recorder::default();
        let (sender, mut actions) = mpsc::channel(16);

        let script = [
            Action::Move(Direction::Left),
            Action::Move(Direction::Up),
            Action::Move(Direction::Right),
            Action::Move(Direction::Down),
            Action::Undo,
            // 紧接着的第二次撤回应被拒绝。
            Action::Undo,
            Action::Resize { cols: 100, rows: 40 },
            Action::Quit,
        ];
        for action in script {
            sender.send(action).await.expect("接收端还活着");
        }

        run(&mut screen, &mut game, &mut actions).await.expect("主循环不应出错");

        // 首帧 + 除退出外的 7 个动作各重画一次。
        assert_eq!(screen.frames.len(), script.len());
        assert_eq!(screen.frames[0], (0, 0, false), "开局应先画一帧空盘");

        // 四个方向里至少有一个能走，走过之后撤回槽就该就绪。
        assert!(screen.frames.iter().any(|frame| frame.2), "走过一步之后撤回槽必须就绪");
        assert!(screen.frames.iter().any(|frame| frame.1 >= 1), "四个方向里至少有一个能走");

        // 撤回一次之后紧接着再撤回会被拒绝，棋盘保持不动。
        assert!(!game.undo_ready());
        assert_eq!(game.status().map(|(text, _)| text), Some("不能连续撤回：先走一步再按 Z"));
    }
}
