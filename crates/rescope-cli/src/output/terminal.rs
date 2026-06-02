use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};

pub fn clear_screen() -> Result<()> {
    let mut stdout = std::io::stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    Ok(())
}
