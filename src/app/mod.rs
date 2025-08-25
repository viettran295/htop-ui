mod config;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{prelude::*, style::palette::tailwind, widgets::{self, *}, DefaultTerminal};
use std::{
    mem, sync::mpsc::{self, Receiver, Sender}, time::{Duration, Instant}
};

use crate::{
    app::config::AppConfig,
    cmd::{list_all_processes, process, Message}
};

struct AppStyle {
    table_fg: Color,
    cpu_frame_fg: Color,
    mem_frame_fg: Color,
    selected_row: Color,
    exceed_threshold_cell: Color,
}

pub struct App {
    exit: bool,
    items: Vec<process::Process>,
    cores_usage: Vec<f32>,
    mem_usage: f32,
    state: TableState,
    style: AppStyle,
    blink_threshold: bool,
    config: AppConfig,
    last_tick: Instant,
    tx: Sender<Message>,
    rx: Receiver<Message>,
}

impl App {
    const CONFIG_PATH: &str = "./config_example.yaml";
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let app_style = AppStyle {
            table_fg: tailwind::LIME.c200,
            cpu_frame_fg: tailwind::YELLOW.c300,
            mem_frame_fg: tailwind::PURPLE.c300,
            selected_row: tailwind::ZINC.c100,
            exceed_threshold_cell: tailwind::PINK.c400,
        };
        let config = AppConfig::new(Self::CONFIG_PATH);
        Self { 
            exit: false,
            items: Vec::new(),
            cores_usage: Vec::new(),
            mem_usage: 0.0,
            state: TableState::default().with_selected(0),
            style: app_style,
            last_tick: Instant::now(),
            blink_threshold: false,
            config: config,
            tx: tx,
            rx: rx,
        }
    }

    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<(), std::io::Error> {
        let mut processes = Vec::new();
        list_all_processes(self.tx.clone());
        while ! self.exit {
            if let Ok(msg) = self.rx.try_recv(){
                match msg {
                    Message::Processes(proc) => {
                        processes = proc;
                        process::Process::sort_most_consume_cpu(&mut processes);
                        self.update_processes(processes);
                    }
                    Message::CPUUsage(cpu_usage) => {
                        self.cores_usage = cpu_usage;
                    }
                    Message::MEMUsage(mem_usage) => {
                        self.mem_usage = mem_usage;
                    }
                }
            }
            terminal.draw(|frame| self.ui(frame))?;
            self.handle_tick_threshold();
            self.handle_keyboard_events()?;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Ok(())
    }
    
    fn handle_tick_threshold(&mut self) {
        if self.last_tick.elapsed() >= self.config.blink_threshold_rate.unwrap()  {
            self.blink_threshold = ! self.blink_threshold;
            self.last_tick = Instant::now();
        }
    }
    
    fn handle_keyboard_events(&mut self) -> Result<(), std::io::Error> {
        let timeout = self.config.tick_rate.unwrap()
                                            .saturating_sub(self.last_tick.elapsed());
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
        let (process_area, cpu_area, mem_area) = Self::create_layout(frame);
        self.render_widgets(frame, cpu_area, mem_area);
        self.render_table(frame, process_area);
        self.render_cpu_usage(frame, cpu_area);
        self.render_mem_usage(frame, mem_area);
    }
    
    fn update_processes(&mut self, processes: Vec<process::Process>) {
        self.items.clear();
        for process in processes {
            if process.cpu_usage < 0.2 {
                continue;
            }
            self.items.push(process);
        }
        self.items.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
    }
    
    fn blink_cell(value: f32, threshold: f32, blink: bool, style: Color) -> Cell<'static> {
        let exceed_threshold_cell = Style::default()
            .add_modifier(Modifier::UNDERLINED)
            .fg(style);
        if value >= threshold && blink {
            return Cell::from(format!("{:.1}%", value)).style(exceed_threshold_cell)
        } else {
            return Cell::from(format!("{:.1}%", value))
        }
    }
    
    fn render_cpu_usage(&mut self, frame: &mut Frame, area: Rect) {
        let mut bars = Vec::new();
        let mut bar_color = self.style.cpu_frame_fg;
        let title = Line::from("CPU usage").centered();
        let block = Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(3))
            .title(title);
        for (idx, cores_usage) in self.cores_usage.iter().enumerate() {
            if *cores_usage > self.config.single_cpu_threshold.unwrap() {
                bar_color = self.style.exceed_threshold_cell;
            } 
            bars.push(
                Bar::default()
                    .value(*cores_usage as u64)
                    .label(Line::from(format!("#{idx}")))
                    .text_value(format!("{}%", *cores_usage as u64))
                    .style(bar_color)
            );
        }
        let bar_chart = BarChart::default()
            .block(block)
            .data(BarGroup::default().bars(&bars))
            .direction(Direction::Vertical)
            .bar_width(5)
            .bar_gap(4)
            .max(100);
        frame.render_widget(bar_chart, area);
    }
    
    fn render_mem_usage(&self, frame: &mut Frame, area: Rect) {
        let title = Line::from("Memory usage").centered();
        let block = Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(3))
            .title(title);
        let bar_style = Style::default()
            .fg(self.style.mem_frame_fg)
            .bg(Color::DarkGray);   
        let bar = vec![
            Bar::default()
                .value(self.mem_usage as u64)
                .value_style(Style::default().bg(self.style.mem_frame_fg))
                .label(Line::from(format!("{:.1}%", self.mem_usage)))
                .style(bar_style)
        ];
        let bar_chart = BarChart::default()
            .block(block)
            .data(BarGroup::default().bars(&bar))
            .direction(Direction::Horizontal)
            .bar_width(1)
            .max(100);
        frame.render_widget(bar_chart, area);
    }
    
    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.style.selected_row);
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
                Self::blink_cell(
                    process.cpu_usage, 
                    self.config.cpu_threshold.unwrap(), 
                    self.blink_threshold, 
                    self.style.exceed_threshold_cell
                ),
                Self::blink_cell(
                    process.mem_usage, 
                    self.config.mem_threshold.unwrap(),
                    self.blink_threshold, 
                    self.style.exceed_threshold_cell
                )
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
        .fg(self.style.table_fg)
        .row_highlight_style(selected_row_style)
        .highlight_spacing(HighlightSpacing::Always)
        .block(Block::default().borders(Borders::ALL).title("Processes"));

        frame.render_stateful_widget(t, area, &mut self.state);
    }
        
    fn render_widgets(
        &mut self,
        frame: &mut Frame, 
        cpu_area: Rect,
        ram_area: Rect
    ) {
        frame.render_widget(
            Paragraph::new("")
                .block(Block::new()
                        .title("CPU")
                        .title_alignment(Alignment::Center)
                        .fg(self.style.cpu_frame_fg)
                        .borders(Borders::all())), 
            cpu_area
        );
        frame.render_widget(
            Paragraph::new("")
                .block(Block::new()
                        .title("RAM")
                        .title_alignment(Alignment::Center)
                        .fg(self.style.mem_frame_fg)
                        .borders(Borders::all())), 
            ram_area
        );
    }
    
    fn create_layout(frame: &mut Frame) -> (Rect, Rect, Rect) {
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ])
            .split(frame.area());
        let right_side = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(20),
                Constraint::Percentage(10),
                Constraint::Percentage(60),
            ])
            .split(main_layout[1]);
        return (main_layout[0], right_side[0], right_side[1]);
    }
    
    fn next_row(&mut self) {
        let row = match self.state.selected() {
            Some(row) => {
                if row >= self.items.len() - 1 {
                    self.items.len() - 1
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
                    0
                } else {
                    row - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(row));
    }
}