mod app;
mod cmd;

use log::Level;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    simple_logger::init_with_level(Level::Debug).unwrap();
    let terminal = ratatui::init();
    let result = app::App::new().run(terminal).await;
    ratatui::restore();
    result
}