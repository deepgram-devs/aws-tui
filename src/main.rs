use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_ec2::Client as Ec2Client;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};
use std::io;

#[derive(Clone)]
struct Ec2Instance {
    id: String,
    name: String,
    state: String,
    instance_type: String,
    public_ip: String,
    private_ip: String,
}

struct App {
    instances: Vec<Ec2Instance>,
    selected_instances: Vec<bool>,
    table_state: TableState,
    current_region: String,
    available_regions: Vec<String>,
    region_index: usize,
    status_message: String,
    show_help: bool,
}

impl App {
    fn new() -> Self {
        let regions = vec![
            "us-east-1".to_string(),
            "us-east-2".to_string(),
            "us-west-1".to_string(),
            "us-west-2".to_string(),
            "eu-west-1".to_string(),
            "eu-central-1".to_string(),
            "ap-southeast-1".to_string(),
            "ap-northeast-1".to_string(),
        ];
        
        Self {
            instances: Vec::new(),
            selected_instances: Vec::new(),
            table_state: TableState::default(),
            current_region: regions[0].clone(),
            available_regions: regions,
            region_index: 0,
            status_message: "Press 'h' for help".to_string(),
            show_help: false,
        }
    }

    fn next_region(&mut self) {
        self.region_index = (self.region_index + 1) % self.available_regions.len();
        self.current_region = self.available_regions[self.region_index].clone();
        self.status_message = format!("Switched to region: {}", self.current_region);
    }

    fn previous_region(&mut self) {
        if self.region_index == 0 {
            self.region_index = self.available_regions.len() - 1;
        } else {
            self.region_index -= 1;
        }
        self.current_region = self.available_regions[self.region_index].clone();
        self.status_message = format!("Switched to region: {}", self.current_region);
    }

    fn next_instance(&mut self) {
        if self.instances.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.instances.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous_instance(&mut self) {
        if self.instances.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.instances.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn toggle_selection(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if i < self.selected_instances.len() {
                self.selected_instances[i] = !self.selected_instances[i];
            }
        }
    }

    fn get_selected_instance_ids(&self) -> Vec<String> {
        self.instances
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected_instances.get(*i).copied().unwrap_or(false))
            .map(|(_, inst)| inst.id.clone())
            .collect()
    }

    fn clear_selections(&mut self) {
        self.selected_instances = vec![false; self.instances.len()];
    }
}

async fn load_instances(region: &str) -> Result<Vec<Ec2Instance>> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = Ec2Client::new(&config);
    
    let resp = client.describe_instances().send().await?;
    
    let mut instances = Vec::new();
    
    for reservation in resp.reservations() {
        for instance in reservation.instances() {
            let id = instance.instance_id().unwrap_or("N/A").to_string();
            
            let name = instance
                .tags()
                .iter()
                .find(|tag| tag.key() == Some("Name"))
                .and_then(|tag| tag.value())
                .unwrap_or("N/A")
                .to_string();
            
            let state = instance
                .state()
                .and_then(|s| s.name())
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "Unknown".to_string());
            
            let instance_type = instance
                .instance_type()
                .map(|t| format!("{:?}", t))
                .unwrap_or_else(|| "N/A".to_string());
            
            let public_ip = instance
                .public_ip_address()
                .unwrap_or("N/A")
                .to_string();
            
            let private_ip = instance
                .private_ip_address()
                .unwrap_or("N/A")
                .to_string();
            
            instances.push(Ec2Instance {
                id,
                name,
                state,
                instance_type,
                public_ip,
                private_ip,
            });
        }
    }
    
    Ok(instances)
}

async fn start_instances(region: &str, instance_ids: Vec<String>) -> Result<String> {
    if instance_ids.is_empty() {
        return Ok("No instances selected".to_string());
    }

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = Ec2Client::new(&config);
    
    client
        .start_instances()
        .set_instance_ids(Some(instance_ids.clone()))
        .send()
        .await?;
    
    Ok(format!("Started {} instance(s)", instance_ids.len()))
}

async fn stop_instances(region: &str, instance_ids: Vec<String>) -> Result<String> {
    if instance_ids.is_empty() {
        return Ok("No instances selected".to_string());
    }

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = Ec2Client::new(&config);
    
    client
        .stop_instances()
        .set_instance_ids(Some(instance_ids.clone()))
        .send()
        .await?;
    
    Ok(format!("Stopped {} instance(s)", instance_ids.len()))
}

async fn terminate_instances(region: &str, instance_ids: Vec<String>) -> Result<String> {
    if instance_ids.is_empty() {
        return Ok("No instances selected".to_string());
    }

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = Ec2Client::new(&config);
    
    client
        .terminate_instances()
        .set_instance_ids(Some(instance_ids.clone()))
        .send()
        .await?;
    
    Ok(format!("Terminated {} instance(s)", instance_ids.len()))
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("EC2 Instance Manager", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(format!("Region: {}", app.current_region), Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(format!("Instances: {}", app.instances.len()), Style::default().fg(Color::Green)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Info"));
    f.render_widget(header, chunks[0]);

    // Help overlay
    if app.show_help {
        let help_text = vec![
            Line::from("Keyboard Shortcuts:"),
            Line::from(""),
            Line::from("  ↑/↓ or j/k    - Navigate instances"),
            Line::from("  Space         - Toggle instance selection"),
            Line::from("  ←/→           - Switch regions"),
            Line::from("  r             - Refresh instance list"),
            Line::from("  s             - Start selected instances"),
            Line::from("  t             - Stop selected instances"),
            Line::from("  d             - Terminate selected instances"),
            Line::from("  c             - Clear all selections"),
            Line::from("  h             - Toggle this help"),
            Line::from("  q or Ctrl+C   - Quit"),
        ];
        
        let help_area = centered_rect(60, 60, f.area());
        let help = Paragraph::new(help_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .style(Style::default().bg(Color::Black)));
        f.render_widget(help, help_area);
    } else {
        // Instance table
        let selected_style = Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD);
        
        let header_cells = ["✓", "Instance ID", "Name", "State", "Type", "Public IP", "Private IP"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        
        let header = Row::new(header_cells).height(1).bottom_margin(1);
        
        let rows = app.instances.iter().enumerate().map(|(i, inst)| {
            let checkbox = if app.selected_instances.get(i).copied().unwrap_or(false) {
                "[✓]"
            } else {
                "[ ]"
            };
            
            let state_color = match inst.state.as_str() {
                "Running" => Color::Green,
                "Stopped" => Color::Red,
                "Stopping" => Color::Yellow,
                "Pending" => Color::Cyan,
                _ => Color::White,
            };
            
            let cells = vec![
                Cell::from(checkbox),
                Cell::from(inst.id.clone()),
                Cell::from(inst.name.clone()),
                Cell::from(inst.state.clone()).style(Style::default().fg(state_color)),
                Cell::from(inst.instance_type.clone()),
                Cell::from(inst.public_ip.clone()),
                Cell::from(inst.private_ip.clone()),
            ];
            
            Row::new(cells).height(1)
        });
        
        let table = Table::new(
            rows,
            [
                Constraint::Length(5),
                Constraint::Length(20),
                Constraint::Length(25),
                Constraint::Length(12),
                Constraint::Length(15),
                Constraint::Length(16),
                Constraint::Length(16),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("EC2 Instances"))
        .highlight_style(selected_style)
        .highlight_symbol(">> ");
        
        f.render_stateful_widget(table, chunks[1], &mut app.table_state);
    }

    // Status bar
    let status = Paragraph::new(app.status_message.clone())
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(status, chunks[2]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();
    
    // Load initial instances
    app.status_message = "Loading instances...".to_string();
    terminal.draw(|f| ui(f, &mut app))?;
    
    match load_instances(&app.current_region).await {
        Ok(instances) => {
            app.instances = instances;
            app.selected_instances = vec![false; app.instances.len()];
            if !app.instances.is_empty() {
                app.table_state.select(Some(0));
            }
            app.status_message = format!("Loaded {} instances from {}", app.instances.len(), app.current_region);
        }
        Err(e) => {
            app.status_message = format!("Error loading instances: {}", e);
        }
    }

    // Main loop
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }

                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('h') => {
                        app.show_help = !app.show_help;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if !app.show_help {
                            app.next_instance();
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if !app.show_help {
                            app.previous_instance();
                        }
                    }
                    KeyCode::Left => {
                        if !app.show_help {
                            app.previous_region();
                            app.status_message = "Loading instances...".to_string();
                            terminal.draw(|f| ui(f, &mut app))?;
                            
                            match load_instances(&app.current_region).await {
                                Ok(instances) => {
                                    app.instances = instances;
                                    app.selected_instances = vec![false; app.instances.len()];
                                    if !app.instances.is_empty() {
                                        app.table_state.select(Some(0));
                                    } else {
                                        app.table_state.select(None);
                                    }
                                    app.status_message = format!("Loaded {} instances from {}", app.instances.len(), app.current_region);
                                }
                                Err(e) => {
                                    app.status_message = format!("Error loading instances: {}", e);
                                }
                            }
                        }
                    }
                    KeyCode::Right => {
                        if !app.show_help {
                            app.next_region();
                            app.status_message = "Loading instances...".to_string();
                            terminal.draw(|f| ui(f, &mut app))?;
                            
                            match load_instances(&app.current_region).await {
                                Ok(instances) => {
                                    app.instances = instances;
                                    app.selected_instances = vec![false; app.instances.len()];
                                    if !app.instances.is_empty() {
                                        app.table_state.select(Some(0));
                                    } else {
                                        app.table_state.select(None);
                                    }
                                    app.status_message = format!("Loaded {} instances from {}", app.instances.len(), app.current_region);
                                }
                                Err(e) => {
                                    app.status_message = format!("Error loading instances: {}", e);
                                }
                            }
                        }
                    }
                    KeyCode::Char(' ') => {
                        if !app.show_help {
                            app.toggle_selection();
                        }
                    }
                    KeyCode::Char('r') => {
                        if !app.show_help {
                            app.status_message = "Refreshing instances...".to_string();
                            terminal.draw(|f| ui(f, &mut app))?;
                            
                            match load_instances(&app.current_region).await {
                                Ok(instances) => {
                                    app.instances = instances;
                                    app.selected_instances = vec![false; app.instances.len()];
                                    if !app.instances.is_empty() && app.table_state.selected().is_none() {
                                        app.table_state.select(Some(0));
                                    }
                                    app.status_message = format!("Refreshed {} instances from {}", app.instances.len(), app.current_region);
                                }
                                Err(e) => {
                                    app.status_message = format!("Error refreshing instances: {}", e);
                                }
                            }
                        }
                    }
                    KeyCode::Char('c') => {
                        if !app.show_help {
                            app.clear_selections();
                            app.status_message = "Cleared all selections".to_string();
                        }
                    }
                    KeyCode::Char('s') => {
                        if !app.show_help {
                            let instance_ids = app.get_selected_instance_ids();
                            if !instance_ids.is_empty() {
                                app.status_message = "Starting instances...".to_string();
                                terminal.draw(|f| ui(f, &mut app))?;
                                
                                match start_instances(&app.current_region, instance_ids).await {
                                    Ok(msg) => {
                                        app.status_message = msg;
                                        app.clear_selections();
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Error starting instances: {}", e);
                                    }
                                }
                            } else {
                                app.status_message = "No instances selected".to_string();
                            }
                        }
                    }
                    KeyCode::Char('t') => {
                        if !app.show_help {
                            let instance_ids = app.get_selected_instance_ids();
                            if !instance_ids.is_empty() {
                                app.status_message = "Stopping instances...".to_string();
                                terminal.draw(|f| ui(f, &mut app))?;
                                
                                match stop_instances(&app.current_region, instance_ids).await {
                                    Ok(msg) => {
                                        app.status_message = msg;
                                        app.clear_selections();
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Error stopping instances: {}", e);
                                    }
                                }
                            } else {
                                app.status_message = "No instances selected".to_string();
                            }
                        }
                    }
                    KeyCode::Char('d') => {
                        if !app.show_help {
                            let instance_ids = app.get_selected_instance_ids();
                            if !instance_ids.is_empty() {
                                app.status_message = "Terminating instances...".to_string();
                                terminal.draw(|f| ui(f, &mut app))?;
                                
                                match terminate_instances(&app.current_region, instance_ids).await {
                                    Ok(msg) => {
                                        app.status_message = msg;
                                        app.clear_selections();
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Error terminating instances: {}", e);
                                    }
                                }
                            } else {
                                app.status_message = "No instances selected".to_string();
                            }
                        }
                    }
                    _ => {}
                }
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
    terminal.show_cursor()?;

    Ok(())
}
