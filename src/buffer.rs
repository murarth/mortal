use std::mem::swap;
use std::ops::Range;

use smallstr::SmallString;

use crate::priv_util::is_visible;
use crate::terminal::{Color, Cursor, Size, Style, Theme};
use crate::util::{char_width, is_combining_mark};

const TAB_STOP: usize = 8;

pub struct ScreenBuffer {
    buffer: Vec<Cell>,
    back_buffer: Vec<Cell>,
    size: Size,
    cursor: Cursor,

    fg: Option<Color>,
    bg: Option<Color>,
    style: Style,
}

impl ScreenBuffer {
    pub fn new(size: Size) -> ScreenBuffer {
        let area = size.area();

        ScreenBuffer{
            buffer: vec![Cell::default(); area],
            back_buffer: vec![Cell::default(); area],
            size: size,
            cursor: Cursor::default(),

            fg: None,
            bg: None,
            style: Style::empty(),
        }
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn resize(&mut self, new_size: Size) {
        // Try our best to maintain the contents of the buffer;
        // though it's really best if users redraw when Resize event is read.
        resize_buffer(&mut self.buffer, self.size, new_size);
        // Totally invalidate the back buffer.
        // Screen implementations will clear the screen and redraw.
        new_buffer(&mut self.back_buffer, new_size);
        self.size = new_size;
    }

    pub fn set_cursor(&mut self, pos: Cursor) {
        self.cursor = pos;
    }

    pub fn next_line(&mut self, column: usize) {
        self.cursor.line += 1;
        self.cursor.column = column;
    }

    pub fn clear_attributes(&mut self) {
        self.fg = None;
        self.bg = None;
        self.style = Style::empty();
    }

    pub fn add_style(&mut self, style: Style) {
        self.style |= style;
    }

    pub fn remove_style(&mut self, style: Style) {
        self.style -= style;
    }

    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }

    pub fn set_fg(&mut self, fg: Option<Color>) {
        self.fg = fg;
    }

    pub fn set_bg(&mut self, bg: Option<Color>) {
        self.bg = bg;
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.set_fg(theme.fg);
        self.set_bg(theme.bg);
        self.set_style(theme.style);
    }

    pub fn clear_screen(&mut self) {
        for cell in &mut self.buffer {
            *cell = Cell::default();
        }
    }

    pub fn indices(&self) -> Range<usize> {
        0..self.size.area()
    }

    // A wrapper type implementing Iterator would be ideal, but that would
    // interefere with Screen implementations calling `&mut self` methods.
    pub fn next_cell(&mut self, indices: &mut Range<usize>) -> Option<(Cursor, Cell)> {
        while let Some(idx) = indices.next() {
            let first = self.buffer[idx].first_char();
            let width = char_width(first).unwrap_or(0);

            // Skip cells overlapped by wide characters
            if width == 2 {
                let _ = indices.next();
            }

            if self.buffer[idx] != self.back_buffer[idx] {
                let cell = self.buffer[idx].clone();

                let line = idx / self.size.columns;
                let column = idx % self.size.columns;

                self.back_buffer[idx] = cell.clone();

                return Some((Cursor{line, column}, cell));
            }
        }

        None
    }

    #[cfg(test)]
    fn cell(&self, pos: Cursor) -> &Cell {
        &self.buffer[pos.as_index(self.size)]
    }

    fn cell_mut(&mut self, pos: Cursor) -> &mut Cell {
        let size = self.size;
        &mut self.buffer[pos.as_index(size)]
    }

    fn set_cell(&mut self, pos: Cursor, ch: char) {
        let fg = self.fg;
        let bg = self.bg;
        let style = self.style;

        let cell = self.cell_mut(pos);

        cell.fg = fg;
        cell.bg = bg;
        cell.style = style;
        cell.text = ch.into();
    }

    pub fn write_char(&mut self, ch: char) -> Result<(), OutOfBounds> {
        if ch == '\t' {
            self.try_cursor()?;
            let rem = self.size.columns - self.cursor.column;
            let n = rem.min(TAB_STOP - (self.cursor.column % TAB_STOP));

            for _ in 0..n {
                self.write_char(' ')?;
            }
        } else if ch == '\r' {
            self.cursor.column = 0;
        } else if ch == '\n' {
            self.cursor.line += 1;
            self.cursor.column = 0;
        } else if is_combining_mark(ch) {
            if let Some(prev) = self.cursor.previous(self.size) {
                self.try_cursor_at(prev)?;
                self.cell_mut(prev).text.push(ch);
            }
        } else if is_visible(ch) {
            self.try_cursor()?;

            if let Some(prev) = self.cursor.previous(self.size) {
                let cell = self.cell_mut(prev);

                if cell.is_wide() {
                    *cell = Cell::default();
                }
            }

            let rem = self.size.columns - self.cursor.column;
            let width = char_width(ch).unwrap_or(0);

            // If insufficient space exists on the current line,
            // fill it with spaces and write the char on the next line.
            if rem < width {
                self.try_cursor()?;
                let mut pos = self.cursor;

                for _ in 0..rem {
                    self.set_cell(pos, ch);
                    pos.column += 1;
                }

                self.cursor.column = 0;
                self.cursor.line += 1;
            }

            self.try_cursor()?;

            let mut pos = self.cursor;
            self.set_cell(pos, ch);

            for _ in 1..width {
                pos.column += 1;
                self.set_cell(pos, ' ');
            }

            self.cursor.column += width;

            if self.cursor.column >= self.size.columns {
                self.cursor.line += 1;
                self.cursor.column = 0;
            }
        }

        Ok(())
    }

    pub fn write_str(&mut self, s: &str) -> Result<(), OutOfBounds> {
        for ch in s.chars() {
            self.write_char(ch)?;
        }

        Ok(())
    }

    pub fn write_at(&mut self, pos: Cursor, text: &str) -> Result<(), OutOfBounds> {
        self.try_cursor_at(pos)?;
        self.cursor = pos;

        self.write_str(text)
    }

    pub fn write_styled(&mut self,
            fg: Option<Color>, bg: Option<Color>, style: Style, text: &str)
            -> Result<(), OutOfBounds> {
        self.fg = fg;
        self.bg = bg;
        self.style = style;

        self.write_str(text)?;
        self.clear_attributes();

        Ok(())
    }

    pub fn write_styled_at(&mut self, pos: Cursor,
            fg: Option<Color>, bg: Option<Color>, style: Style, text: &str)
            -> Result<(), OutOfBounds> {
        self.try_cursor_at(pos)?;
        self.cursor = pos;

        self.write_styled(fg, bg, style, text)
    }

    fn try_cursor(&self) -> Result<(), OutOfBounds> {
        self.try_cursor_at(self.cursor)
    }

    fn try_cursor_at(&self, pos: Cursor) -> Result<(), OutOfBounds> {
        if pos.line >= self.size.lines || pos.column >= self.size.columns {
            Err(OutOfBounds(()))
        } else {
            Ok(())
        }
    }
}

// Generates buffer methods (to be invoked from within an impl block)
// forwarded to a buffer contained in self.
//
// All methods accept `&self`. Interior mutability is required.
macro_rules! forward_screen_buffer_methods {
    ( |$slf:ident| $field:expr ) => {
        pub fn size(&self) -> crate::terminal::Size {
            let $slf = self;
            $field.size()
        }

        pub fn cursor(&self) -> crate::terminal::Cursor {
            let $slf = self;
            $field.cursor()
        }

        pub fn set_cursor(&self, pos: crate::terminal::Cursor) {
            let $slf = self;
            $field.set_cursor(pos);
        }

        pub fn next_line(&self, column: usize) {
            let $slf = self;
            $field.next_line(column);
        }

        pub fn clear_screen(&self) {
            let $slf = self;
            $field.clear_screen();
        }

        pub fn clear_attributes(&self) {
            let $slf = self;
            $field.clear_attributes();
        }

        pub fn add_style(&self, style: crate::terminal::Style) {
            let $slf = self;
            $field.add_style(style);
        }

        pub fn remove_style(&self, style: crate::terminal::Style) {
            let $slf = self;
            $field.remove_style(style);
        }

        pub fn set_style(&self, style: crate::terminal::Style) {
            let $slf = self;
            $field.set_style(style);
        }

        pub fn set_fg(&self, fg: Option<crate::terminal::Color>) {
            let $slf = self;
            $field.set_fg(fg);
        }

        pub fn set_bg(&self, bg: Option<crate::terminal::Color>) {
            let $slf = self;
            $field.set_bg(bg);
        }

        pub fn set_theme(&self, theme: crate::terminal::Theme) {
            let $slf = self;
            $field.set_theme(theme)
        }

        pub fn write_char(&self, ch: char) {
            let $slf = self;
            let _ = $field.write_char(ch);
        }

        pub fn write_str(&self, s: &str) {
            let $slf = self;
            let _ = $field.write_str(s);
        }

        pub fn write_at(&self, pos: crate::terminal::Cursor, text: &str) {
            let $slf = self;
            let _ = $field.write_at(pos, text);
        }

        pub fn write_styled(&self,
                fg: Option<crate::terminal::Color>, bg: Option<crate::terminal::Color>,
                style: crate::terminal::Style, text: &str) {
            let $slf = self;
            let _ = $field.write_styled(fg, bg, style, text);
        }

        pub fn write_styled_at(&self, pos: crate::terminal::Cursor,
                fg: Option<crate::terminal::Color>, bg: Option<crate::terminal::Color>,
                style: crate::terminal::Style, text: &str) {
            let $slf = self;
            let _ = $field.write_styled_at(pos, fg, bg, style, text);
        }
    }
}

// Same as above, but methods take `&mut self` where appropriate.
macro_rules! forward_screen_buffer_mut_methods {
    ( |$slf:ident| $field:expr ) => {
        pub fn size(&self) -> crate::terminal::Size {
            let $slf = self;
            $field.size()
        }

        pub fn cursor(&self) -> crate::terminal::Cursor {
            let $slf = self;
            $field.cursor()
        }

        pub fn set_cursor(&mut self, pos: crate::terminal::Cursor) {
            let $slf = self;
            $field.set_cursor(pos);
        }

        pub fn next_line(&mut self, column: usize) {
            let $slf = self;
            $field.next_line(column);
        }

        pub fn clear_screen(&mut self) {
            let $slf = self;
            $field.clear_screen();
        }

        pub fn clear_attributes(&mut self) {
            let $slf = self;
            $field.clear_attributes();
        }

        pub fn add_style(&mut self, style: crate::terminal::Style) {
            let $slf = self;
            $field.add_style(style);
        }

        pub fn remove_style(&mut self, style: crate::terminal::Style) {
            let $slf = self;
            $field.remove_style(style);
        }

        pub fn set_style(&mut self, style: crate::terminal::Style) {
            let $slf = self;
            $field.set_style(style);
        }

        pub fn set_fg(&mut self, fg: Option<crate::terminal::Color>) {
            let $slf = self;
            $field.set_fg(fg);
        }

        pub fn set_bg(&mut self, bg: Option<crate::terminal::Color>) {
            let $slf = self;
            $field.set_bg(bg);
        }

        pub fn set_theme(&mut self, theme: crate::terminal::Theme) {
            let $slf = self;
            $field.set_theme(theme);
        }

        pub fn write_char(&mut self, ch: char) {
            let $slf = self;
            let _ = $field.write_char(ch);
        }

        pub fn write_str(&mut self, s: &str) {
            let $slf = self;
            let _ = $field.write_str(s);
        }

        pub fn write_at(&mut self, pos: crate::terminal::Cursor, text: &str) {
            let $slf = self;
            let _ = $field.write_at(pos, text);
        }

        pub fn write_styled(&mut self,
                fg: Option<crate::terminal::Color>, bg: Option<crate::terminal::Color>,
                style: crate::terminal::Style, text: &str) {
            let $slf = self;
            let _ = $field.write_styled(fg, bg, style, text);
        }

        pub fn write_styled_at(&mut self, pos: crate::terminal::Cursor,
                fg: Option<crate::terminal::Color>, bg: Option<crate::terminal::Color>,
                style: crate::terminal::Style, text: &str) {
            let $slf = self;
            let _ = $field.write_styled_at(pos, fg, bg, style, text);
        }
    }
}

#[derive(Debug)]
pub struct OutOfBounds(());

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cell {
    fg: Option<Color>,
    bg: Option<Color>,
    style: Style,
    text: SmallString<[u8; 8]>,
}

impl Cell {
    fn new(fg: Option<Color>, bg: Option<Color>, style: Style, chr: char) -> Cell {
        Cell{
            fg,
            bg,
            style,
            text: chr.into(),
        }
    }

    fn invalid() -> Cell {
        Cell{
            fg: None,
            bg: None,
            style: Style::empty(),
            text: SmallString::new(),
        }
    }

    pub fn attrs(&self) -> (Option<Color>, Option<Color>, Style) {
        (self.fg, self.bg, self.style)
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    fn first_char(&self) -> char {
        self.text.chars().next().expect("empty cell text")
    }

    fn is_wide(&self) -> bool {
        self.text.chars().next()
            .and_then(char_width).unwrap_or(0) == 2
    }
}

impl Default for Cell {
    fn default() -> Cell {
        Cell::new(None, None, Style::empty(), ' ')
    }
}

fn resize_buffer(buf: &mut Vec<Cell>, old: Size, new: Size) {
    if old != new {
        let mut new_buf = vec![Cell::default(); new.area()];

        if !buf.is_empty() {
            let n_cols = old.columns.min(new.columns);

            for (old, new) in buf.chunks_mut(old.columns)
                    .zip(new_buf.chunks_mut(new.columns)) {
                for i in 0..n_cols {
                    swap(&mut new[i], &mut old[i]);
                }
            }
        }

        *buf = new_buf;
    }
}

fn new_buffer(buf: &mut Vec<Cell>, new_size: Size) {
    // Invalidate the buffer; all cells will be redrawn
    *buf = vec![Cell::invalid(); new_size.area()];
}

#[cfg(test)]
mod test {
    use crate::terminal::{Cursor, Size};
    use crate::util::char_width;
    use super::ScreenBuffer;

    macro_rules! assert_lines {
        ( $buf:expr , $lines:expr ) => {
            assert_lines(&$buf, &$lines[..], line!())
        }
    }

    fn assert_lines(buf: &ScreenBuffer, lines: &[&str], line_num: u32) {
        let size = buf.size();
        let mut text = String::with_capacity(size.columns);

        assert_eq!(size.lines, lines.len(),
            "line count does not match at line {}", line_num);

        for line in 0..size.lines {
            let mut column = 0;

            while column < size.columns {
                let cell = buf.cell(Cursor{line, column});
                text.push_str(&cell.text);

                column += cell.text.chars().next()
                    .and_then(char_width).unwrap_or(1);
            }

            let next_line = lines[line];

            assert_eq!(text.trim_end(), next_line,
                "buffer line {} does not match at line {}", line, line_num);

            text.clear();
        }
    }

    #[test]
    fn test_buffer_bounds() {
        let mut buf = ScreenBuffer::new(Size{lines: 1, columns: 1});

        buf.write_char('a').unwrap();
        assert!(buf.write_char('b').is_err());
    }

    #[test]
    fn test_buffer_combining() {
        let mut buf = ScreenBuffer::new(Size{lines: 1, columns: 1});

        buf.write_str("a\u{301}\u{302}\u{303}\u{304}").unwrap();
        assert_lines!(buf, ["a\u{301}\u{302}\u{303}\u{304}"]);

        assert!(buf.write_str("x").is_err())
    }

    #[test]
    fn test_buffer_tab() {
        let mut buf = ScreenBuffer::new(Size{lines: 2, columns: 10});

        buf.write_str("xxxxxxxxxx").unwrap();
        assert_lines!(buf, ["xxxxxxxxxx", ""]);

        buf.set_cursor((0, 0).into());
        buf.write_str("\tyyz").unwrap();
        assert_lines!(buf, ["        yy", "z"]);

        buf.set_cursor((0, 0).into());
        buf.write_str("\tx\tx").unwrap();
        assert_lines!(buf, ["        x", "x"]);
    }

    #[test]
    fn test_buffer_wide() {
        let mut buf = ScreenBuffer::new(Size{lines: 1, columns: 10});

        buf.write_str("Ｆｏｏ").unwrap();
        assert_lines!(buf, ["Ｆｏｏ"]);

        buf.write_str("\rx").unwrap();
        assert_lines!(buf, ["x ｏｏ"]);

        buf.write_str("xx").unwrap();
        assert_lines!(buf, ["xxx ｏ"]);
    }
}
