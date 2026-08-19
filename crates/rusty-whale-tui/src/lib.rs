//! rusty-whale-tui: terminal UI for rusty-whale.

use anyhow::Result;
use rusty_whale_core::AgentState;

/// Render the agent state to a ratatui frame. The skeleton just centers the
/// current state name; a real impl would render message history + tool calls.
pub fn render_state(frame: &mut ratatui::Frame, state: AgentState) {
    let area = frame.area();
    let label = format!("state: {:?}", state);
    let paragraph = ratatui::widgets::Paragraph::new(label)
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(paragraph, area);
}

/// Run the TUI event loop until the user quits.
pub async fn run() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut state = AgentState::Idle;
    loop {
        terminal.draw(|f| render_state(f, state))?;
        if let ratatui::crossterm::event::Event::Key(key) = ratatui::crossterm::event::read()? {
            if key.kind == ratatui::crossterm::event::KeyEventKind::Press
                && key.code == ratatui::crossterm::event::KeyCode::Char('q')
            {
                break;
            }
        }
    }
    ratatui::restore();
    Ok(())
}
