use anyhow::Result;
use arboard::Clipboard;
use aws_config::BehaviorVersion;
use aws_sdk_cloudformation::Client as CfnClient;
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
struct CfnStack {
    name: String,
    status: String,
    creation_time: String,
    description: String,
}

#[derive(Clone)]
struct StackEventInfo {
    timestamp: String,
    resource_type: String,
    logical_id: String,
    status: String,
    status_reason: String,
}

#[derive(Clone)]
struct StackResourceInfo {
    logical_id: String,
    physical_id: String,
    resource_type: String,
    status: String,
}

#[derive(Clone)]
struct StackExportInfo {
    name: String,
    value: String,
    exporting_stack: String,
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

#[derive(PartialEq)]
enum ViewMode {
    StackList,
    StackEvents,
    StackResources,
    StackExports,
}

struct App {
    stacks: Vec<CfnStack>,
    selected_stack_index: Option<usize>,
    table_state: TableState,
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
    view_mode: ViewMode,
    stack_events: Vec<StackEventInfo>,
    events_scroll_state: ListState,
    stack_resources: Vec<StackResourceInfo>,
    resources_table_state: TableState,
    stack_exports: Vec<StackExportInfo>,
    exports_table_state: TableState,
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
            stacks: Vec::new(),
            selected_stack_index: None,
            table_state: TableState::default(),
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
            view_mode: ViewMode::StackList,
            stack_events: Vec::new(),
            events_scroll_state: ListState::default(),
            stack_resources: Vec::new(),
            resources_table_state: TableState::default(),
            stack_exports: Vec::new(),
            exports_table_state: TableState::default(),
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

    fn next_stack(&mut self) {
        if self.stacks.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.stacks.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.selected_stack_index = Some(i);
    }

    fn previous_stack(&mut self) {
        if self.stacks.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.stacks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
        self.selected_stack_index = Some(i);
    }

    fn next_event(&mut self) {
        if self.stack_events.is_empty() {
            return;
        }
        let i = match self.events_scroll_state.selected() {
            Some(i) => {
                if i >= self.stack_events.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.events_scroll_state.select(Some(i));
    }

    fn previous_event(&mut self) {
        if self.stack_events.is_empty() {
            return;
        }
        let i = match self.events_scroll_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.stack_events.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.events_scroll_state.select(Some(i));
    }

    fn next_resource(&mut self) {
        if self.stack_resources.is_empty() {
            return;
        }
        let i = match self.resources_table_state.selected() {
            Some(i) => {
                if i >= self.stack_resources.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.resources_table_state.select(Some(i));
    }

    fn previous_resource(&mut self) {
        if self.stack_resources.is_empty() {
            return;
        }
        let i = match self.resources_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.stack_resources.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.resources_table_state.select(Some(i));
    }

    fn next_export(&mut self) {
        if self.stack_exports.is_empty() {
            return;
        }
        let i = match self.exports_table_state.selected() {
            Some(i) => {
                if i >= self.stack_exports.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.exports_table_state.select(Some(i));
    }

    fn previous_export(&mut self) {
        if self.stack_exports.is_empty() {
            return;
        }
        let i = match self.exports_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.stack_exports.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.exports_table_state.select(Some(i));
    }

    fn get_selected_stack_name(&self) -> Option<String> {
        self.selected_stack_index
            .and_then(|i| self.stacks.get(i))
            .map(|stack| stack.name.clone())
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

    fn return_to_stack_list(&mut self) {
        self.view_mode = ViewMode::StackList;
        self.stack_events.clear();
        self.stack_resources.clear();
        self.stack_exports.clear();
    }
}

async fn load_stacks(region: &str) -> Result<Vec<CfnStack>> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = CfnClient::new(&config);
    
    let resp = client.describe_stacks().send().await?;
    
    let mut stacks = Vec::new();
    
    for stack in resp.stacks() {
            let name = stack.stack_name().unwrap_or("N/A").to_string();
            
            let status = stack
                .stack_status()
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "Unknown".to_string());
            
            let creation_time = stack
                .creation_time()
                .map(|dt| {
                    dt.fmt(aws_sdk_cloudformation::primitives::DateTimeFormat::DateTime)
                        .unwrap_or_else(|_| "N/A".to_string())
                })
                .unwrap_or_else(|| "N/A".to_string());
            
            let description = stack
                .description()
                .unwrap_or("N/A")
                .to_string();
            
            stacks.push(CfnStack {
                name,
                status,
                creation_time,
                description,
            });
    }
    
    Ok(stacks)
}

async fn delete_stack(region: &str, stack_name: &str, log: &AppLog) -> Result<String> {
    log_message(log, LogLevel::Info, format!("Deleting stack: {}", stack_name));
    
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = CfnClient::new(&config);
    
    client
        .delete_stack()
        .stack_name(stack_name)
        .send()
        .await?;
    
    let msg = format!("Stack deletion initiated: {}", stack_name);
    log_message(log, LogLevel::Info, msg.clone());
    Ok(msg)
}

async fn load_stack_events(region: &str, stack_name: &str) -> Result<Vec<StackEventInfo>> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = CfnClient::new(&config);
    
    let resp = client
        .describe_stack_events()
        .stack_name(stack_name)
        .send()
        .await?;
    
    let mut events = Vec::new();
    
    for event in resp.stack_events() {
            let timestamp = event
                .timestamp()
                .map(|dt| {
                    dt.fmt(aws_sdk_cloudformation::primitives::DateTimeFormat::DateTime)
                        .unwrap_or_else(|_| "N/A".to_string())
                })
                .unwrap_or_else(|| "N/A".to_string());
            
            let resource_type = event
                .resource_type()
                .unwrap_or("N/A")
                .to_string();
            
            let logical_id = event
                .logical_resource_id()
                .unwrap_or("N/A")
                .to_string();
            
            let status = event
                .resource_status()
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "Unknown".to_string());
            
            let status_reason = event
                .resource_status_reason()
                .unwrap_or("")
                .to_string();
            
            events.push(StackEventInfo {
                timestamp,
                resource_type,
                logical_id,
                status,
                status_reason,
            });
    }
    
    Ok(events)
}

async fn load_stack_resources(region: &str, stack_name: &str) -> Result<Vec<StackResourceInfo>> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = CfnClient::new(&config);
    
    let resp = client
        .describe_stack_resources()
        .stack_name(stack_name)
        .send()
        .await?;
    
    let mut resources = Vec::new();
    
    for resource in resp.stack_resources() {
            let logical_id = resource
                .logical_resource_id()
                .unwrap_or("N/A")
                .to_string();
            
            let physical_id = resource
                .physical_resource_id()
                .unwrap_or("N/A")
                .to_string();
            
            let resource_type = resource
                .resource_type()
                .unwrap_or("N/A")
                .to_string();
            
            let status = resource
                .resource_status()
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "Unknown".to_string());
            
            resources.push(StackResourceInfo {
                logical_id,
                physical_id,
                resource_type,
                status,
            });
    }
    
    Ok(resources)
}

async fn get_stack_template(region: &str, stack_name: &str, log: &AppLog) -> Result<String> {
    log_message(log, LogLevel::Info, format!("Fetching template for stack: {}", stack_name));
    
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = CfnClient::new(&config);
    
    let resp = client
        .get_template()
        .stack_name(stack_name)
        .send()
        .await?;
    
    let template = resp
        .template_body()
        .unwrap_or("Template not available")
        .to_string();
    
    log_message(log, LogLevel::Info, format!("Template fetched successfully for stack: {}", stack_name));
    Ok(template)
}

async fn load_stack_exports(region: &str) -> Result<Vec<StackExportInfo>> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let client = CfnClient::new(&config);
    
    let resp = client.list_exports().send().await?;
    
    let mut exports = Vec::new();
    
    for export in resp.exports() {
            let name = export
                .name()
                .unwrap_or("N/A")
                .to_string();
            
            let value = export
                .value()
                .unwrap_or("N/A")
                .to_string();
            
            let exporting_stack = export
                .exporting_stack_id()
                .unwrap_or("N/A")
                .to_string();
            
            exports.push(StackExportInfo {
                name,
                value,
                exporting_stack,
            });
    }
    
    Ok(exports)
}

fn log_message(log: &AppLog, level: LogLevel, message: String) {
    if let Ok(mut log_entries) = log.lock() {
        log_entries.push(LogEntry::new(level, message));
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
        ViewMode::StackList => "Stack List",
        ViewMode::StackEvents => "Stack Events",
        ViewMode::StackResources => "Stack Resources",
        ViewMode::StackExports => "Stack Exports",
    };
    
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("CloudFormation Manager", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled(format!("Region: {}", app.current_region), Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(format!("View: {}", view_name), Style::default().fg(Color::Magenta)),
            Span::raw(" | "),
            Span::styled(format!("Stacks: {}", app.stacks.len()), Style::default().fg(Color::Green)),
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
            Line::from("  ←/→           - Switch regions"),
            Line::from("  g             - Select region (with filter)"),
            Line::from("  r             - Refresh stack list"),
            Line::from("  d             - Delete selected stack"),
            Line::from("  e             - View stack events"),
            Line::from("  s             - View stack resources"),
            Line::from("  x             - View stack exports"),
            Line::from("  t             - Copy template to clipboard"),
            Line::from("  Esc or b      - Back to stack list"),
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
            ViewMode::StackList => {
                let selected_style = Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD);
                
                let header_cells = ["Stack Name", "Status", "Created", "Description"]
                    .iter()
                    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                
                let header = Row::new(header_cells).height(1).bottom_margin(1);
                
                let rows = app.stacks.iter().map(|stack| {
                    let status_color = if stack.status.contains("COMPLETE") {
                        Color::Green
                    } else if stack.status.contains("FAILED") || stack.status.contains("ROLLBACK") {
                        Color::Red
                    } else if stack.status.contains("IN_PROGRESS") {
                        Color::Yellow
                    } else {
                        Color::White
                    };
                    
                    let cells = vec![
                        Cell::from(stack.name.clone()),
                        Cell::from(stack.status.clone()).style(Style::default().fg(status_color)),
                        Cell::from(stack.creation_time.clone()),
                        Cell::from(stack.description.clone()),
                    ];
                    
                    Row::new(cells).height(1)
                });
                
                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(30),
                        Constraint::Length(25),
                        Constraint::Length(20),
                        Constraint::Min(30),
                    ],
                )
                .header(header)
                .block(Block::default().borders(Borders::ALL).title("CloudFormation Stacks"))
                .row_highlight_style(selected_style)
                .highlight_symbol(">> ");
                
                f.render_stateful_widget(table, chunks[1], &mut app.table_state);
            }
            ViewMode::StackEvents => {
                let event_items: Vec<ListItem> = app.stack_events
                    .iter()
                    .map(|event| {
                        let status_color = if event.status.contains("COMPLETE") {
                            Color::Green
                        } else if event.status.contains("FAILED") {
                            Color::Red
                        } else if event.status.contains("IN_PROGRESS") {
                            Color::Yellow
                        } else {
                            Color::White
                        };
                        
                        let mut lines = vec![
                            Line::from(vec![
                                Span::styled(format!("[{}] ", event.timestamp), Style::default().fg(Color::DarkGray)),
                                Span::styled(&event.status, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                            ]),
                            Line::from(vec![
                                Span::raw("  Resource: "),
                                Span::styled(&event.logical_id, Style::default().fg(Color::Cyan)),
                                Span::raw(" ("),
                                Span::styled(&event.resource_type, Style::default().fg(Color::Yellow)),
                                Span::raw(")"),
                            ]),
                        ];
                        
                        if !event.status_reason.is_empty() {
                            lines.push(Line::from(vec![
                                Span::raw("  Reason: "),
                                Span::styled(&event.status_reason, Style::default().fg(Color::Magenta)),
                            ]));
                        }
                        
                        lines.push(Line::from(""));
                        
                        ListItem::new(lines)
                    })
                    .collect();
                
                let stack_name = app.get_selected_stack_name().unwrap_or_else(|| "Unknown".to_string());
                let title = format!("Stack Events: {} (Press 'b' or Esc to go back)", stack_name);
                
                let event_list = List::new(event_items)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title(title))
                    .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
                    .highlight_symbol(">> ");
                
                f.render_stateful_widget(event_list, chunks[1], &mut app.events_scroll_state);
            }
            ViewMode::StackResources => {
                let selected_style = Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD);
                
                let header_cells = ["Logical ID", "Physical ID", "Type", "Status"]
                    .iter()
                    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                
                let header = Row::new(header_cells).height(1).bottom_margin(1);
                
                let rows = app.stack_resources.iter().map(|resource| {
                    let status_color = if resource.status.contains("COMPLETE") {
                        Color::Green
                    } else if resource.status.contains("FAILED") {
                        Color::Red
                    } else if resource.status.contains("IN_PROGRESS") {
                        Color::Yellow
                    } else {
                        Color::White
                    };
                    
                    let cells = vec![
                        Cell::from(resource.logical_id.clone()),
                        Cell::from(resource.physical_id.clone()),
                        Cell::from(resource.resource_type.clone()),
                        Cell::from(resource.status.clone()).style(Style::default().fg(status_color)),
                    ];
                    
                    Row::new(cells).height(1)
                });
                
                let stack_name = app.get_selected_stack_name().unwrap_or_else(|| "Unknown".to_string());
                let title = format!("Stack Resources: {} (Press 'b' or Esc to go back)", stack_name);
                
                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(30),
                        Constraint::Length(40),
                        Constraint::Length(30),
                        Constraint::Min(20),
                    ],
                )
                .header(header)
                .block(Block::default().borders(Borders::ALL).title(title))
                .row_highlight_style(selected_style)
                .highlight_symbol(">> ");
                
                f.render_stateful_widget(table, chunks[1], &mut app.resources_table_state);
            }
            ViewMode::StackExports => {
                let selected_style = Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD);
                
                let header_cells = ["Export Name", "Value", "Exporting Stack"]
                    .iter()
                    .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                
                let header = Row::new(header_cells).height(1).bottom_margin(1);
                
                let rows = app.stack_exports.iter().map(|export| {
                    let cells = vec![
                        Cell::from(export.name.clone()),
                        Cell::from(export.value.clone()),
                        Cell::from(export.exporting_stack.clone()),
                    ];
                    
                    Row::new(cells).height(1)
                });
                
                let title = "Stack Exports (Press 'b' or Esc to go back)";
                
                let table = Table::new(
                    rows,
                    [
                        Constraint::Length(40),
                        Constraint::Length(40),
                        Constraint::Min(40),
                    ],
                )
                .header(header)
                .block(Block::default().borders(Borders::ALL).title(title))
                .row_highlight_style(selected_style)
                .highlight_symbol(">> ");
                
                f.render_stateful_widget(table, chunks[1], &mut app.exports_table_state);
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
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state with shared log
    let app_log: AppLog = Arc::new(Mutex::new(Vec::new()));
    let mut app = App::new(app_log.clone());
    
    log_message(&app_log, LogLevel::Info, "CloudFormation TUI application started".to_string());
    
    // Load initial stacks
    app.is_loading = true;
    terminal.draw(|f| ui(f, &mut app))?;
    
    match load_stacks(&app.current_region).await {
        Ok(stacks) => {
            app.stacks = stacks;
            if !app.stacks.is_empty() {
                app.table_state.select(Some(0));
                app.selected_stack_index = Some(0);
            }
            app.status_message = format!("Loaded {} stacks from {}", app.stacks.len(), app.current_region);
        }
        Err(e) => {
            app.status_message = format!("Error loading stacks: {}", e);
            log_message(&app_log, LogLevel::Error, format!("Error loading stacks: {}", e));
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
                                
                                match load_stacks(&selected_region).await {
                                    Ok(stacks) => {
                                        app.stacks = stacks;
                                        if !app.stacks.is_empty() {
                                            app.table_state.select(Some(0));
                                            app.selected_stack_index = Some(0);
                                        } else {
                                            app.table_state.select(None);
                                            app.selected_stack_index = None;
                                        }
                                        app.status_message = format!("Loaded {} stacks from {}", app.stacks.len(), app.current_region);
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Error loading stacks: {}", e);
                                        log_message(&app_log, LogLevel::Error, format!("Error loading stacks: {}", e));
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
                        if !app.show_help && !app.is_loading && app.view_mode == ViewMode::StackList {
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
                            match app.view_mode {
                                ViewMode::StackList => app.next_stack(),
                                ViewMode::StackEvents => app.next_event(),
                                ViewMode::StackResources => app.next_resource(),
                                ViewMode::StackExports => app.next_export(),
                            }
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if !app.show_help {
                            match app.view_mode {
                                ViewMode::StackList => app.previous_stack(),
                                ViewMode::StackEvents => app.previous_event(),
                                ViewMode::StackResources => app.previous_resource(),
                                ViewMode::StackExports => app.previous_export(),
                            }
                        }
                    }
                    KeyCode::Left => {
                        if !app.show_help && !app.is_loading && app.view_mode == ViewMode::StackList {
                            app.previous_region();
                            app.is_loading = true;
                            terminal.draw(|f| ui(f, &mut app))?;
                            
                            match load_stacks(&app.current_region).await {
                                Ok(stacks) => {
                                    app.stacks = stacks;
                                    if !app.stacks.is_empty() {
                                        app.table_state.select(Some(0));
                                        app.selected_stack_index = Some(0);
                                    } else {
                                        app.table_state.select(None);
                                        app.selected_stack_index = None;
                                    }
                                    app.status_message = format!("Loaded {} stacks from {}", app.stacks.len(), app.current_region);
                                }
                                Err(e) => {
                                    app.status_message = format!("Error loading stacks: {}", e);
                                    log_message(&app_log, LogLevel::Error, format!("Error loading stacks: {}", e));
                                }
                            }
                            app.is_loading = false;
                        }
                    }
                    KeyCode::Right => {
                        if !app.show_help && !app.is_loading && app.view_mode == ViewMode::StackList {
                            app.next_region();
                            app.is_loading = true;
                            terminal.draw(|f| ui(f, &mut app))?;
                            
                            match load_stacks(&app.current_region).await {
                                Ok(stacks) => {
                                    app.stacks = stacks;
                                    if !app.stacks.is_empty() {
                                        app.table_state.select(Some(0));
                                        app.selected_stack_index = Some(0);
                                    } else {
                                        app.table_state.select(None);
                                        app.selected_stack_index = None;
                                    }
                                    app.status_message = format!("Loaded {} stacks from {}", app.stacks.len(), app.current_region);
                                }
                                Err(e) => {
                                    app.status_message = format!("Error loading stacks: {}", e);
                                    log_message(&app_log, LogLevel::Error, format!("Error loading stacks: {}", e));
                                }
                            }
                            app.is_loading = false;
                        }
                    }
                    KeyCode::Char('r') => {
                        if !app.show_help && !app.is_loading && app.view_mode == ViewMode::StackList {
                            app.is_loading = true;
                            terminal.draw(|f| ui(f, &mut app))?;
                            
                            match load_stacks(&app.current_region).await {
                                Ok(stacks) => {
                                    app.stacks = stacks;
                                    if !app.stacks.is_empty() && app.table_state.selected().is_none() {
                                        app.table_state.select(Some(0));
                                        app.selected_stack_index = Some(0);
                                    }
                                    app.status_message = format!("Refreshed {} stacks from {}", app.stacks.len(), app.current_region);
                                }
                                Err(e) => {
                                    app.status_message = format!("Error refreshing stacks: {}", e);
                                    log_message(&app_log, LogLevel::Error, format!("Error refreshing stacks: {}", e));
                                }
                            }
                            app.is_loading = false;
                        }
                    }
                    KeyCode::Char('d') => {
                        if !app.show_help && app.view_mode == ViewMode::StackList {
                            if let Some(stack_name) = app.get_selected_stack_name() {
                                app.status_message = format!("Deleting stack: {}...", stack_name);
                                terminal.draw(|f| ui(f, &mut app))?;
                                
                                match delete_stack(&app.current_region, &stack_name, &app_log).await {
                                    Ok(msg) => {
                                        app.status_message = msg;
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Error deleting stack: {}", e);
                                        log_message(&app_log, LogLevel::Error, format!("Error deleting stack: {}", e));
                                    }
                                }
                            } else {
                                app.status_message = "No stack selected".to_string();
                            }
                        }
                    }
                    KeyCode::Char('e') => {
                        if !app.show_help && app.view_mode == ViewMode::StackList {
                            if let Some(stack_name) = app.get_selected_stack_name() {
                                app.is_loading = true;
                                terminal.draw(|f| ui(f, &mut app))?;
                                
                                match load_stack_events(&app.current_region, &stack_name).await {
                                    Ok(events) => {
                                        app.stack_events = events;
                                        app.view_mode = ViewMode::StackEvents;
                                        if !app.stack_events.is_empty() {
                                            app.events_scroll_state.select(Some(0));
                                        }
                                        app.status_message = format!("Loaded {} events for stack: {}", app.stack_events.len(), stack_name);
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Error loading stack events: {}", e);
                                        log_message(&app_log, LogLevel::Error, format!("Error loading stack events: {}", e));
                                    }
                                }
                                app.is_loading = false;
                            } else {
                                app.status_message = "No stack selected".to_string();
                            }
                        }
                    }
                    KeyCode::Char('s') => {
                        if !app.show_help && app.view_mode == ViewMode::StackList {
                            if let Some(stack_name) = app.get_selected_stack_name() {
                                app.is_loading = true;
                                terminal.draw(|f| ui(f, &mut app))?;
                                
                                match load_stack_resources(&app.current_region, &stack_name).await {
                                    Ok(resources) => {
                                        app.stack_resources = resources;
                                        app.view_mode = ViewMode::StackResources;
                                        if !app.stack_resources.is_empty() {
                                            app.resources_table_state.select(Some(0));
                                        }
                                        app.status_message = format!("Loaded {} resources for stack: {}", app.stack_resources.len(), stack_name);
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Error loading stack resources: {}", e);
                                        log_message(&app_log, LogLevel::Error, format!("Error loading stack resources: {}", e));
                                    }
                                }
                                app.is_loading = false;
                            } else {
                                app.status_message = "No stack selected".to_string();
                            }
                        }
                    }
                    KeyCode::Char('x') => {
                        if !app.show_help && app.view_mode == ViewMode::StackList {
                            app.is_loading = true;
                            terminal.draw(|f| ui(f, &mut app))?;
                            
                            match load_stack_exports(&app.current_region).await {
                                Ok(exports) => {
                                    app.stack_exports = exports;
                                    app.view_mode = ViewMode::StackExports;
                                    if !app.stack_exports.is_empty() {
                                        app.exports_table_state.select(Some(0));
                                    }
                                    app.status_message = format!("Loaded {} exports from region: {}", app.stack_exports.len(), app.current_region);
                                }
                                Err(e) => {
                                    app.status_message = format!("Error loading stack exports: {}", e);
                                    log_message(&app_log, LogLevel::Error, format!("Error loading stack exports: {}", e));
                                }
                            }
                            app.is_loading = false;
                        }
                    }
                    KeyCode::Char('t') => {
                        if !app.show_help && app.view_mode == ViewMode::StackList {
                            if let Some(stack_name) = app.get_selected_stack_name() {
                                app.status_message = format!("Fetching template for stack: {}...", stack_name);
                                terminal.draw(|f| ui(f, &mut app))?;
                                
                                match get_stack_template(&app.current_region, &stack_name, &app_log).await {
                                    Ok(template) => {
                                        match Clipboard::new().and_then(|mut clipboard| clipboard.set_text(&template)) {
                                            Ok(_) => {
                                                app.status_message = format!("Template copied to clipboard for stack: {}", stack_name);
                                                log_message(&app_log, LogLevel::Info, format!("Template copied to clipboard for stack: {}", stack_name));
                                            }
                                            Err(e) => {
                                                app.status_message = format!("Failed to copy template to clipboard: {}", e);
                                                log_message(&app_log, LogLevel::Error, format!("Clipboard error: {}", e));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Error fetching template: {}", e);
                                        log_message(&app_log, LogLevel::Error, format!("Error fetching template: {}", e));
                                    }
                                }
                            } else {
                                app.status_message = "No stack selected".to_string();
                            }
                        }
                    }
                    KeyCode::Char('b') | KeyCode::Esc => {
                        if app.show_help {
                            app.show_help = false;
                        } else if app.show_logs {
                            app.show_logs = false;
                        } else if app.view_mode != ViewMode::StackList {
                            app.return_to_stack_list();
                            app.status_message = "Returned to stack list".to_string();
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
