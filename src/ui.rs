//! 终端绘制层。
//!
//! 整帧先在内存里拼好，再放进一次 `crossterm` 的“同步更新”里写出：终端会把这一批
//! 输出当成一次原子刷新，因此不会出现画到一半的画面。

use std::io::{self, Write};

use crossterm::SynchronizedUpdate;
use crossterm::cursor::MoveTo;
use crossterm::queue;
use crossterm::style::{Color, Stylize};
use crossterm::terminal::{self, Clear, ClearType};
use unicode_width::UnicodeWidthStr;

use crate::board::{Board, SIZE};
use crate::game::{Game, Phase, StatusKind};

/// 每个格子占的列数。
const CELL_WIDTH: usize = 7;
/// 每个格子占的行数。
const CELL_HEIGHT: usize = 3;
/// 棋盘外框的可见宽度。
const BOARD_WIDTH: usize = SIZE * CELL_WIDTH + SIZE + 1;
/// 标题行按固定宽度排版，这样分数位数变化时整帧不会左右跳动。
const HEADER_WIDTH: usize = 43;
/// 内容区至少需要的行数。
const MIN_ROWS: usize = 22;
/// 内容区至少需要的列数。
const MIN_COLS: usize = HEADER_WIDTH + 4;

/// 棋盘必须比标题行窄，否则居中基准就失去意义了。
const _: () = assert!(BOARD_WIDTH <= HEADER_WIDTH);

/// 常驻的按键提示。
const HINT: &str = "方向键 移动 · Z 撤回 · R 重开 · Q 退出";

const GRID_COLOR: Color = Color::Rgb { r: 0x9c, g: 0x8e, b: 0x82 };
const EMPTY_COLOR: Color = Color::Rgb { r: 0xcd, g: 0xc1, b: 0xb4 };
const TITLE_COLOR: Color = Color::Rgb { r: 0xed, g: 0xc2, b: 0x2e };
const HINT_COLOR: Color = Color::DarkGrey;
const INFO_COLOR: Color = Color::Rgb { r: 0x7f, g: 0xb0, b: 0x69 };
const WARNING_COLOR: Color = Color::Rgb { r: 0xe0, g: 0x7a, b: 0x5f };

const LIGHT_TEXT: Color = Color::Rgb { r: 0xf9, g: 0xf6, b: 0xf2 };
const DARK_TEXT: Color = Color::Rgb { r: 0x77, g: 0x6e, b: 0x65 };

/// 终端可用区域（列 x 行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub cols: u16,
    pub rows: u16,
}

impl Area {
    /// 读取当前终端尺寸；读不到就退回常见的 80x24。
    pub fn from_terminal() -> Self {
        let (cols, rows) = terminal::size().unwrap_or((80, 24));
        Self { cols, rows }
    }
}

/// 能把一局游戏画出来的东西。
///
/// 正式实现是 [`Ui`]；主循环只依赖这个接口，所以可以在没有终端的环境里测。
pub trait Screen {
    /// 画当前这一帧。
    fn draw(&mut self, game: &Game, area: Area) -> io::Result<()>;
}

/// 绑定标准输出的渲染器。
pub struct Ui {
    output: io::Stdout,
}

impl Ui {
    /// 绑定到给定的标准输出句柄。
    pub fn new(output: io::Stdout) -> Self {
        Self { output }
    }
}

impl Screen for Ui {
    fn draw(&mut self, game: &Game, area: Area) -> io::Result<()> {
        let frame = render(game, area.cols, area.rows);

        let mut output = self.output.lock();
        // 外层 `?` 是同步更新本身的开销，内层 `?` 是闭包里写终端的结果。
        output.sync_update(|writer| -> io::Result<()> {
            queue!(writer, MoveTo(0, 0))?;
            for line in &frame {
                // 先擦掉这一行的旧内容，避免上一帧更长的行留下尾巴。
                queue!(writer, Clear(ClearType::UntilNewLine))?;
                writer.write_all(line.as_bytes())?;
                writer.write_all(b"\r\n")?;
            }
            // 上一帧比这一帧高时，清掉下面多出来的行。
            queue!(writer, Clear(ClearType::FromCursorDown))
        })??;
        output.flush()
    }
}

/// 按终端尺寸拼出整帧的每一行（不含换行符）。
///
/// 纯函数，不碰终端，因此可以直接在测试里断言版面。
fn render(game: &Game, cols: u16, rows: u16) -> Vec<String> {
    let cols = usize::from(cols);
    let rows = usize::from(rows);

    if cols < MIN_COLS || rows < MIN_ROWS {
        let notice =
            format!("终端太小：至少需要 {MIN_COLS} 列 x {MIN_ROWS} 行，当前只有 {cols} x {rows}");
        return compose(vec![Line::single(&notice, Style::foreground(WARNING_COLOR))], cols, rows);
    }

    let mut lines = Vec::with_capacity(MIN_ROWS);
    lines.push(header(game));
    lines.push(Line::default());
    lines.push(border(Border::Top));
    for row in 0..SIZE {
        for band in 0..CELL_HEIGHT {
            lines.push(tile_band(game.board(), row, band));
        }
        lines.push(border(if row + 1 == SIZE { Border::Bottom } else { Border::Middle }));
    }
    lines.push(Line::default());
    lines.push(Line::single(HINT, Style::foreground(HINT_COLOR)));
    lines.push(status(game));
    compose(lines, cols, rows)
}

/// 把各行居中放进终端，返回每行的最终文本（不含换行符）。
fn compose(lines: Vec<Line>, cols: usize, rows: usize) -> Vec<String> {
    // 以固定宽度为基准排版，这样提示文字的长短不会带动整块内容左右移动。
    let frame_width = lines.iter().map(|line| line.width).max().unwrap_or(0).max(HEADER_WIDTH);
    let margin_left = cols.saturating_sub(frame_width) / 2;
    let margin_top = rows.saturating_sub(lines.len()) / 2;

    let mut frame = Vec::with_capacity(rows.max(lines.len()));
    frame.extend(std::iter::repeat_n(String::new(), margin_top));
    for line in lines {
        let indent = margin_left + (frame_width - line.width) / 2;
        frame.push(format!("{}{}", " ".repeat(indent), line.text));
    }
    frame
}

/// 标题行：游戏名、分数、步数、撤回槽状态。
fn header(game: &Game) -> Line {
    let (undo, undo_style) = if game.undo_ready() {
        ("就绪", Style::foreground(INFO_COLOR))
    } else {
        ("不可用", Style::foreground(HINT_COLOR))
    };

    let mut line = Line::default();
    line.push("2048", Style::foreground(TITLE_COLOR).emphasized());
    line.push("   分数 ", Style::default());
    line.push(&fit(&game.score().to_string(), 6), Style::default());
    line.push("  步数 ", Style::default());
    line.push(&fit(&game.moves().to_string(), 5), Style::default());
    line.push("  撤回 ", Style::default());
    line.push(&fit(undo, 6), undo_style);
    line
}

/// 棋盘外框的三种横向边线。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Border {
    Top,
    Middle,
    Bottom,
}

fn border(kind: Border) -> Line {
    let (left, junction, right) = match kind {
        Border::Top => ('╭', '┬', '╮'),
        Border::Middle => ('├', '┼', '┤'),
        Border::Bottom => ('╰', '┴', '╯'),
    };
    let grid = Style::foreground(GRID_COLOR);

    let mut line = Line::default();
    line.push(&left.to_string(), grid);
    for column in 0..SIZE {
        line.push(&"─".repeat(CELL_WIDTH), grid);
        if column + 1 < SIZE {
            line.push(&junction.to_string(), grid);
        }
    }
    line.push(&right.to_string(), grid);
    line
}

/// 棋盘第 `row` 行、第 `band` 条格子带（每格 3 行，数字画在中间那条）。
fn tile_band(board: &Board, row: usize, band: usize) -> Line {
    let grid = Style::foreground(GRID_COLOR);

    let mut line = Line::default();
    line.push("│", grid);
    for column in 0..SIZE {
        let value = board.get(row, column);
        let (foreground, background) = tile_colors(value);
        let style = Style::tile(foreground, background);

        // 数字只画在中间那条带上，上下两条留白但保持同一底色。
        let (content, style) = match value {
            0 => (" ".repeat(CELL_WIDTH), style),
            _ if band == 1 => (center(&value.to_string(), CELL_WIDTH), style.emphasized()),
            _ => (" ".repeat(CELL_WIDTH), style),
        };

        line.push(&content, style);
        line.push("│", grid);
    }
    line
}

/// 撤回状态与临时提示。
fn status(game: &Game) -> Line {
    if let Some((text, kind)) = game.status() {
        let color = match kind {
            StatusKind::Info => INFO_COLOR,
            StatusKind::Warning => WARNING_COLOR,
        };
        return Line::single(text, Style::foreground(color));
    }

    match game.phase() {
        Phase::Won => Line::single("达成 2048！可以继续冲更高分", Style::foreground(TITLE_COLOR)),
        Phase::Over => {
            Line::single("无处可走：Z 撤回上一步，R 重开，Q 退出", Style::foreground(WARNING_COLOR))
        }
        Phase::Playing => Line::default(),
    }
}

/// 每个数字对应的前景色与底色。
fn tile_colors(value: u32) -> (Color, Color) {
    let rgb = |r, g, b| Color::Rgb { r, g, b };
    match value {
        0 => (DARK_TEXT, EMPTY_COLOR),
        2 => (DARK_TEXT, rgb(0xee, 0xe4, 0xda)),
        4 => (DARK_TEXT, rgb(0xed, 0xe0, 0xc8)),
        8 => (LIGHT_TEXT, rgb(0xf2, 0xb1, 0x79)),
        16 => (LIGHT_TEXT, rgb(0xf5, 0x95, 0x63)),
        32 => (LIGHT_TEXT, rgb(0xf6, 0x7c, 0x5f)),
        64 => (LIGHT_TEXT, rgb(0xf6, 0x5e, 0x3b)),
        128 => (LIGHT_TEXT, rgb(0xed, 0xcf, 0x72)),
        256 => (LIGHT_TEXT, rgb(0xed, 0xcc, 0x61)),
        512 => (LIGHT_TEXT, rgb(0xed, 0xc8, 0x50)),
        1024 => (LIGHT_TEXT, rgb(0xed, 0xc5, 0x3f)),
        2048 => (LIGHT_TEXT, rgb(0xed, 0xc2, 0x2e)),
        _ => (LIGHT_TEXT, rgb(0x3c, 0x3a, 0x32)),
    }
}

/// 一段文本的样式。
#[derive(Debug, Clone, Copy, Default)]
struct Style {
    foreground: Option<Color>,
    background: Option<Color>,
    bold: bool,
}

impl Style {
    const fn foreground(foreground: Color) -> Self {
        Self { foreground: Some(foreground), background: None, bold: false }
    }

    const fn tile(foreground: Color, background: Color) -> Self {
        Self { foreground: Some(foreground), background: Some(background), bold: false }
    }

    const fn emphasized(self) -> Self {
        Self { foreground: self.foreground, background: self.background, bold: true }
    }

    const fn is_plain(self) -> bool {
        self.foreground.is_none() && self.background.is_none() && !self.bold
    }
}

/// 一行输出，同时记录它占用的终端列数。
#[derive(Default)]
struct Line {
    text: String,
    width: usize,
}

impl Line {
    /// 追加一段文本；样式为空时直接写原文，不产生多余的转义序列。
    fn push(&mut self, content: &str, style: Style) -> &mut Self {
        if style.is_plain() {
            self.text.push_str(content);
        } else {
            let mut styled = content.stylize();
            if let Some(foreground) = style.foreground {
                styled = styled.with(foreground);
            }
            if let Some(background) = style.background {
                styled = styled.on(background);
            }
            if style.bold {
                styled = styled.bold();
            }
            self.text.push_str(&styled.to_string());
        }
        self.width += content.width();
        self
    }

    /// 只有一段文本的一行。
    fn single(content: &str, style: Style) -> Self {
        let mut line = Self::default();
        line.push(content, style);
        line
    }
}

/// 右侧补空格到 `width` 列；已经够宽就原样返回。
fn fit(content: &str, width: usize) -> String {
    let padding = width.saturating_sub(content.width());
    format!("{content}{}", " ".repeat(padding))
}

/// 把 `label` 居中放进 `width` 列。
fn center(label: &str, width: usize) -> String {
    let label_width = label.width();
    if label_width >= width {
        return label.to_owned();
    }
    let padding = width - label_width;
    let left = padding / 2;
    format!("{}{label}{}", " ".repeat(left), " ".repeat(padding - left))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Direction;
    use crate::rng::GameRng;

    fn game() -> Game {
        Game::new(GameRng::from_seed([11; 32]))
    }

    /// 去掉 ANSI 转义序列，方便按可见文本断言。
    fn plain(frame: &[String]) -> Vec<String> {
        frame
            .iter()
            .map(|line| {
                let mut out = String::new();
                let mut chars = line.chars();
                while let Some(ch) = chars.next() {
                    if ch != '\u{1b}' {
                        out.push(ch);
                        continue;
                    }
                    // 吃掉 CSI 序列，直到遇到终止字母。
                    for code in chars.by_ref() {
                        if code.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                out
            })
            .collect()
    }

    /// 非空行即真正的内容行（其余是垂直居中留下的空行）。
    fn content_lines(frame: &[String]) -> Vec<String> {
        plain(frame).into_iter().filter(|line| !line.is_empty()).collect()
    }

    #[test]
    fn a_frame_fits_the_terminal_with_a_fixed_layout() {
        let frame = render(&game(), 80, 30);
        let content = content_lines(&frame);

        assert_eq!(content.len(), MIN_ROWS);
        assert!(frame.len() <= 30);
        assert!(content.iter().all(|line| line.width() <= 80), "所有行都必须塞得下");

        // 外框与格子带使用同一个左缩进，所以棋盘不会画歪。
        let leading = |line: &str| line.width() - line.trim_start().width();
        let border = content.iter().find(|line| line.contains('╭')).unwrap();
        let band = content.iter().find(|line| line.contains('│')).unwrap();
        assert_eq!(leading(border), leading(band));
        assert!(leading(border) > 0, "80 列时左右都应该留白");
    }

    #[test]
    fn the_board_frame_is_drawn_with_four_tile_rows() {
        let joined = content_lines(&render(&game(), 80, 30)).join("\n");
        assert_eq!(joined.matches('╭').count(), 1);
        assert_eq!(joined.matches('╰').count(), 1);
        assert_eq!(joined.matches('├').count(), SIZE - 1);
        // 每个格子带都画了左右两条竖线。
        assert_eq!(joined.matches('│').count(), SIZE * CELL_HEIGHT * (SIZE + 1));
    }

    #[test]
    fn a_tiny_terminal_gets_a_notice_instead_of_a_board() {
        let notice = content_lines(&render(&game(), 20, 5));
        assert_eq!(notice.len(), 1);
        assert!(notice[0].contains("终端太小"));
        assert!(!notice[0].contains('│'));
    }

    #[test]
    fn the_header_reports_score_moves_and_the_undo_slot() {
        let mut game = game();
        let before = plain(&render(&game, 80, 30)).join("\n");
        assert!(before.contains("分数 0"));
        assert!(before.contains("步数 0"));
        assert!(before.contains("撤回 不可用"));

        for direction in [Direction::Left, Direction::Up, Direction::Right, Direction::Down] {
            if game.play(direction) == crate::game::MoveOutcome::Moved {
                break;
            }
        }

        let after = plain(&render(&game, 80, 30)).join("\n");
        assert!(after.contains("步数 1"));
        assert!(after.contains("撤回 就绪"));
    }

    #[test]
    fn every_line_pads_to_the_same_visual_width() {
        // 中文标签是双宽度字符，用固定宽度排版才能保证不跳动。
        let line = header(&game());
        assert_eq!(line.width, HEADER_WIDTH);
    }

    #[test]
    fn fit_and_center_respect_display_width() {
        assert_eq!(fit("撤回", 6).width(), 6);
        assert_eq!(fit("不可用", 6).width(), 6);
        assert_eq!(center("2", 7), "   2   ");
        assert_eq!(center("2048", 7), " 2048  ");
        assert_eq!(center("", 7), "       ");
    }
}
