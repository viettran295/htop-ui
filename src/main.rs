mod app;
mod cmd;

fn main() -> Result<(), std::io::Error> {
    let terminal = ratatui::init();
    let result = app::App::new().run(terminal);
    ratatui::restore();
    result
}