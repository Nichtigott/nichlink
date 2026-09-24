//! Box-drawing merge rules for the call-tree canvas.
//! 调用树画布的制表符合并规则。
//!
//! A terminal has no compositor: two edges that cross must end up as one glyph
//! that says "these lines meet here", not as whichever line was drawn last. So
//! every line glyph is read as a set of four arms (left, right, up, down), a new
//! line is merged by unioning the arm sets, and the result is looked up in a
//! sixteen-entry table. Written this way there is exactly one answer for every
//! crossing — `┼` for a cross, `├`/`┤`/`┬`/`┴` for a tee — and the table is small
//! enough to test exhaustively.
//! 终端没有合成器：两条交叉的边必须落成一个"线在这里相交"的字形，而不是看谁最后画。
//! 因此每个线字形都读成四个臂（左、右、上、下）的集合，新线通过并集合并，结果在十六项的
//! 表里查出。这样每种交叉只有一个答案——十字是 `┼`，丁字是 `├`/`┤`/`┬`/`┴`——而且表小到
//! 可以穷举测试。

/// Arm bit for the left side of a cell.
/// 单元格左侧的臂位。
const LEFT: u8 = 0b0001;
/// Arm bit for the right side of a cell.
/// 单元格右侧的臂位。
const RIGHT: u8 = 0b0010;
/// Arm bit for the top side of a cell.
/// 单元格上侧的臂位。
const UP: u8 = 0b0100;
/// Arm bit for the bottom side of a cell.
/// 单元格下侧的臂位。
const DOWN: u8 = 0b1000;

/// The glyph for each arm set, indexed by the mask: `L=1 R=2 U=4 D=8`.
/// 每个臂集合对应的字形，以掩码为下标：`L=1 R=2 U=4 D=8`。
///
/// A single arm is a stub that keeps the line it came from, because the arrow
/// heads are written separately and must stay visible; `0` is a blank cell.
/// 只有一条臂时是保持原线形状的残端，因为箭头单独写入、必须保持可见；`0` 是空白单元格。
const GLYPHS: [char; 16] = [
    ' ', // ....
    '─', // L...
    '─', // .R..
    '─', // LR..
    '│', // ..U.
    '╯', // L.U.
    '╰', // .RU.
    '┴', // LRU.
    '│', // ...D
    '╮', // L..D
    '╭', // .R.D
    '┬', // LR.D
    '│', // ..UD
    '┤', // L.UD
    '├', // .RUD
    '┼', // LRUD
];

/// The arms one line glyph reaches out with, or `None` for a glyph that is not a
/// line (text, a border corner of another widget, an arrow head).
/// 一个线字形伸出的臂；不是线字形（文字、别的控件的角、箭头）时为 `None`。
fn arms(glyph: char) -> Option<u8> {
    Some(match glyph {
        // A blank cell is an empty arm set, not a foreign glyph: every line
        // starts by being written into one.
        // 空白单元格是空臂集，而不是外来字形：每条线都从写进空单元格开始。
        ' ' => 0,
        '─' | '━' | '═' => LEFT | RIGHT,
        '│' | '┃' | '║' => UP | DOWN,
        // Double borders merge like single ones, so a port still lands on the
        // cursor's heavier box.
        // 双线边框与单线一样参与合并，因此端口同样能落在游标那个更重的盒子上。
        '╔' => RIGHT | DOWN,
        '╗' => LEFT | DOWN,
        '╚' => RIGHT | UP,
        '╝' => LEFT | UP,
        '╭' => RIGHT | DOWN,
        '╮' => LEFT | DOWN,
        '╰' => RIGHT | UP,
        '╯' => LEFT | UP,
        '├' => RIGHT | UP | DOWN,
        '┤' => LEFT | UP | DOWN,
        '┬' => LEFT | RIGHT | DOWN,
        '┴' => LEFT | RIGHT | UP,
        '┼' => LEFT | RIGHT | UP | DOWN,
        _ => return None,
    })
}

/// Merge a wanted line glyph into whatever is already in the cell.
/// 把想画的线字形合并进单元格里已有的内容。
///
/// A cell that holds text, an arrow head, or a corner of some other widget keeps
/// it: the canvas never overwrites something that is not a line.
/// 持有文字、箭头或别的控件边角的单元格保持原样：画布绝不覆盖不是线的东西。
pub(super) fn merged(existing: char, wanted: char) -> char {
    let (Some(existing_arms), Some(wanted_arms)) = (arms(existing), arms(wanted)) else {
        return existing;
    };
    GLYPHS[(existing_arms | wanted_arms) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Crossing two lines yields the junction glyph, not one of the two lines.
    /// 两条线交叉得到路口字形，而不是其中任一条线。
    #[test]
    fn crossings_resolve_to_junctions() {
        assert_eq!(merged('─', '│'), '┼');
        assert_eq!(merged('│', '─'), '┼');
        assert_eq!(merged('─', '─'), '─');
        assert_eq!(merged('│', '│'), '│');
    }

    /// A line that meets a box border leaves a port: the border keeps running up
    /// and down, the line arrives from the side, and the cell becomes a cross.
    /// 线接到盒子边框时留下端口：边框继续上下延伸、线从侧面进入，该单元格成为十字。
    #[test]
    fn meeting_a_border_leaves_a_port() {
        assert_eq!(merged('│', '─'), '┼');
        assert_eq!(merged('─', '│'), '┼');
    }

    /// Bends merge with the runs they join instead of erasing them.
    /// 转折与它连接的两段线合并，而不是擦掉它们。
    #[test]
    fn bends_join_the_runs_they_connect() {
        // A run from the left that turns up.
        // 从左来的横线向上转折。
        assert_eq!(merged('─', '╰'), '┴');
        // A run from the left that turns down.
        // 从左来的横线向下转折。
        assert_eq!(merged('─', '╮'), '┬');
        // A run coming down that turns right.
        // 从上来的竖线向右转折。
        assert_eq!(merged('│', '╰'), '├');
    }

    /// A blank cell takes the first line written into it.
    /// 空白单元格接受写进它的第一条线。
    #[test]
    fn a_blank_cell_accepts_a_line() {
        assert_eq!(arms(' '), Some(0));
        assert_eq!(merged(' ', '─'), '─');
        assert_eq!(merged(' ', '│'), '│');
        assert_eq!(merged(' ', '╰'), '╰');
    }

    /// Text, arrows and unknown glyphs are left alone.
    /// 文字、箭头与未知字形保持原样。
    #[test]
    fn non_line_cells_are_never_overwritten() {
        for existing in ['a', '▶', '→', '·', '╱'] {
            for wanted in ['─', '│', '╰'] {
                assert_eq!(
                    merged(existing, wanted),
                    existing,
                    "{existing} was overwritten by {wanted}"
                );
            }
        }
    }

    /// Every arm set with two or more arms reads back exactly; the five single-arm
    /// sets are stubs that keep the line they came from, because the arrow heads
    /// are written separately and must stay visible.
    /// 两个及以上臂的集合都能精确读回；五个单臂集合是保持原线形状的残端，因为箭头单独写入、
    /// 必须保持可见。
    #[test]
    fn multi_arm_sets_round_trip_and_single_arms_are_stubs() {
        assert_eq!(GLYPHS[0], ' ', "an empty arm set is a blank cell");
        assert_eq!(arms(' '), Some(0));
        for (subset, glyph) in GLYPHS.iter().enumerate().skip(1) {
            if subset.count_ones() >= 2 {
                assert_eq!(
                    arms(*glyph),
                    Some(subset as u8),
                    "arms {subset:04b} did not round-trip through {glyph}"
                );
            }
        }
        assert_eq!(GLYPHS[LEFT as usize], '─');
        assert_eq!(GLYPHS[RIGHT as usize], '─');
        assert_eq!(GLYPHS[UP as usize], '│');
        assert_eq!(GLYPHS[DOWN as usize], '│');
    }

    /// A requested glyph merged into an empty cell is that glyph.
    /// 想画的字形合并进空单元格就是该字形。
    #[test]
    fn a_first_line_keeps_its_shape() {
        for wanted in ['─', '│', '╭', '╮', '╰', '╯', '├', '┤', '┬', '┴', '┼']
        {
            assert_eq!(merged(' ', wanted), wanted);
        }
    }
}
