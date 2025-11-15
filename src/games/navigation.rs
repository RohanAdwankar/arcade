use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Default, Clone, Copy)]
pub struct VimMotionState {
    count: Option<usize>,
    pending_g: bool,
}

impl VimMotionState {
    pub fn prefix(&self) -> Option<usize> {
        self.count
    }

    pub fn clear(&mut self) {
        self.count = None;
        self.pending_g = false;
    }

    pub fn handle_key(
        &mut self,
        key: &KeyEvent,
        cursor: &mut (usize, usize),
        width: usize,
        height: usize,
    ) -> bool {
        match key.code {
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                self.pending_g = false;
                let digit = ch.to_digit(10).unwrap() as usize;
                if digit == 0 && self.count.is_none() {
                    if width > 0 {
                        cursor.0 = 0;
                    }
                    self.count = None;
                } else {
                    let next = self
                        .count
                        .unwrap_or(0)
                        .saturating_mul(10)
                        .saturating_add(digit);
                    self.count = Some(next.min(999));
                }
                true
            }
            KeyCode::Char('g') => {
                if self.pending_g {
                    if height > 0 {
                        cursor.1 = 0;
                    }
                    self.count = None;
                    self.pending_g = false;
                } else {
                    self.pending_g = true;
                }
                true
            }
            KeyCode::Char('G') => {
                self.pending_g = false;
                self.count = None;
                if height > 0 {
                    cursor.1 = height - 1;
                }
                true
            }
            KeyCode::Char('$') => {
                self.consume_pending();
                if width > 0 {
                    cursor.0 = width - 1;
                }
                self.count = None;
                true
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.consume_pending();
                self.move_cursor(cursor, width, height, -1, 0);
                true
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.consume_pending();
                self.move_cursor(cursor, width, height, 1, 0);
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.consume_pending();
                self.move_cursor(cursor, width, height, 0, -1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.consume_pending();
                self.move_cursor(cursor, width, height, 0, 1);
                true
            }
            _ => {
                self.consume_pending();
                false
            }
        }
    }

    fn move_cursor(
        &mut self,
        cursor: &mut (usize, usize),
        width: usize,
        height: usize,
        dx: isize,
        dy: isize,
    ) {
        if width == 0 || height == 0 {
            self.count = None;
            return;
        }
        let steps = self.count.take().unwrap_or(1);
        for _ in 0..steps {
            let new_x =
                ((cursor.0 as isize + dx).clamp(0, width.saturating_sub(1) as isize)) as usize;
            let new_y =
                ((cursor.1 as isize + dy).clamp(0, height.saturating_sub(1) as isize)) as usize;
            if new_x == cursor.0 && new_y == cursor.1 {
                break;
            }
            cursor.0 = new_x;
            cursor.1 = new_y;
        }
    }

    fn consume_pending(&mut self) {
        self.pending_g = false;
    }
}
