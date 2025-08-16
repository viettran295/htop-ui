use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{prelude::*, widgets::*, DefaultTerminal};
use indexmap::IndexMap;
use std::{
    sync::{mpsc::{self, Receiver, Sender}}, 
    thread, 
    time::{Duration, Instant}
};

use crate::cmd::{list_all_processes, process};

pub struct App {
    exit: bool,
    items: IndexMap<u32, process::Process>,
    state: TableState,
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
            items: IndexMap::new(),
            state: TableState::default().with_selected(0),
            last_tick: Instant::now(),
            tx: tx,
            rx: rx
        }
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<(), std::io::Error> {
        let mut processes = Vec::new();
        list_all_processes(self.tx.clone());
        
        while ! self.exit {
            if let Ok(proc) = self.rx.try_recv(){
                processes = proc;
                process::Process::sort_most_consume_cpu(&mut processes);
                self.update_processes(processes);
            }
            terminal.draw(|frame| self.ui(frame))?;
            self.handle_keyboard_events()?;
            thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }
    
    fn handle_keyboard_events(&mut self) -> Result<(), std::io::Error> {
        let timeout = Self::TICK_RATE.saturating_sub(self.last_tick.elapsed());
        while event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => self.exit = true,
                        KeyCode::Char('j') | KeyCode::Down => self.next_row(),
                        KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
                        _ => {}
                    }
                    
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
    
    fn update_processes(&mut self, processes: Vec<process::Process>) {
        for proc in processes {
            self.items.insert(proc.pid, proc);
        }
    }
    
    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED);
        let header = ["PID", "Name", "User", "CPU %", "Memory %"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .height(1);

        let rows = self.items.iter().map(|(_pid, process)| {
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
        .row_highlight_style(selected_row_style)
        .block(Block::default().borders(Borders::ALL).title("Processes"));

        frame.render_stateful_widget(t, area, &mut self.state);
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
    
    fn next_row(&mut self) {
        let row = match self.state.selected() {
            Some(row) => {
                if row >= self.items.len() - 1 {
                    0
                } else {
                    row + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(row));
    }
    
    fn previous_row(&mut self) {
        let row = match self.state.selected() {
            Some(row) => {
                if row == 0 {
                    self.items.len() - 1
                } else {
                    row - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(row));
    }
}