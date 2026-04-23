use std::io::{self, Stdout, Write};

use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::shared::error::AppResult;

pub struct TerminalUi {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalUi {
    pub fn new() -> AppResult<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;
        Ok(Self { terminal })
    }

    pub fn draw<F>(&mut self, mut render: F) -> AppResult<()>
    where
        F: FnMut(&mut ratatui::Frame<'_>),
    {
        self.terminal.draw(|frame| render(frame))?;
        Ok(())
    }

    pub fn restore(&mut self) -> AppResult<()> {
        restore_terminal()?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    pub fn begin_pty_session(&mut self) -> AppResult<()> {
        self.terminal.show_cursor()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            Clear(ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;
        stdout.flush()?;
        Ok(())
    }

    pub fn end_pty_session(&mut self) -> AppResult<()> {
        self.terminal.hide_cursor()?;
        Ok(())
    }
}

impl Drop for TerminalUi {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

pub fn restore_terminal() -> AppResult<()> {
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen)?;
    Ok(())
}
