use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{prelude::*, widgets::*, DefaultTerminal};
use std::{
    sync::{mpsc::{self, Receiver, Sender}}, 
    thread, 
    time::{Duration, Instant}
};

use crate::cmd::{list_all_processes, process};

pub struct App {
    exit: bool,
    items: Vec<process::Process>,    
    last_tick: Instant,
    tx: Sender<Vec<process::Process>>,
    rx: Receiver<Vec<process::Process>>
}

impl App {
    const TICK_RATE: Duration = Duration::from_millis(100);
    
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { 
            exit: false,
            items: Vec::new(),
            last_tick: Instant::now(),
            tx: tx,
            rx: rx
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<(), std::io::Error> {
        list_all_processes(self.tx.clone());
        while ! self.exit {
            if let Ok(sys) = self.rx.try_recv(){
                self.items = sys;
            }
            terminal.draw(|frame| self.ui(frame))?;
            self.handle_events()?;
            thread::sleep(Duration::from_secs(1));
        }
        Ok(())
    }
    
    fn handle_events(&mut self) -> Result<(), std::io::Error> {
        let timeout = Self::TICK_RATE.saturating_sub(self.last_tick.elapsed());
        while event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && 
                    key.code == KeyCode::Char('q') {
                    self.exit = true;
                }
            }
        }
        Ok(())
    }
    
    fn ui(&mut self, frame: &mut Frame) {
        let (process_area, cpu_area, ram_area) = Self::create_layout(frame);
        Self::render_widgets(frame, cpu_area, ram_area);
        self.render_table(frame, process_area);
    }
    
    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
            let header = ["PID", "Name", "User", "CPU %", "Memory %"]
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .height(1);

            let rows = self.items.iter().map(|process| {
                Row::new(vec![
                    Cell::from(process.pid.to_string()),
                    Cell::from(process.process_name.to_string()),
                    Cell::from(process.user.to_string()),
                    Cell::from(format!("{:.1}%", process.cpu_usage)),
                    Cell::from(format!("{:.1}%", process.mem_usage)),
                ])
            });

            let t = Table::new(
                rows,
                [
                    Constraint::Length(10),
                    Constraint::Min(20),
                    Constraint::Min(15),
                    Constraint::Length(10),
                    Constraint::Length(10),
                ],
            )
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Processes"));

            frame.render_widget(t, area);
        }
    
    fn render_widgets(
        frame: &mut Frame, 
        cpu_area: Rect,
        ram_area: Rect
    ) {
        frame.render_widget(
            Paragraph::new("")
                .block(Block::new()
                        .title("CPU")
                        .title_alignment(Alignment::Center)
                        .fg(Color::LightGreen)
                        .borders(Borders::all())), 
            cpu_area
        );
        frame.render_widget(
            Paragraph::new("")
                .block(Block::new()
                        .title("RAM")
                        .title_alignment(Alignment::Center)
                        .fg(Color::LightYellow)
                        .borders(Borders::all())), 
            ram_area
        );
    }
    
    fn create_layout(frame: &mut Frame) -> (Rect, Rect, Rect) {
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(70),
                Constraint::Percentage(30),
            ])
            .split(frame.area());
        let right_side = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(main_layout[1]);
        return (main_layout[0], right_side[0], right_side[1]);
    }
}