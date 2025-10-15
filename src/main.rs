use anyhow::Result;
use arboard::Clipboard;
use aws_config::BehaviorVersion;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_ec2::types::Tag;
use aws_sdk_sts::Client as StsClient;
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
struct Ec2Instance {
    id: String,
    name: String,
    state: String,
    instance_type: String,
    public_ip: String,
    private_ip: String,
    ipv6: String,
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

struct App {
    instances: Vec<Ec2Instance>,
    selected_instances: Vec<bool>,
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
            instances: Vec::new(),
            selected_instances: Vec::new(),
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
                    // If regex is invalid, treat as literal string match
                    let filter_lower = self.region_filter.to_lowercase();
                    self.filtered_regions = self.available_regions
                        .iter()
                        .filter(|region| region.to_lowercase().contains(&filter_lower))
                        .cloned()
                        .collect();
                }
            }
        }
        
        // Reset selection if out of bounds
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
                    0 // Loop back to top
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
                    self.filtered_regions.len() - 1 // Loop to bottom
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
                
                // Update current region and region index
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
            
            // Get IPv6 address from network interfaces
            let ipv6 = instance
                .network_interfaces()
                .iter()
                .flat_map(|ni| ni.ipv6_addresses())
                .next()
                .and_then(|ipv6_addr| ipv6_addr.ipv6_address())
                .unwrap_or("N/A")
                .to_string();
            
            instances.push(Ec2Instance {
                id,
                name,
                state,
                instance_type,
                public_ip,
                private_ip,
                ipv6,
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

async fn create_ami_from_instances(region: &str, instances: &[Ec2Instance], selected_indices: &[usize], log: &AppLog) -> Result<String> {
    if selected_indices.is_empty() {
        let msg = "No instances selected for AMI creation".to_string();
        log_message(log, LogLevel::Warning, msg.clone());
        return Ok(msg);
    }

    log_message(log, LogLevel::Info, format!("Starting AMI creation for {} instance(s) in region {}", selected_indices.len(), region));

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;
    
    let ec2_client = Ec2Client::new(&config);
    let sts_client = StsClient::new(&config);
    
    // Get IAM identity information
    let caller_identity = match sts_client.get_caller_identity().send().await {
        Ok(identity) => identity,
        Err(e) => {
            let error_msg = format!("Failed to get IAM identity: {}", e);
            log_message(log, LogLevel::Error, error_msg.clone());
            return Err(anyhow::anyhow!(error_msg));
        }
    };
    let iam_identity = caller_identity.arn().unwrap_or("Unknown");
    log_message(log, LogLevel::Info, format!("IAM Identity: {}", iam_identity));
    
    // Get current system username
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "Unknown".to_string());
    log_message(log, LogLevel::Info, format!("System username: {}", username));
    
    // Get current timestamp
    let timestamp = Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    
    let mut ami_ids = Vec::new();
    
    for &idx in selected_indices {
        if let Some(instance) = instances.get(idx) {
            log_message(log, LogLevel::Info, format!("Processing instance: {} ({})", instance.name, instance.id));
            
            // Get instance details to retrieve block device mappings
            let instance_details = match ec2_client
                .describe_instances()
                .instance_ids(&instance.id)
                .send()
                .await {
                    Ok(details) => details,
                    Err(e) => {
                        let error_msg = format!("Failed to describe instance {}: {}", instance.id, e);
                        log_message(log, LogLevel::Error, error_msg.clone());
                        return Err(anyhow::anyhow!(error_msg));
                    }
                };
            
            // Extract block device mappings and filter out ephemeral disks
            let mut block_device_mappings = Vec::new();
            
            for reservation in instance_details.reservations() {
                for inst in reservation.instances() {
                    if inst.instance_id() == Some(&instance.id) {
                        for bdm in inst.block_device_mappings() {
                            // Only include EBS volumes, exclude ephemeral disks (instance store)
                            if bdm.ebs().is_some() {
                                if let Some(device_name) = bdm.device_name() {
                                    log_message(log, LogLevel::Info, format!("  Including EBS volume: {}", device_name));
                                    
                                    // For AMI creation, we just need to specify the device name
                                    // AWS will automatically capture the EBS volume configuration
                                    let block_device = aws_sdk_ec2::types::EbsBlockDevice::builder().delete_on_termination(true).build();
                                    let mapping = aws_sdk_ec2::types::BlockDeviceMapping::builder()
                                        .device_name(device_name)
                                        .ebs(block_device)
                                        .build();
                                    block_device_mappings.push(mapping);
                                }
                            } else if let Some(device_name) = bdm.device_name() {
                                log_message(log, LogLevel::Info, format!("  Excluding ephemeral disk: {}", device_name));
                            }
                        }
                        break;
                    }
                }
            }
            
            log_message(log, LogLevel::Info, format!("  Total EBS volumes to include: {}", block_device_mappings.len()));
            
            // Create AMI name
            let ami_name = format!("{}_AMI_{}", instance.name, timestamp);
            log_message(log, LogLevel::Info, format!("  AMI name: {}", ami_name));
            
            // Create tags for the AMI
            let tags = vec![
                Tag::builder()
                    .key("SourceInstanceName")
                    .value(&instance.name)
                    .build(),
                Tag::builder()
                    .key("SourceInstanceId")
                    .value(&instance.id)
                    .build(),
                Tag::builder()
                    .key("CreatedBy")
                    .value(&username)
                    .build(),
                Tag::builder()
                    .key("CreatedAt")
                    .value(&timestamp)
                    .build(),
                Tag::builder()
                    .key("IAMIdentity")
                    .value(iam_identity)
                    .build(),
            ];
            
            log_message(log, LogLevel::Info, format!("  AMI Configuration:"));
            log_message(log, LogLevel::Info, format!("    - Instance ID: {}", instance.id));
            log_message(log, LogLevel::Info, format!("    - Instance Name: {}", instance.name));
            log_message(log, LogLevel::Info, format!("    - Instance Type: {}", instance.instance_type));
            log_message(log, LogLevel::Info, format!("    - Region: {}", region));
            log_message(log, LogLevel::Info, format!("    - Created By: {}", username));
            log_message(log, LogLevel::Info, format!("    - IAM Identity: {}", iam_identity));
            log_message(log, LogLevel::Info, format!("    - Timestamp: {}", timestamp));
            
            // Create the AMI with filtered block device mappings
            let mut create_image_request = ec2_client
                .create_image()
                .instance_id(&instance.id)
                .name(&ami_name)
                .description(format!(
                    "AMI created from instance {} ({}) by {} at {}",
                    instance.name, instance.id, username, timestamp
                ))
                .set_tag_specifications(Some(vec![
                    aws_sdk_ec2::types::TagSpecification::builder()
                        .resource_type(aws_sdk_ec2::types::ResourceType::Image)
                        .set_tags(Some(tags))
                        .build(),
                ]));
            
            // Only set block device mappings if we found any EBS volumes
            if !block_device_mappings.is_empty() {
                create_image_request = create_image_request.set_block_device_mappings(Some(block_device_mappings));
                log_message(log, LogLevel::Info, format!("  Added block device mappings..."));
            }
            
            log_message(log, LogLevel::Info, format!("  Sending CreateImage request to AWS..."));
            
            let response = match create_image_request.send().await {
                Ok(resp) => resp,
                Err(e) => {
                    let error_msg = format!("Failed to create AMI for instance {}: {}", instance.id, e);
                    log_message(log, LogLevel::Error, error_msg.clone());
                    log_message(log, LogLevel::Error, format!("  Error details: {:?}", e));
                    return Err(anyhow::anyhow!(error_msg));
                }
            };
            
            if let Some(ami_id) = response.image_id() {
                log_message(log, LogLevel::Info, format!("  Successfully created AMI: {}", ami_id));
                ami_ids.push(ami_id.to_string());
            } else {
                let error_msg = format!("AMI creation succeeded but no AMI ID returned for instance {}", instance.id);
                log_message(log, LogLevel::Warning, error_msg);
            }
        }
    }
    
    let success_msg = format!("Created {} AMI(s): {}", ami_ids.len(), ami_ids.join(", "));
    log_message(log, LogLevel::Info, success_msg.clone());
    Ok(success_msg)
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

    // Loading overlay (highest priority)
    if app.is_loading {
        let loading_area = centered_rect(40, 20, f.area());
        
        let loading_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("⏳ Loading instances...", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
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
        
        // Create the region selector UI
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
        
        // Split the area to add instructions at the bottom
        let selector_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(5),
            ])
            .split(selector_area);
        
        f.render_stateful_widget(region_list, selector_chunks[0], &mut app.region_selector_state);
        
        // Instructions
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
                    // Calculate available width for message (subtract prefix length and borders)
                    let available_width = (log_area.width as usize).saturating_sub(prefix.len() + 4);
                    
                    if available_width > 0 {
                        // Split message into chunks that fit the available width
                        let mut lines = Vec::new();
                        let message_chars: Vec<char> = entry.message.chars().collect();
                        let mut start = 0;
                        
                        while start < message_chars.len() {
                            let end = (start + available_width).min(message_chars.len());
                            let chunk: String = message_chars[start..end].iter().collect();
                            
                            if start == 0 {
                                // First line includes the prefix
                                lines.push(ListItem::new(Line::from(vec![
                                    Span::styled(format!("[{}] ", entry.timestamp), Style::default().fg(Color::DarkGray)),
                                    Span::styled(format!("{:5} ", level_text), level_style.add_modifier(Modifier::BOLD)),
                                    Span::raw(chunk),
                                ])));
                            } else {
                                // Continuation lines are indented
                                lines.push(ListItem::new(Line::from(vec![
                                    Span::raw(" ".repeat(prefix.len())),
                                    Span::raw(chunk),
                                ])));
                            }
                            
                            start = end;
                        }
                        lines
                    } else {
                        // Fallback if width calculation fails
                        vec![ListItem::new(Line::from(vec![
                            Span::styled(format!("[{}] ", entry.timestamp), Style::default().fg(Color::DarkGray)),
                            Span::styled(format!("{:5} ", level_text), level_style.add_modifier(Modifier::BOLD)),
                            Span::raw(&entry.message),
                        ]))]
                    }
                } else {
                    // No wrapping - single line
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
            Line::from("  ↑/↓ or j/k    - Navigate instances"),
            Line::from("  Space         - Toggle instance selection"),
            Line::from("  ←/→           - Switch regions"),
            Line::from("  g             - Select region (with filter)"),
            Line::from("  r             - Refresh instance list"),
            Line::from("  s             - Start selected instances"),
            Line::from("  t             - Stop selected instances"),
            Line::from("  d             - Terminate selected instances"),
            Line::from("  a             - Create AMI from selected instances"),
            Line::from("  c             - Clear all selections"),
            Line::from("  Ctrl+4        - Copy IPv4 address to clipboard"),
            Line::from("  Ctrl+6        - Copy IPv6 address to clipboard"),
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
        .row_highlight_style(selected_style)
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

    // Create app state with shared log
    let app_log: AppLog = Arc::new(Mutex::new(Vec::new()));
    let mut app = App::new(app_log.clone());
    
    // Log application startup
    log_message(&app_log, LogLevel::Info, "EC2 TUI application started".to_string());
    
    // Load initial instances
    app.is_loading = true;
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
    app.is_loading = false;

    // Main loop
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }
                
                // Handle CTRL+4 to copy IPv4 address
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('4') {
                    if let Some(selected_idx) = app.table_state.selected() {
                        if let Some(instance) = app.instances.get(selected_idx) {
                            if instance.public_ip != "N/A" {
                                match Clipboard::new().and_then(|mut clipboard| clipboard.set_text(&instance.public_ip)) {
                                    Ok(_) => {
                                        app.status_message = format!("Copied IPv4 address: {}", instance.public_ip);
                                        log_message(&app.log, LogLevel::Info, format!("Copied IPv4 address {} to clipboard", instance.public_ip));
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Failed to copy to clipboard: {}", e);
                                        log_message(&app.log, LogLevel::Error, format!("Clipboard error: {}", e));
                                    }
                                }
                            } else {
                                app.status_message = "No IPv4 address available for this instance".to_string();
                            }
                        }
                    } else {
                        app.status_message = "No instance selected".to_string();
                    }
                    continue;
                }
                
                // Handle CTRL+6 to copy IPv6 address
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('6') {
                    if let Some(selected_idx) = app.table_state.selected() {
                        if let Some(instance) = app.instances.get(selected_idx) {
                            if instance.ipv6 != "N/A" {
                                match Clipboard::new().and_then(|mut clipboard| clipboard.set_text(&instance.ipv6)) {
                                    Ok(_) => {
                                        app.status_message = format!("Copied IPv6 address: {}", instance.ipv6);
                                        log_message(&app.log, LogLevel::Info, format!("Copied IPv6 address {} to clipboard", instance.ipv6));
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Failed to copy to clipboard: {}", e);
                                        log_message(&app.log, LogLevel::Error, format!("Clipboard error: {}", e));
                                    }
                                }
                            } else {
                                app.status_message = "No IPv6 address available for this instance".to_string();
                            }
                        }
                    } else {
                        app.status_message = "No instance selected".to_string();
                    }
                    continue;
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
                                
                                match load_instances(&selected_region).await {
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
                        log_message(&app.log, LogLevel::Info, "Application shutting down".to_string());
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
                            app.next_instance();
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if !app.show_help {
                            app.previous_instance();
                        }
                    }
                    KeyCode::Left => {
                        if !app.show_help && !app.is_loading {
                            app.previous_region();
                            app.is_loading = true;
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
                            app.is_loading = false;
                        }
                    }
                    KeyCode::Right => {
                        if !app.show_help && !app.is_loading {
                            app.next_region();
                            app.is_loading = true;
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
                            app.is_loading = false;
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
                    KeyCode::Char('a') => {
                        if !app.show_help {
                            let selected_indices: Vec<usize> = app.selected_instances
                                .iter()
                                .enumerate()
                                .filter(|(_, &selected)| selected)
                                .map(|(i, _)| i)
                                .collect();
                            
                            if !selected_indices.is_empty() {
                                app.status_message = "Creating AMI(s)...".to_string();
                                terminal.draw(|f| ui(f, &mut app))?;
                                
                                match create_ami_from_instances(&app.current_region, &app.instances, &selected_indices, &app.log).await {
                                    Ok(msg) => {
                                        app.status_message = msg;
                                        app.clear_selections();
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Error creating AMI: {}", e);
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
