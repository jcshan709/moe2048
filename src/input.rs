//! 把 `crossterm` 的阻塞式终端事件桥接进异步世界。
//!
//! 终端读取本身是阻塞的，所以它被放到 `tokio` 的 blocking 线程池里运行，
//! 再通过 `mpsc` 通道把语义化的 [`Action`] 交给异步主循环。

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc;

use crate::board::Direction;

/// 每次阻塞等待的最长时间；用它来周期性检查主循环是否已经退出。
const POLL_INTERVAL: Duration = Duration::from_millis(40);

/// 一次用户意图，与具体终端事件解耦。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// 方向键：滑动棋盘。
    Move(Direction),
    /// `Z`：撤回上一步。
    Undo,
    /// `R`：重开一局。
    Restart,
    /// `Q`（或 `Esc`、`Ctrl+C`）：退出。
    Quit,
    /// 终端窗口尺寸变了，需要重画。
    Resize { cols: u16, rows: u16 },
}

/// 在 blocking 线程池上启动终端读取任务，返回接收端。
///
/// 接收端一旦被丢弃，读取任务会在一个 [`POLL_INTERVAL`] 之内自行结束，
/// 因此退出时不会把进程卡在阻塞读上。
pub fn spawn() -> mpsc::Receiver<Action> {
    let (sender, receiver) = mpsc::channel(64);

    tokio::task::spawn_blocking(move || {
        loop {
            if sender.is_closed() {
                break;
            }
            match event::poll(POLL_INTERVAL) {
                Ok(true) => {}
                // 超时：回去检查接收端是否还在。
                Ok(false) => continue,
                Err(_) => break,
            }
            let Ok(event) = event::read() else {
                break;
            };
            let Some(action) = translate(event) else {
                continue;
            };
            if sender.blocking_send(action).is_err() {
                break;
            }
        }
    });

    receiver
}

fn translate(event: Event) -> Option<Action> {
    match event {
        // Windows 会为按下、重复、抬起分别发事件，只处理“按下”和“重复”。
        Event::Key(key) if key.kind != KeyEventKind::Release => key_to_action(key),
        Event::Resize(cols, rows) => Some(Action::Resize { cols, rows }),
        _ => None,
    }
}

fn key_to_action(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Up => Some(Action::Move(Direction::Up)),
        KeyCode::Down => Some(Action::Move(Direction::Down)),
        KeyCode::Left => Some(Action::Move(Direction::Left)),
        KeyCode::Right => Some(Action::Move(Direction::Right)),
        KeyCode::Char('q' | 'Q') => Some(Action::Quit),
        KeyCode::Char('z' | 'Z') => Some(Action::Undo),
        KeyCode::Char('r' | 'R') => Some(Action::Restart),
        // 原始模式下 Ctrl+C 不再产生信号，需要自己识别。
        KeyCode::Char('c' | 'C') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::Quit)
        }
        KeyCode::Esc => Some(Action::Quit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    #[test]
    fn arrow_keys_become_moves() {
        for (code, direction) in [
            (KeyCode::Up, Direction::Up),
            (KeyCode::Down, Direction::Down),
            (KeyCode::Left, Direction::Left),
            (KeyCode::Right, Direction::Right),
        ] {
            assert_eq!(translate(press(code, KeyModifiers::NONE)), Some(Action::Move(direction)));
        }
    }

    #[test]
    fn letter_keys_are_case_insensitive() {
        for (letter, action) in [
            ('q', Action::Quit),
            ('Q', Action::Quit),
            ('z', Action::Undo),
            ('Z', Action::Undo),
            ('r', Action::Restart),
            ('R', Action::Restart),
        ] {
            assert_eq!(translate(press(KeyCode::Char(letter), KeyModifiers::NONE)), Some(action));
        }
    }

    #[test]
    fn key_releases_are_ignored() {
        let release = Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ));
        assert_eq!(translate(release), None);
    }

    #[test]
    fn ctrl_c_quits_and_resizes_are_forwarded() {
        assert_eq!(translate(press(KeyCode::Char('c'), KeyModifiers::CONTROL)), Some(Action::Quit));
        assert_eq!(translate(Event::Resize(100, 40)), Some(Action::Resize { cols: 100, rows: 40 }));
    }

    #[test]
    fn unrelated_events_are_dropped() {
        assert_eq!(translate(Event::FocusGained), None);
        assert_eq!(translate(press(KeyCode::F(1), KeyModifiers::NONE)), None);
    }
}
