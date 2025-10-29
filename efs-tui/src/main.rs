use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_efs::Client as EfsClient;
use aws_sdk_efs::types::{LifeCycleState, PerformanceMode, ThroughputMode};
use chrono::Utc;
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
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};
use regex::Regex;
use std::io;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct EfsVolume {
    file_system_id: String,
    name: String,
    life_cycle_state: String,
    size_in_bytes: u64,
    number_of_mount_targets: i32,
    performance_mode: String,
    throughput_mode: String,
    encrypted: bool,
    creation_time: String,
}

#[derive(Clone)]
struct MountTarget {
    mount_target_id: String,
    file_system_id: String,
    subnet_id: String,
    ip_address: String,
    availability_zone_name: String,
    life_cycle_state: String,
}

#[derive(Clone, Debug)]
enum LogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
struct LogEntry {
    timestamp: String,
    level: LogLevel,
    message: String,
}

impl LogEntry {
    fn new(level: LogLevel, message: String) -> Self {
        Self {
            timestamp: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            level,
            message,
        }
    }
}

type AppLog = Arc<Mutex<Vec<LogEntry>>>;

enum ViewMode {
    FileSystemList,
    MountTargetList,
}

struct App {
    view_mode: ViewMode,
    file_systems: Vec<EfsVolume>,
    selected_file_systems: Vec<bool>,
    fs_table_state: TableState,
    mount_targets: Vec<MountTarget>,
    selected_mount_targets: Vec<bool>,
    mt_table_state: TableState,
    current_region: String,
    available_regions: Vec<String>,
    region_index: usize,
    status_message: String,
    show_help: bool,
    show_logs: bool,
    log_wrap_enabled: bool,
    log: AppLog,
    is_loading: bool,
    show_region_selector: bool,
    region_filter: String,
    filtered_regions: Vec<String>,
    region_selector_state: ListState,
    selected_fs_for_mount_targets: Option<String>,
}

impl App {
    fn new(log: AppLog) -> Self {
        let regions = vec![
            "us-east-1".to_string(),
            "us-east-2".to_string(),
            "us-west-1".to_string(),
            "us-west-2".to_string(),
            "af-south-1".to_string(),
            "ap-east-1".to_string(),
            "ap-south-1".to_string(),
            "ap-south-2".to_string(),
            "ap-southeast-1".to_string(),
            "ap-southeast-2".to_string(),
            "ap-southeast-3".to_string(),
            "ap-southeast-4".to_string(),
            "ap-northeast-1".to_string(),
            "ap-northeast-2".to_string(),
            "ap-northeast-3".to_string(),
            "ca-central-1".to_string(),
            "ca-west-1".to_string(),
            "eu-central-1".to_string(),
            "eu-central-2".to_string(),
            "eu-west-1".to_string(),
            "eu-west-2".to_string(),
            "eu-west-3".to_string(),
            "eu-south-1".to_string(),
            "eu-south-2".to_string(),
            "eu-north-1".to_string(),
            "il-central-1".to_string(),
            "me-south-1".to_string(),
            "me-central-1".to_string(),
            "sa-east-1".to_string(),
        ];
        
        let filtered_regions = regions.clone();
        let mut region_selector_state = ListState::default();
        region_selector_state.select(Some(0));
        
        Self {
            view_mode: ViewMode::FileSystemList,
            file_systems: Vec::new(),
            selected_file_systems: Vec::new(),
            fs_table_state: TableState::default(),
            mount_targets: Vec::new(),
            selected_mount_targets: Vec::new(),
            mt_table_state: TableState::default(),
            current_region: regions[0].clone(),
            available_regions: regions,
            region_index: 0,
            status_message: "Press 'h' for help".to_string(),
            show_help: false,
            show_logs: false,
            log_wrap_enabled: false,
            log,
            is_loading: false,
            show_region_selector: false,
            region_filter: String::new(),
            filtered_regions,
            region_selector_state,
            selected_fs_for_mount_targets: None,
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

    fn next_item(&mut self) {
        match self.view_mode {
            ViewMode::FileSystemList => {
                if self.file_systems.is_empty() {
                    return;
                }
                let i = match self.fs_table_state.selected() {
                    Some(i) => {
                        if i >= self.file_systems.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.fs_table_state.select(Some(i));
            }
            ViewMode::MountTargetList => {
                if self.mount_targets.is_empty() {
                    return;
                }
                let i = match self.mt_table_state.selected() {
                    Some(i) => {
                        if i >= self.mount_targets.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.mt_table_state.select(Some(i));
            }
        }
    }

    fn previous_item(&mut self) {
        match self.view_mode {
            ViewMode::FileSystemList => {
                if self.file_systems.is_empty() {
                    return;
                }
                let i = match self.fs_table_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.file_systems.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.fs_table_state.select(Some(i));
            }
            ViewMode::MountTargetList => {
                if self.mount_targets.is_empty() {
                    return;
                }
                let i = match self.mt_table_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.mount_targets.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.mt_table_state.select(Some(i));
            }
        }
    }

    fn toggle_selection(&mut self) {
        match self.view_mode {
            ViewMode::FileSystemList => {
                if let Some(i) = self.fs_table_state.selected() {
                    if i < self.selected_file_systems.len() {
                        self.selected_file_systems[i] = !self.selected_file_systems[i];
                    }
                }
            }
            ViewMode::MountTargetList => {
                if let Some(i) = self.mt_table_state.selected() {
                    if i < self.selected_mount_targets.len() {
                        self.selected_mount_targets[i] = !self.selected_mount_targets[i];
                    }
                }
            }
        }
    }

    fn get_selected_fs_ids(&self) -> Vec<String> {
        self.file_systems
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected_file_systems.get(*i).copied().unwrap_or(false))
            .map(|(_, fs)| fs.file_system_id.clone())
            .collect()
    }

    fn get_selected_mt_ids(&self) -> Vec<String> {
        self.mount_targets
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected_mount_targets.get(*i).copied().unwrap_or(false))
            .map(|(_, mt)| mt.mount_target_id.clone())
            .collect()
    }

    fn clear_selections(&mut self) {
        match self.view_mode {
            ViewMode::FileSystemList => {
                self.selected_file_systems = vec![false; self.file_systems.len()];
            }
            ViewMode::MountTargetList => {
                self.selected_mount_targets = vec![false; self.mount_targets.len()];
            }
        }
    }

    fn open_region_selector(&mut self) {
        self.show_region_selector = true;
        self.region_filter.clear();
        self.filtered_regions = self.available_regions.clone();
        self.region_selector_state.select(Some(0));
    }

    fn close_region_selector(&mut self) {
        self.show_region_selector = false;
        self.region_filter.clear();
    }

    fn update_region_filter(&mut self) {
        if self.region_filter.is_empty() {
            self.filtered_regions = self.available_regions.clone();
        } else {
            match Regex::new(&self.region_filter) {
                Ok(re) => {
                    self.filtered_regions = self.available_regions
                        .iter()
                        .filter(|region| re.is_match(region))
                        .cloned()
                        .collect();
                }
                Err(_) => {
                    let filter_lower = self.region_filter.to_lowercase();
                    self.filtered_regions = self.available_regions
                        .iter()
                        .filter(|region| region.to_lowercase().contains(&filter_lower))
                        .cloned()
                        .collect();
                }
            }
        }
        
        let current_selection = self.region_selector_state.selected().unwrap_or(0);
        if current_selection >= self.filtered_regions.len() && !self.filtered_regions.is_empty() {
            self.region_selector_state.select(Some(0));
        } else if self.filtered_regions.is_empty() {
            self.region_selector_state.select(None);
        }
    }

    fn next_filtered_region(&mut self) {
        if self.filtered_regions.is_empty() {
            return;
        }
        
        let i = match self.region_selector_state.selected() {
            Some(i) => {
                if i >= self.filtered_regions.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.region_selector_state.select(Some(i));
    }

    fn previous_filtered_region(&mut self) {
        if self.filtered_regions.is_empty() {
            return;
        }
        
        let i = match self.region_selector_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_regions.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.region_selector_state.select(Some(i));
    }

    fn select_filtered_region(&mut self) -> Option<String> {
        if let Some(selected_idx) = self.region_selector_state.selected() {
            if selected_idx < self.filtered_regions.len() {
                let selected_region = self.filtered_regions[selected_idx].clone();
                
                if let Some(idx) = self.available_regions.iter().position(|r| r == &selected_region) {
                    self.region_index = idx;
                    self.current_region = selected_region.clone();
                }
                
                self.close_region_selector();
                return Some(selected_region);
            }
        }
        None
    }

    fn back_to_file_systems(&mut self) {
        self.view_mode = ViewMode::FileSystemList;
        self.selected_fs_for_mount_targets = None;
        self.mount_targets.clear();
        self.selected_mount_targets.clear();
        self.mt_table_state.select(None);
        self.status_message = "Returned to file system list".to_string();
    }
}

async fn load_file_systems(region: &str) -> Result<Vec<EfsVolume>> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = EfsClient::new(&config);
    
    let resp = client.describe_file_systems().send().await?;
    
    let mut file_systems = Vec::new();
    
    for fs in resp.file_systems() {
        let file_system_id = fs.file_system_id().to_string();
        
        let name = fs
            .tags()
            .iter()
            .find(|tag| tag.key() == "Name")
            .map(|tag| tag.value().to_string())
            .unwrap_or_else(|| "N/A".to_string());
        
        let life_cycle_state = match fs.life_cycle_state() {
            LifeCycleState::Available => "Available".to_string(),
            LifeCycleState::Creating => "Creating".to_string(),
            LifeCycleState::Deleting => "Deleting".to_string(),
            LifeCycleState::Deleted => "Deleted".to_string(),
            LifeCycleState::Updating => "Updating".to_string(),
            LifeCycleState::Error => "Error".to_string(),
            _ => "Unknown".to_string(),
        };
        
        let size_in_bytes = fs.size_in_bytes().map(|s| s.value() as u64).unwrap_or(0);
        
        let number_of_mount_targets = fs.number_of_mount_targets();
        
        let performance_mode = match fs.performance_mode() {
            PerformanceMode::GeneralPurpose => "GeneralPurpose".to_string(),
            PerformanceMode::MaxIo => "MaxIO".to_string(),
            _ => "Unknown".to_string(),
        };
        
        let throughput_mode = match fs.throughput_mode() {
            Some(ThroughputMode::Bursting) => "Bursting".to_string(),
            Some(ThroughputMode::Provisioned) => "Provisioned".to_string(),
            Some(ThroughputMode::Elastic) => "Elastic".to_string(),
            _ => "Unknown".to_string(),
        };
        
        let encrypted = fs.encrypted().unwrap_or(false);
        
        let creation_time = format!("{}", fs.creation_time());
        
        file_systems.push(EfsVolume {
            file_system_id,
            name,
            life_cycle_state,
            size_in_bytes,
            number_of_mount_targets,
            performance_mode,
            throughput_mode,
            encrypted,
            creation_time,
        });
    }
    
    Ok(file_systems)
}

async fn load_mount_targets(region: &str, file_system_id: &str) -> Result<Vec<MountTarget>> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = EfsClient::new(&config);
    
    let resp = client
        .describe_mount_targets()
        .file_system_id(file_system_id)
        .send()
        .await?;
    
    let mut mount_targets = Vec::new();
    
    for mt in resp.mount_targets() {
        let mount_target_id = mt.mount_target_id().to_string();
        let file_system_id = mt.file_system_id().to_string();
        let subnet_id = mt.subnet_id().to_string();
        let ip_address = mt.ip_address().map(|s| s.to_string()).unwrap_or_else(|| "N/A".to_string());
        let availability_zone_name = mt.availability_zone_name().map(|s| s.to_string()).unwrap_or_else(|| "N/A".to_string());
        
        let life_cycle_state = match mt.life_cycle_state() {
            LifeCycleState::Available => "Available".to_string(),
            LifeCycleState::Creating => "Creating".to_string(),
            LifeCycleState::Deleting => "Deleting".to_string(),
            LifeCycleState::Deleted => "Deleted".to_string(),
            LifeCycleState::Updating => "Updating".to_string(),
            LifeCycleState::Error => "Error".to_string(),
            _ => "Unknown".to_string(),
        };
        
        mount_targets.push(MountTarget {
            mount_target_id,
            file_system_id,
            subnet_id,
            ip_address,
            availability_zone_name,
            life_cycle_state,
        });
    }
    
    Ok(mount_targets)
}

async fn delete_file_systems(region: &str, file_system_ids: Vec<String>, log: &AppLog) -> Result<String> {
    if file_system_ids.is_empty() {
        return Ok("No file systems selected".to_string());
    }

    log_message(log, LogLevel::Info, format!("Starting deletion of {} file system(s) in region {}", file_system_ids.len(), region));

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = EfsClient::new(&config);
    
    let mut deleted_count = 0;
    let mut errors = Vec::new();
    
    for fs_id in &file_system_ids {
        log_message(log, LogLevel::Info, format!("Deleting file system: {}", fs_id));
        
        match client.delete_file_system().file_system_id(fs_id).send().await {
            Ok(_) => {
                deleted_count += 1;
                log_message(log, LogLevel::Info, format!("Successfully deleted file system: {}", fs_id));
            }
            Err(e) => {
                let error_msg = format!("Failed to delete {}: {}", fs_id, e);
                log_message(log, LogLevel::Error, error_msg.clone());
                errors.push(error_msg);
            }
        }
    }
    
    let result_msg = if errors.is_empty() {
        format!("Successfully deleted {} file system(s)", deleted_count)
    } else {
        format!("Deleted {} of {} file system(s). Errors: {}", deleted_count, file_system_ids.len(), errors.join("; "))
    };
    
    log_message(log, LogLevel::Info, result_msg.clone());
    Ok(result_msg)
}

async fn delete_mount_targets(region: &str, mount_target_ids: Vec<String>, log: &AppLog) -> Result<String> {
    if mount_target_ids.is_empty() {
        return Ok("No mount targets selected".to_string());
    }

    log_message(log, LogLevel::Info, format!("Starting deletion of {} mount target(s) in region {}", mount_target_ids.len(), region));

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = EfsClient::new(&config);
    
    let mut deleted_count = 0;
    let mut errors = Vec::new();
    
    for mt_id in &mount_target_ids {
        log_message(log, LogLevel::Info, format!("Deleting mount target: {}", mt_id));
        
        match client.delete_mount_target().mount_target_id(mt_id).send().await {
            Ok(_) => {
                deleted_count += 1;
                log_message(log, LogLevel::Info, format!("Successfully deleted mount target: {}", mt_id));
            }
            Err(e) => {
                let error_msg = format!("Failed to delete {}: {}", mt_id, e);
                log_message(log, LogLevel::Error, error_msg.clone());
                errors.push(error_msg);
            }
        }
    }
    
    let result_msg = if errors.is_empty() {
        format!("Successfully deleted {} mount target(s)", deleted_count)
    } else {
        format!("Deleted {} of {} mount target(s). Errors: {}", deleted_count, mount_target_ids.len(), errors.join("; "))
    };
    
    log_message(log, LogLevel::Info, result_msg.clone());
    Ok(result_msg)
}

fn log_message(log: &AppLog, level: LogLevel, message: String) {
    if let Ok(mut log_entries) = log.lock() {
        log_entries.push(LogEntry::new(level, message));
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
    
    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
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
    let view_name = match app.view_mode {
        ViewMode::FileSystemList => "File Systems",
        ViewMode::MountTargetList => "Mount Targets",
    };
    
    let count = match app.view_mode {
        ViewMode::FileSystemList => app.file_systems.len(),
        ViewMode::MountTargetList => app.mount_targets.len(),
    };
    
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("EFS Manager", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(format!("Region: {}", app.current_region), Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(format!("View: {}", view_name), Style::default().fg(Color::Magenta)),
            Span::raw(" | "),
            Span::styled(format!("Count: {}", count), Style::default().fg(Color::Green)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Info"));
    f.render_widget(header, chunks[0]);

    // Loading overlay
    if app.is_loading {
        let loading_area = centered_rect(40, 20, f.area());
        
        let loading_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("⏳ Loading...", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(format!("Region: {}", app.current_region), Style::default().fg(Color::Yellow)),
            ]),
        ];
        
        let loading = Paragraph::new(loading_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Loading")
                .style(Style::default().bg(Color::Black)))
            .alignment(ratatui::layout::Alignment::Center);
        
        f.render_widget(loading, loading_area);
    } else if app.show_region_selector {
        let selector_area = centered_rect(60, 70, f.area());
        
        let selected_idx = app.region_selector_state.selected();
        let region_items: Vec<ListItem> = app.filtered_regions
            .iter()
            .enumerate()
            .map(|(i, region)| {
                let is_selected = selected_idx == Some(i);
                let is_current = region == &app.current_region;
                
                let style = if is_current && !is_selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                
                ListItem::new(Line::from(vec![
                    Span::styled(region.clone(), style),
                ]))
            })
            .collect();
        
        let title = if app.region_filter.is_empty() {
            format!("Select Region (showing {} of {})", app.filtered_regions.len(), app.available_regions.len())
        } else {
            format!("Select Region - Filter: '{}' (showing {} of {})", app.region_filter, app.filtered_regions.len(), app.available_regions.len())
        };
        
        let region_list = List::new(region_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(Style::default().bg(Color::Black)))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");
        
        let selector_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(5),
            ])
            .split(selector_area);
        
        f.render_stateful_widget(region_list, selector_chunks[0], &mut app.region_selector_state);
        
        let instructions = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("↑/↓ or j/k", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Navigate  "),
                Span::styled("Enter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Select"),
            ]),
            Line::from(vec![
                Span::styled("Type", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Filter (regex)  "),
                Span::styled("Backspace", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Delete char"),
            ]),
            Line::from(vec![
                Span::styled("Esc or g", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Cancel"),
            ]),
        ])
        .block(Block::default()
            .borders(Borders::ALL)
            .title("Controls")
            .style(Style::default().bg(Color::Black)));
        
        f.render_widget(instructions, selector_chunks[1]);
    } else if app.show_logs {
        let log_area = centered_rect(80, 80, f.area());
        
        let log_entries = if let Ok(entries) = app.log.lock() {
            entries.iter().rev().take(100).rev().cloned().collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        
        let log_items: Vec<ListItem> = log_entries
            .iter()
            .flat_map(|entry| {
                let level_style = match entry.level {
                    LogLevel::Info => Style::default().fg(Color::Green),
                    LogLevel::Warning => Style::default().fg(Color::Yellow),
                    LogLevel::Error => Style::default().fg(Color::Red),
                };
                
                let level_text = match entry.level {
                    LogLevel::Info => "INFO",
                    LogLevel::Warning => "WARN",
                    LogLevel::Error => "ERROR",
                };
                
                let prefix = format!("[{}] {:5} ", entry.timestamp, level_text);
                
                if app.log_wrap_enabled {
                    let available_width = (log_area.width as usize).saturating_sub(prefix.len() + 4);
                    
                    if available_width > 0 {
                        let mut lines = Vec::new();
                        let message_chars: Vec<char> = entry.message.chars().collect();
                        let mut start = 0;
                        
                        while start < message_chars.len() {
                            let end = (start + available_width).min(message_chars.len());
                            let chunk: String = message_chars[start..end].iter().collect();
                            
                            if start == 0 {
                                lines.push(ListItem::new(Line::from(vec![
                                    Span::styled(format!("[{}] ", entry.timestamp), Style::default().fg(Color::DarkGray)),
                                    Span::styled(format!("{:5} ", level_text), level_style.add_modifier(Modifier::BOLD)),
                                    Span::raw(chunk),
                                ])));
                            } else {
                                lines.push(ListItem::new(Line::from(vec![
                                    Span::raw(" ".repeat(prefix.len())),
                                    Span::raw(chunk),
                                ])));
                            }
                            
                            start = end;
                        }
                        lines
                    } else {
                        vec![ListItem::new(Line::from(vec![
                            Span::styled(format!("[{}] ", entry.timestamp), Style::default().fg(Color::DarkGray)),
                            Span::styled(format!("{:5} ", level_text), level_style.add_modifier(Modifier::BOLD)),
                            Span::raw(&entry.message),
                        ]))]
                    }
                } else {
                    vec![ListItem::new(Line::from(vec![
                        Span::styled(format!("[{}] ", entry.timestamp), Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("{:5} ", level_text), level_style.add_modifier(Modifier::BOLD)),
                        Span::raw(&entry.message),
                    ]))]
                }
            })
            .collect();
        
        let wrap_status = if app.log_wrap_enabled { "ON" } else { "OFF" };
        let title = format!("Application Logs (Press 'l' to close, 'w' to toggle wrap [{}], showing last 100 entries)", wrap_status);
        
        let log_list = List::new(log_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(Style::default().bg(Color::Black)));
        
        f.render_widget(log_list, log_area);
    } else if app.show_help {
        let help_text = vec![
            Line::from("Keyboard Shortcuts:"),
            Line::from(""),
            Line::from("  ↑/↓ or j/k    - Navigate items"),
            Line::from("  Space         - Toggle selection"),
            Line::from("  ←/→           - Switch regions"),
            Line::from("  g             - Select region (with filter)"),
            Line::from("  r             - Refresh current view"),
            Line::from("  d             - Delete selected items"),
            Line::from("  m             - View mount targets (file system view)"),
            Line::from("  b             - Back to file systems (mount target view)"),
            Line::from("  c             - Clear all selections"),
            Line::from("  l             - Toggle application logs"),
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
        match app.view_mode {
            ViewMode::FileSystemList => {
                let selected_style = Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD);
                
                let header_cells = ["✓", "File System ID", "Name", "State", "Size", "Mount Targets", "Performance", "Throughput", "Encrypted"]
                    .iter()
                    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                
                let header = Row::new(header_cells).height(1).bottom_margin(1);
                
                let rows = app.file_systems.iter().enumerate().map(|(i, fs)| {
                    let checkbox = if app.selected_file_systems.get(i).copied().unwrap_or(false) {
                        "[✓]"
                    } else {
                        "[ ]"
                    };
                    
                    let state_color = match fs.life_cycle_state.as_str() {
                        "Available" => Color::Green,
                        "Creating" => Color::Cyan,
                        "Deleting" => Color::Red,
                        "Error" => Color::Red,
                        _ => Color::White,
                    };
                    
                    let encrypted_text = if fs.encrypted { "Yes" } else { "No" };
                    
                    let cells = vec![
                        Cell::from(checkbox),
                        Cell::from(fs.file_system_id.clone()),
                        Cell::from(fs.name.clone()),
                        Cell::from(fs.life_cycle_state.clone()).style(Style::default().fg(state_color)),
                        Cell::from(format_bytes(fs.size_in_bytes)),
                        Cell::from(fs.number_of_mount_targets.to_string()),
                        Cell::from(fs.performance_mode.clone()),
                        Cell::from(fs.throughput_mode.clone()),
                        Cell::from(encrypted_text),
                    ];
                    
                    Row::new(cells).height(1)
                });
                
                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(5),
                        Constraint::Length(25),
                        Constraint::Length(32),
                        Constraint::Length(12),
                        Constraint::Length(12),
                        Constraint::Length(15),
                        Constraint::Length(15),
                        Constraint::Length(12),
                        Constraint::Length(10),
                    ],
                )
                .header(header)
                .block(Block::default().borders(Borders::ALL).title("EFS File Systems"))
                .row_highlight_style(selected_style)
                .highlight_symbol(">> ");
                
                f.render_stateful_widget(table, chunks[1], &mut app.fs_table_state);
            }
            ViewMode::MountTargetList => {
                let selected_style = Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD);
                
                let header_cells = ["✓", "Mount Target ID", "File System ID", "Subnet ID", "IP Address", "Availability Zone", "State"]
                    .iter()
                    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                
                let header = Row::new(header_cells).height(1).bottom_margin(1);
                
                let rows = app.mount_targets.iter().enumerate().map(|(i, mt)| {
                    let checkbox = if app.selected_mount_targets.get(i).copied().unwrap_or(false) {
                        "[✓]"
                    } else {
                        "[ ]"
                    };
                    
                    let state_color = match mt.life_cycle_state.as_str() {
                        "Available" => Color::Green,
                        "Creating" => Color::Cyan,
                        "Deleting" => Color::Red,
                        "Error" => Color::Red,
                        _ => Color::White,
                    };
                    
                    let cells = vec![
                        Cell::from(checkbox),
                        Cell::from(mt.mount_target_id.clone()),
                        Cell::from(mt.file_system_id.clone()),
                        Cell::from(mt.subnet_id.clone()),
                        Cell::from(mt.ip_address.clone()),
                        Cell::from(mt.availability_zone_name.clone()),
                        Cell::from(mt.life_cycle_state.clone()).style(Style::default().fg(state_color)),
                    ];
                    
                    Row::new(cells).height(1)
                });
                
                let fs_id_display = app.selected_fs_for_mount_targets.as_ref().map(|s| s.as_str()).unwrap_or("N/A");
                let title = format!("Mount Targets for File System: {}", fs_id_display);
                
                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(5),
                        Constraint::Length(25),
                        Constraint::Length(25),
                        Constraint::Length(20),
                        Constraint::Length(16),
                        Constraint::Length(20),
                        Constraint::Length(12),
                    ],
                )
                .header(header)
                .block(Block::default().borders(Borders::ALL).title(title))
                .row_highlight_style(selected_style)
                .highlight_symbol(">> ");
                
                f.render_stateful_widget(table, chunks[1], &mut app.mt_table_state);
            }
        }
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
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_log: AppLog = Arc::new(Mutex::new(Vec::new()));
    let mut app = App::new(app_log.clone());
    
    log_message(&app_log, LogLevel::Info, "EFS TUI application started".to_string());
    
    // Load initial file systems
    app.is_loading = true;
    terminal.draw(|f| ui(f, &mut app))?;
    
    match load_file_systems(&app.current_region).await {
        Ok(file_systems) => {
            app.file_systems = file_systems;
            app.selected_file_systems = vec![false; app.file_systems.len()];
            if !app.file_systems.is_empty() {
                app.fs_table_state.select(Some(0));
            }
            app.status_message = format!("Loaded {} file systems from {}", app.file_systems.len(), app.current_region);
        }
        Err(e) => {
            app.status_message = format!("Error loading file systems: {}", e);
            log_message(&app_log, LogLevel::Error, format!("Error loading file systems: {}", e));
        }
    }
    app.is_loading = false;

    // Main loop
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }

                // Handle region selector input
                if app.show_region_selector {
                    match key.code {
                        KeyCode::Char('g') | KeyCode::Esc => {
                            app.close_region_selector();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.next_filtered_region();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.previous_filtered_region();
                        }
                        KeyCode::Enter => {
                            if let Some(selected_region) = app.select_filtered_region() {
                                app.is_loading = true;
                                terminal.draw(|f| ui(f, &mut app))?;
                                
                                match load_file_systems(&selected_region).await {
                                    Ok(file_systems) => {
                                        app.file_systems = file_systems;
                                        app.selected_file_systems = vec![false; app.file_systems.len()];
                                        if !app.file_systems.is_empty() {
                                            app.fs_table_state.select(Some(0));
                                        } else {
                                            app.fs_table_state.select(None);
                                        }
                                        app.status_message = format!("Loaded {} file systems from {}", app.file_systems.len(), app.current_region);
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Error loading file systems: {}", e);
                                        log_message(&app_log, LogLevel::Error, format!("Error loading file systems: {}", e));
                                    }
                                }
                                app.is_loading = false;
                            }
                        }
                        KeyCode::Backspace => {
                            app.region_filter.pop();
                            app.update_region_filter();
                        }
                        KeyCode::Char(c) => {
                            app.region_filter.push(c);
                            app.update_region_filter();
                        }
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => {
                        log_message(&app_log, LogLevel::Info, "Application shutting down".to_string());
                        break;
                    }
                    KeyCode::Char('g') => {
                        if !app.show_help && !app.is_loading {
                            app.open_region_selector();
                        }
                    }
                    KeyCode::Char('h') => {
                        app.show_help = !app.show_help;
                    }
                    KeyCode::Char('l') => {
                        app.show_logs = !app.show_logs;
                    }
                    KeyCode::Char('w') => {
                        if app.show_logs {
                            app.log_wrap_enabled = !app.log_wrap_enabled;
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if !app.show_help {
                            app.next_item();
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if !app.show_help {
                            app.previous_item();
                        }
                    }
                    KeyCode::Left => {
                        if !app.show_help && !app.is_loading {
                            app.previous_region();
                            app.is_loading = true;
                            terminal.draw(|f| ui(f, &mut app))?;
                            
                            match load_file_systems(&app.current_region).await {
                                Ok(file_systems) => {
                                    app.file_systems = file_systems;
                                    app.selected_file_systems = vec![false; app.file_systems.len()];
                                    if !app.file_systems.is_empty() {
                                        app.fs_table_state.select(Some(0));
                                    } else {
                                        app.fs_table_state.select(None);
                                    }
                                    app.status_message = format!("Loaded {} file systems from {}", app.file_systems.len(), app.current_region);
                                }
                                Err(e) => {
                                    app.status_message = format!("Error loading file systems: {}", e);
                                    log_message(&app_log, LogLevel::Error, format!("Error loading file systems: {}", e));
                                }
                            }
                            app.is_loading = false;
                        }
                    }
                    KeyCode::Right => {
                        if !app.show_help && !app.is_loading {
                            app.next_region();
                            app.is_loading = true;
                            terminal.draw(|f| ui(f, &mut app))?;
                            
                            match load_file_systems(&app.current_region).await {
                                Ok(file_systems) => {
                                    app.file_systems = file_systems;
                                    app.selected_file_systems = vec![false; app.file_systems.len()];
                                    if !app.file_systems.is_empty() {
                                        app.fs_table_state.select(Some(0));
                                    } else {
                                        app.fs_table_state.select(None);
                                    }
                                    app.status_message = format!("Loaded {} file systems from {}", app.file_systems.len(), app.current_region);
                                }
                                Err(e) => {
                                    app.status_message = format!("Error loading file systems: {}", e);
                                    log_message(&app_log, LogLevel::Error, format!("Error loading file systems: {}", e));
                                }
                            }
                            app.is_loading = false;
                        }
                    }
                    KeyCode::Char(' ') => {
                        if !app.show_help {
                            app.toggle_selection();
                        }
                    }
                    KeyCode::Char('r') => {
                        if !app.show_help && !app.is_loading {
                            app.is_loading = true;
                            terminal.draw(|f| ui(f, &mut app))?;
                            
                            match app.view_mode {
                                ViewMode::FileSystemList => {
                                    match load_file_systems(&app.current_region).await {
                                        Ok(file_systems) => {
                                            app.file_systems = file_systems;
                                            app.selected_file_systems = vec![false; app.file_systems.len()];
                                            if !app.file_systems.is_empty() && app.fs_table_state.selected().is_none() {
                                                app.fs_table_state.select(Some(0));
                                            }
                                            app.status_message = format!("Refreshed {} file systems from {}", app.file_systems.len(), app.current_region);
                                        }
                                        Err(e) => {
                                            app.status_message = format!("Error refreshing file systems: {}", e);
                                            log_message(&app_log, LogLevel::Error, format!("Error refreshing file systems: {}", e));
                                        }
                                    }
                                }
                                ViewMode::MountTargetList => {
                                    if let Some(fs_id) = &app.selected_fs_for_mount_targets {
                                        match load_mount_targets(&app.current_region, fs_id).await {
                                            Ok(mount_targets) => {
                                                app.mount_targets = mount_targets;
                                                app.selected_mount_targets = vec![false; app.mount_targets.len()];
                                                if !app.mount_targets.is_empty() && app.mt_table_state.selected().is_none() {
                                                    app.mt_table_state.select(Some(0));
                                                }
                                                app.status_message = format!("Refreshed {} mount targets", app.mount_targets.len());
                                            }
                                            Err(e) => {
                                                app.status_message = format!("Error refreshing mount targets: {}", e);
                                                log_message(&app_log, LogLevel::Error, format!("Error refreshing mount targets: {}", e));
                                            }
                                        }
                                    }
                                }
                            }
                            app.is_loading = false;
                        }
                    }
                    KeyCode::Char('c') => {
                        if !app.show_help {
                            app.clear_selections();
                            app.status_message = "Cleared all selections".to_string();
                        }
                    }
                    KeyCode::Char('d') => {
                        if !app.show_help && !app.is_loading {
                            match app.view_mode {
                                ViewMode::FileSystemList => {
                                    let fs_ids = app.get_selected_fs_ids();
                                    if !fs_ids.is_empty() {
                                        app.status_message = "Deleting file systems...".to_string();
                                        terminal.draw(|f| ui(f, &mut app))?;
                                        
                                        match delete_file_systems(&app.current_region, fs_ids, &app_log).await {
                                            Ok(msg) => {
                                                app.status_message = msg;
                                                app.clear_selections();
                                                
                                                // Refresh the list
                                                match load_file_systems(&app.current_region).await {
                                                    Ok(file_systems) => {
                                                        app.file_systems = file_systems;
                                                        app.selected_file_systems = vec![false; app.file_systems.len()];
                                                        if !app.file_systems.is_empty() && app.fs_table_state.selected().is_none() {
                                                            app.fs_table_state.select(Some(0));
                                                        }
                                                    }
                                                    Err(e) => {
                                                        log_message(&app_log, LogLevel::Error, format!("Error refreshing after delete: {}", e));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                app.status_message = format!("Error deleting file systems: {}", e);
                                            }
                                        }
                                    } else {
                                        app.status_message = "No file systems selected".to_string();
                                    }
                                }
                                ViewMode::MountTargetList => {
                                    let mt_ids = app.get_selected_mt_ids();
                                    if !mt_ids.is_empty() {
                                        app.status_message = "Deleting mount targets...".to_string();
                                        terminal.draw(|f| ui(f, &mut app))?;
                                        
                                        match delete_mount_targets(&app.current_region, mt_ids, &app_log).await {
                                            Ok(msg) => {
                                                app.status_message = msg;
                                                app.clear_selections();
                                                
                                                // Refresh the list
                                                if let Some(fs_id) = &app.selected_fs_for_mount_targets {
                                                    match load_mount_targets(&app.current_region, fs_id).await {
                                                        Ok(mount_targets) => {
                                                            app.mount_targets = mount_targets;
                                                            app.selected_mount_targets = vec![false; app.mount_targets.len()];
                                                            if !app.mount_targets.is_empty() && app.mt_table_state.selected().is_none() {
                                                                app.mt_table_state.select(Some(0));
                                                            }
                                                        }
                                                        Err(e) => {
                                                            log_message(&app_log, LogLevel::Error, format!("Error refreshing after delete: {}", e));
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                app.status_message = format!("Error deleting mount targets: {}", e);
                                            }
                                        }
                                    } else {
                                        app.status_message = "No mount targets selected".to_string();
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('m') => {
                        if !app.show_help && !app.is_loading {
                            if let ViewMode::FileSystemList = app.view_mode {
                                if let Some(selected_idx) = app.fs_table_state.selected() {
                                    if let Some(fs) = app.file_systems.get(selected_idx) {
                                        let fs_id = fs.file_system_id.clone();
                                        app.is_loading = true;
                                        terminal.draw(|f| ui(f, &mut app))?;
                                        
                                        match load_mount_targets(&app.current_region, &fs_id).await {
                                            Ok(mount_targets) => {
                                                app.view_mode = ViewMode::MountTargetList;
                                                app.selected_fs_for_mount_targets = Some(fs_id.clone());
                                                app.mount_targets = mount_targets;
                                                app.selected_mount_targets = vec![false; app.mount_targets.len()];
                                                if !app.mount_targets.is_empty() {
                                                    app.mt_table_state.select(Some(0));
                                                }
                                                app.status_message = format!("Loaded {} mount targets for {}", app.mount_targets.len(), fs_id);
                                            }
                                            Err(e) => {
                                                app.status_message = format!("Error loading mount targets: {}", e);
                                                log_message(&app_log, LogLevel::Error, format!("Error loading mount targets: {}", e));
                                            }
                                        }
                                        app.is_loading = false;
                                    }
                                } else {
                                    app.status_message = "No file system selected".to_string();
                                }
                            }
                        }
                    }
                    KeyCode::Char('b') => {
                        if !app.show_help {
                            if let ViewMode::MountTargetList = app.view_mode {
                                app.back_to_file_systems();
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
