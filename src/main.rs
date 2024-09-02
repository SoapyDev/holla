use crate::app::{App, CurrentScreen};
use crate::error::HollaError;
use crate::ui::ui;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::cursor::EnableBlinking;
use ratatui::crossterm::event::{poll, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::crossterm::{event, execute};
use ratatui::Terminal;
use std::io;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

mod app;
mod ui;
mod error;

#[tokio::main]
async fn main() -> Result<(), HollaError> {
    validate_ollama_is_installed()?;

    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(
        stderr,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBlinking
    )?;

    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()
        .await
        .with_viewport_height(terminal.size()?.height as usize);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    terminal.show_cursor()?;

    if let Ok(do_print) = res {
        if do_print {
            println!("The content of the conversation is saved at : $HOME/.hollama/history.json");
        } else if let Err(err) = res {
            println!("{}", err);
        }
    }

    Ok(())
}

fn validate_ollama_is_installed() -> Result<(), HollaError> {
    let current_user = std::env::var("USER").map_err(|_| HollaError::UserNotFound)?;

    if !std::path::Path::new(&format!("/home/{}/.ollama/", current_user)).exists() {
        return Err(HollaError::NotInstalled("Ollama could not be found at $HOME/.ollama"));
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<bool> {
    while app.current_screen != CurrentScreen::Exiting {
        terminal.draw(|f| ui(f, app))?;

        if !poll(Duration::from_millis(100))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }

            match app.current_screen {
                CurrentScreen::Home => handle_home_keys(app, key),
                CurrentScreen::Settings => handle_settings_keys(app, key),
                CurrentScreen::History => handle_history_keys(app, key),
                CurrentScreen::Exiting => {}
            }
        }
    }
    Ok(true)
}

fn handle_home_keys(app: &mut App, key: event::KeyEvent) {
    let is_waiting = Arc::clone(&app.is_waiting);
    if is_waiting.load(Ordering::Relaxed) {
        return;
    }
    match key.code {
        KeyCode::Backspace => app.remove_previous(),
        KeyCode::Delete => app.remove_next(),
        KeyCode::Tab => app.current_screen.next(),
        KeyCode::Enter => app.handle_enter(),
        KeyCode::Left => app.cursor_left(),
        KeyCode::Right => app.cursor_right(),
        KeyCode::Up => app.scroll_up(),
        KeyCode::Down => app.scroll_down(),
        KeyCode::Char(key) => app.insert_char(key),
        KeyCode::Esc => app.exit(),
        _ => {}
    }
}

fn handle_settings_keys(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Tab => app.current_screen.next(),
        KeyCode::Esc => app.exit(),
        KeyCode::Down => app.model_state.select_next(),
        KeyCode::Up => app.model_state.select_previous(),
        KeyCode::Enter => {}
        _ => {}
    }
}

fn handle_history_keys(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Tab => app.current_screen.next(),
        KeyCode::Esc => app.exit(),
        KeyCode::Down => {}
        KeyCode::Up => {}
        KeyCode::Enter => {}
        _ => {}
    }
}
