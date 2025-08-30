mod config;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{prelude::*, style::palette::tailwind, widgets::*, DefaultTerminal};
use std::{
    sync::mpsc::{self, Receiver, Sender}, time::{Duration, Instant}
};

use crate::{
    app::config::AppConfig,
    cmd::{disk::Disk, get_disk_usage, get_network_info, list_all_processes, network::Network, process, Message}
};

struct AppStyle {
    table_fg: Color,
    cpu_frame_fg: Color,
    mem_frame_fg: Color,
    disk_frame_fg: Color,
    net_frame_fg: Color,
    selected_row: Color,
    exceed_threshold_cell: Color,
}

pub struct App {
    exit: bool,
    processes: Vec<process::Process>,
    network: Network,
    cores_usage: Vec<f32>,
    mem_usage: f32,
    disks_usage: Vec<Disk>,
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
            disk_frame_fg: tailwind::INDIGO.c300,
            net_frame_fg: tailwind::GREEN.c300,
            selected_row: tailwind::ZINC.c100,
            exceed_threshold_cell: tailwind::PINK.c400,
        };
        let config = AppConfig::new(Self::CONFIG_PATH);
        Self { 
            exit: false,
            processes: Vec::new(),
            network: Network::new(),
            cores_usage: Vec::new(),
            mem_usage: 0.0,
            disks_usage: Vec::new(),
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
        list_all_processes(self.tx.clone());
        get_network_info(self.tx.clone());
        get_disk_usage(self.tx.clone());
        while ! self.exit {
            if let Ok(msg) = self.rx.try_recv(){
                match msg {
                    Message::Processes(proc) => {
                        let mut processes = proc;
                        process::Process::sort_most_consume_cpu(&mut processes);
                        self.update_processes(processes);
                    }
                    Message::CPUUsage(cpu_usage) => {
                        self.cores_usage = cpu_usage;
                    }
                    Message::MEMUsage(mem_usage) => {
                        self.mem_usage = mem_usage;
                    }
                    Message::Network(net_data) => {
                        self.network.update(net_data.upload, net_data.download);
                    }
                    Message::DiskUsage(disk_data) => {
                        self.disks_usage = disk_data;
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
        let (process_area, cpu_area, network_area, mem_area, disk_area) = Self::create_layout(frame);
        self.render_widgets(frame, cpu_area, mem_area, network_area, disk_area);
        self.render_table(frame, process_area);
        self.render_cpu_usage(frame, cpu_area);
        self.render_mem_usage(frame, mem_area);
        self.render_network(frame, network_area);
        self.render_disks_usage(frame, disk_area);
    }
    
    fn update_processes(&mut self, processes: Vec<process::Process>) {
        self.processes.clear();
        for process in processes {
            if process.cpu_usage < 0.2 {
                continue;
            }
            self.processes.push(process);
        }
        self.processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
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
    
    fn render_disks_usage(&self, frame: &mut Frame, area: Rect) {
        let title = Line::from("Disk usage").centered();
        let block = Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(3))
            .title(title);
        let bar_style = Style::default()
            .fg(self.style.disk_frame_fg)
            .bg(Color::DarkGray);
        let text_style = Style::default()
            .fg(tailwind::BLACK)
            .bg(self.style.disk_frame_fg);
        let mut bars: Vec<Bar> = Vec::new();
        for disk in self.disks_usage.iter() {
            let total_space_gb = disk.total_space / 1_000_000_000;
            bars.push(
                Bar::default()
                    .value(disk.percent_used_space()as u64)
                    .value_style(Style::default().bg(self.style.mem_frame_fg))
                    .text_value(format!("{}% of {}GB", disk.percent_used_space(), total_space_gb))
                    .value_style(text_style)
                    .label(Line::from(format!("{:?}", disk.name)))
                    .style(bar_style)
            );
        }
        let bar_chart = BarChart::default()
            .block(block)
            .data(BarGroup::default().bars(&bars))
            .direction(Direction::Horizontal)
            .bar_width(1)
            .max(100);
        frame.render_widget(bar_chart, area);
    }
    
    fn render_network(&mut self, frame: &mut Frame, area: Rect) {
        let title = Line::from("Network").centered();
        let block = Block::new()
            .borders(Borders::ALL)
            .padding(Padding::horizontal(3))
            .title(title);
        let bar_style = Style::default()
            .fg(self.style.net_frame_fg)
            .bg(Color::DarkGray);   
        let bar = vec![
            Bar::default()
                .value(self.network.upload as u64)
                .value_style(Style::default().bg(self.style.net_frame_fg))
                .label(Line::from(format!("Upload {:.1} Kbps", self.network.upload)))
                .style(bar_style),
            Bar::default()
                .value(self.network.download as u64)
                .value_style(Style::default().bg(self.style.net_frame_fg))
                .label(Line::from(format!("Download {:.1} Kbps", self.network.download)))
                .style(bar_style)
        ];
        let bar_chart = BarChart::default()
            .block(block)
            .data(BarGroup::default().bars(&bar))
            .direction(Direction::Horizontal)
            .bar_width(1)
            .max(200);
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
        
        let rows = self.processes.iter().map(|process| {
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
        ram_area: Rect,
        net_area: Rect,
        disk_area: Rect
    ) {
        frame.render_widget(
            Paragraph::new("")
                .block(Block::new()
                        .title_alignment(Alignment::Center)
                        .fg(self.style.cpu_frame_fg)
                        .borders(Borders::all())), 
            cpu_area
        );
        frame.render_widget(
            Paragraph::new("")
                .block(Block::new()
                        .title_alignment(Alignment::Center)
                        .fg(self.style.net_frame_fg)
                        .borders(Borders::all())), 
            net_area
        );
        frame.render_widget(
            Paragraph::new("")
                .block(Block::new()
                        .title_alignment(Alignment::Center)
                        .fg(self.style.mem_frame_fg)
                        .borders(Borders::all())), 
            ram_area
        );
        frame.render_widget(
            Paragraph::new("")
                .block(Block::new()
                        .title_alignment(Alignment::Center)
                        .fg(self.style.disk_frame_fg)
                        .borders(Borders::all())), 
            disk_area
        );
    }
    
    fn create_layout(frame: &mut Frame) -> (Rect, Rect, Rect, Rect, Rect) {
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
                Constraint::Percentage(15),
                Constraint::Percentage(10),
                Constraint::Percentage(15),
            ])
            .split(main_layout[1]);
        return (main_layout[0], right_side[0], right_side[1], right_side[2], right_side[3]);
    }
    
    fn next_row(&mut self) {
        let row = match self.state.selected() {
            Some(row) => {
                if row >= self.processes.len() - 1 {
                    self.processes.len() - 1
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