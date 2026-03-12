pub mod app;
pub mod event;
pub mod pages;

use std::io;
use std::path::PathBuf;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::error::AppError;
use crate::service::task_service::TaskService;

use self::app::App;
use self::event::{AppEvent, EventHandler};

/// Run the TUI application.
pub fn run_tui(service: TaskService, tasks_path: PathBuf) -> Result<(), AppError> {
    // Set up panic hook to restore terminal before printing the panic message.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .map_err(|e| AppError::Io(e))?;

    let mut app = App::new(service, tasks_path);
    let event_handler = EventHandler::new(250);

    // Main loop
    loop {
        terminal.draw(|f| app.render(f))
            .map_err(|e| AppError::Io(e))?;

        match event_handler.next()? {
            AppEvent::Key(key_event) => {
                let result = app.handle_key(key_event);
                if result.should_quit {
                    break;
                }
            }
            AppEvent::Tick => {
                app.handle_tick();
            }
            AppEvent::Resize(_, _) => {
                // Terminal handles resize automatically
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()
        .map_err(|e| AppError::Io(e))?;

    Ok(())
}
