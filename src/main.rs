use std::{env, io::stdout};

use app::App;
use config::AppConfig;
use ratatui::{backend::CrosstermBackend, Terminal};
use sync::AppSync;

pub mod app;
pub mod client;
pub mod clip;
pub mod config;
pub mod macros;
pub mod results;
pub mod source;
pub mod sync;
pub mod theme;
pub mod util;
pub mod widget;

#[tokio::main()]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Try to reset terminal on panic
        let _ = util::term::reset_terminal();
        default_panic(info);
        std::process::exit(1);
    }));

    // TODO: Use real command line package
    let args: Vec<String> = env::args().collect();
    for arg in args {
        if arg == "--version" || arg == "-V" || arg == "-v" {
            println!("nyaa v{}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
    }
    util::term::setup_terminal()?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::default();
    let sync = AppSync {};

    app.run_app::<_, _, AppConfig, false>(&mut terminal, sync)
        .await?;

    util::term::reset_terminal()?;
    terminal.show_cursor()?;

    std::process::exit(0);
}
