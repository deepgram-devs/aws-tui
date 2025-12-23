use anyhow::Result;
use arboard::Clipboard;
use aws_config::BehaviorVersion;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_ec2::types::Tag;
use aws_sdk_ec2::error::ProvideErrorMetadata;
use aws_sdk_ec2::operation::RequestId;
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
use std::time::Instant;
use rand::seq::SliceRandom;

#[derive(Clone)]
struct Ec2Instance {
    id: String,
    name: String,
    state: String,
    instance_type: String,
    public_ip: String,
    private_ip: String,
    ipv6: String,
    tags: Vec<(String, String)>,
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
    show_instance_filter: bool,
    instance_filter: String,
    filtered_instance_indices: Vec<usize>,
    animation_start: Instant,
    color_sequence: Vec<Color>,
    show_resize_dialog: bool,
    resize_instance_types: Vec<String>,
    resize_type_filter: String,
    filtered_resize_types: Vec<String>,
    resize_selector_state: ListState,
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

        // Create randomized color sequence for border animation
        let mut color_sequence = vec![
            Color::White,
            Color::DarkGray,
            Color::Cyan,      // Teal
            Color::Magenta,   // Purple
        ];
        let mut rng = rand::thread_rng();
        color_sequence.shuffle(&mut rng);

        // Comprehensive EC2 instance types list
        let resize_instance_types = vec![
            // General Purpose - T family (Burstable)
            "t1.micro",
            "t2.nano", "t2.micro", "t2.small", "t2.medium", "t2.large", "t2.xlarge", "t2.2xlarge",
            "t3.nano", "t3.micro", "t3.small", "t3.medium", "t3.large", "t3.xlarge", "t3.2xlarge",
            "t3a.nano", "t3a.micro", "t3a.small", "t3a.medium", "t3a.large", "t3a.xlarge", "t3a.2xlarge",
            "t4g.nano", "t4g.micro", "t4g.small", "t4g.medium", "t4g.large", "t4g.xlarge", "t4g.2xlarge",
            // General Purpose - M family
            "m1.small", "m1.medium", "m1.large", "m1.xlarge",
            "m2.xlarge", "m2.2xlarge", "m2.4xlarge",
            "m3.medium", "m3.large", "m3.xlarge", "m3.2xlarge",
            "m4.large", "m4.xlarge", "m4.2xlarge", "m4.4xlarge", "m4.10xlarge", "m4.16xlarge",
            "m5.large", "m5.xlarge", "m5.2xlarge", "m5.4xlarge", "m5.8xlarge", "m5.12xlarge", "m5.16xlarge", "m5.24xlarge", "m5.metal",
            "m5a.large", "m5a.xlarge", "m5a.2xlarge", "m5a.4xlarge", "m5a.8xlarge", "m5a.12xlarge", "m5a.16xlarge", "m5a.24xlarge",
            "m5ad.large", "m5ad.xlarge", "m5ad.2xlarge", "m5ad.4xlarge", "m5ad.8xlarge", "m5ad.12xlarge", "m5ad.16xlarge", "m5ad.24xlarge",
            "m5d.large", "m5d.xlarge", "m5d.2xlarge", "m5d.4xlarge", "m5d.8xlarge", "m5d.12xlarge", "m5d.16xlarge", "m5d.24xlarge", "m5d.metal",
            "m5dn.large", "m5dn.xlarge", "m5dn.2xlarge", "m5dn.4xlarge", "m5dn.8xlarge", "m5dn.12xlarge", "m5dn.16xlarge", "m5dn.24xlarge", "m5dn.metal",
            "m5n.large", "m5n.xlarge", "m5n.2xlarge", "m5n.4xlarge", "m5n.8xlarge", "m5n.12xlarge", "m5n.16xlarge", "m5n.24xlarge", "m5n.metal",
            "m5zn.large", "m5zn.xlarge", "m5zn.2xlarge", "m5zn.3xlarge", "m5zn.6xlarge", "m5zn.12xlarge", "m5zn.metal",
            "m6a.large", "m6a.xlarge", "m6a.2xlarge", "m6a.4xlarge", "m6a.8xlarge", "m6a.12xlarge", "m6a.16xlarge", "m6a.24xlarge", "m6a.32xlarge", "m6a.48xlarge", "m6a.metal",
            "m6g.medium", "m6g.large", "m6g.xlarge", "m6g.2xlarge", "m6g.4xlarge", "m6g.8xlarge", "m6g.12xlarge", "m6g.16xlarge", "m6g.metal",
            "m6gd.medium", "m6gd.large", "m6gd.xlarge", "m6gd.2xlarge", "m6gd.4xlarge", "m6gd.8xlarge", "m6gd.12xlarge", "m6gd.16xlarge", "m6gd.metal",
            "m6i.large", "m6i.xlarge", "m6i.2xlarge", "m6i.4xlarge", "m6i.8xlarge", "m6i.12xlarge", "m6i.16xlarge", "m6i.24xlarge", "m6i.32xlarge", "m6i.metal",
            "m6id.large", "m6id.xlarge", "m6id.2xlarge", "m6id.4xlarge", "m6id.8xlarge", "m6id.12xlarge", "m6id.16xlarge", "m6id.24xlarge", "m6id.32xlarge", "m6id.metal",
            "m6idn.large", "m6idn.xlarge", "m6idn.2xlarge", "m6idn.4xlarge", "m6idn.8xlarge", "m6idn.12xlarge", "m6idn.16xlarge", "m6idn.24xlarge", "m6idn.32xlarge", "m6idn.metal",
            "m6in.large", "m6in.xlarge", "m6in.2xlarge", "m6in.4xlarge", "m6in.8xlarge", "m6in.12xlarge", "m6in.16xlarge", "m6in.24xlarge", "m6in.32xlarge", "m6in.metal",
            "m7a.medium", "m7a.large", "m7a.xlarge", "m7a.2xlarge", "m7a.4xlarge", "m7a.8xlarge", "m7a.12xlarge", "m7a.16xlarge", "m7a.24xlarge", "m7a.32xlarge", "m7a.48xlarge", "m7a.metal-48xl",
            "m7g.medium", "m7g.large", "m7g.xlarge", "m7g.2xlarge", "m7g.4xlarge", "m7g.8xlarge", "m7g.12xlarge", "m7g.16xlarge", "m7g.metal",
            "m7gd.medium", "m7gd.large", "m7gd.xlarge", "m7gd.2xlarge", "m7gd.4xlarge", "m7gd.8xlarge", "m7gd.12xlarge", "m7gd.16xlarge", "m7gd.metal",
            "m7i.large", "m7i.xlarge", "m7i.2xlarge", "m7i.4xlarge", "m7i.8xlarge", "m7i.12xlarge", "m7i.16xlarge", "m7i.24xlarge", "m7i.48xlarge", "m7i.metal-24xl", "m7i.metal-48xl",
            "m7i-flex.large", "m7i-flex.xlarge", "m7i-flex.2xlarge", "m7i-flex.4xlarge", "m7i-flex.8xlarge",
            "mac1.metal", "mac2.metal", "mac2-m2.metal", "mac2-m2pro.metal",
            // Compute Optimized - C family
            "c1.medium", "c1.xlarge",
            "c3.large", "c3.xlarge", "c3.2xlarge", "c3.4xlarge", "c3.8xlarge",
            "c4.large", "c4.xlarge", "c4.2xlarge", "c4.4xlarge", "c4.8xlarge",
            "c5.large", "c5.xlarge", "c5.2xlarge", "c5.4xlarge", "c5.9xlarge", "c5.12xlarge", "c5.18xlarge", "c5.24xlarge", "c5.metal",
            "c5a.large", "c5a.xlarge", "c5a.2xlarge", "c5a.4xlarge", "c5a.8xlarge", "c5a.12xlarge", "c5a.16xlarge", "c5a.24xlarge",
            "c5ad.large", "c5ad.xlarge", "c5ad.2xlarge", "c5ad.4xlarge", "c5ad.8xlarge", "c5ad.12xlarge", "c5ad.16xlarge", "c5ad.24xlarge",
            "c5d.large", "c5d.xlarge", "c5d.2xlarge", "c5d.4xlarge", "c5d.9xlarge", "c5d.12xlarge", "c5d.18xlarge", "c5d.24xlarge", "c5d.metal",
            "c5n.large", "c5n.xlarge", "c5n.2xlarge", "c5n.4xlarge", "c5n.9xlarge", "c5n.18xlarge", "c5n.metal",
            "c6a.large", "c6a.xlarge", "c6a.2xlarge", "c6a.4xlarge", "c6a.8xlarge", "c6a.12xlarge", "c6a.16xlarge", "c6a.24xlarge", "c6a.32xlarge", "c6a.48xlarge", "c6a.metal",
            "c6g.medium", "c6g.large", "c6g.xlarge", "c6g.2xlarge", "c6g.4xlarge", "c6g.8xlarge", "c6g.12xlarge", "c6g.16xlarge", "c6g.metal",
            "c6gd.medium", "c6gd.large", "c6gd.xlarge", "c6gd.2xlarge", "c6gd.4xlarge", "c6gd.8xlarge", "c6gd.12xlarge", "c6gd.16xlarge", "c6gd.metal",
            "c6gn.medium", "c6gn.large", "c6gn.xlarge", "c6gn.2xlarge", "c6gn.4xlarge", "c6gn.8xlarge", "c6gn.12xlarge", "c6gn.16xlarge",
            "c6i.large", "c6i.xlarge", "c6i.2xlarge", "c6i.4xlarge", "c6i.8xlarge", "c6i.12xlarge", "c6i.16xlarge", "c6i.24xlarge", "c6i.32xlarge", "c6i.metal",
            "c6id.large", "c6id.xlarge", "c6id.2xlarge", "c6id.4xlarge", "c6id.8xlarge", "c6id.12xlarge", "c6id.16xlarge", "c6id.24xlarge", "c6id.32xlarge", "c6id.metal",
            "c6in.large", "c6in.xlarge", "c6in.2xlarge", "c6in.4xlarge", "c6in.8xlarge", "c6in.12xlarge", "c6in.16xlarge", "c6in.24xlarge", "c6in.32xlarge", "c6in.metal",
            "c7a.medium", "c7a.large", "c7a.xlarge", "c7a.2xlarge", "c7a.4xlarge", "c7a.8xlarge", "c7a.12xlarge", "c7a.16xlarge", "c7a.24xlarge", "c7a.32xlarge", "c7a.48xlarge", "c7a.metal-48xl",
            "c7g.medium", "c7g.large", "c7g.xlarge", "c7g.2xlarge", "c7g.4xlarge", "c7g.8xlarge", "c7g.12xlarge", "c7g.16xlarge", "c7g.metal",
            "c7gd.medium", "c7gd.large", "c7gd.xlarge", "c7gd.2xlarge", "c7gd.4xlarge", "c7gd.8xlarge", "c7gd.12xlarge", "c7gd.16xlarge", "c7gd.metal",
            "c7gn.medium", "c7gn.large", "c7gn.xlarge", "c7gn.2xlarge", "c7gn.4xlarge", "c7gn.8xlarge", "c7gn.12xlarge", "c7gn.16xlarge", "c7gn.metal",
            "c7i.large", "c7i.xlarge", "c7i.2xlarge", "c7i.4xlarge", "c7i.8xlarge", "c7i.12xlarge", "c7i.16xlarge", "c7i.24xlarge", "c7i.48xlarge", "c7i.metal-24xl", "c7i.metal-48xl",
            // Memory Optimized - R family
            "r3.large", "r3.xlarge", "r3.2xlarge", "r3.4xlarge", "r3.8xlarge",
            "r4.large", "r4.xlarge", "r4.2xlarge", "r4.4xlarge", "r4.8xlarge", "r4.16xlarge",
            "r5.large", "r5.xlarge", "r5.2xlarge", "r5.4xlarge", "r5.8xlarge", "r5.12xlarge", "r5.16xlarge", "r5.24xlarge", "r5.metal",
            "r5a.large", "r5a.xlarge", "r5a.2xlarge", "r5a.4xlarge", "r5a.8xlarge", "r5a.12xlarge", "r5a.16xlarge", "r5a.24xlarge",
            "r5ad.large", "r5ad.xlarge", "r5ad.2xlarge", "r5ad.4xlarge", "r5ad.8xlarge", "r5ad.12xlarge", "r5ad.16xlarge", "r5ad.24xlarge",
            "r5b.large", "r5b.xlarge", "r5b.2xlarge", "r5b.4xlarge", "r5b.8xlarge", "r5b.12xlarge", "r5b.16xlarge", "r5b.24xlarge", "r5b.metal",
            "r5d.large", "r5d.xlarge", "r5d.2xlarge", "r5d.4xlarge", "r5d.8xlarge", "r5d.12xlarge", "r5d.16xlarge", "r5d.24xlarge", "r5d.metal",
            "r5dn.large", "r5dn.xlarge", "r5dn.2xlarge", "r5dn.4xlarge", "r5dn.8xlarge", "r5dn.12xlarge", "r5dn.16xlarge", "r5dn.24xlarge", "r5dn.metal",
            "r5n.large", "r5n.xlarge", "r5n.2xlarge", "r5n.4xlarge", "r5n.8xlarge", "r5n.12xlarge", "r5n.16xlarge", "r5n.24xlarge", "r5n.metal",
            "r6a.large", "r6a.xlarge", "r6a.2xlarge", "r6a.4xlarge", "r6a.8xlarge", "r6a.12xlarge", "r6a.16xlarge", "r6a.24xlarge", "r6a.32xlarge", "r6a.48xlarge", "r6a.metal",
            "r6g.medium", "r6g.large", "r6g.xlarge", "r6g.2xlarge", "r6g.4xlarge", "r6g.8xlarge", "r6g.12xlarge", "r6g.16xlarge", "r6g.metal",
            "r6gd.medium", "r6gd.large", "r6gd.xlarge", "r6gd.2xlarge", "r6gd.4xlarge", "r6gd.8xlarge", "r6gd.12xlarge", "r6gd.16xlarge", "r6gd.metal",
            "r6i.large", "r6i.xlarge", "r6i.2xlarge", "r6i.4xlarge", "r6i.8xlarge", "r6i.12xlarge", "r6i.16xlarge", "r6i.24xlarge", "r6i.32xlarge", "r6i.metal",
            "r6id.large", "r6id.xlarge", "r6id.2xlarge", "r6id.4xlarge", "r6id.8xlarge", "r6id.12xlarge", "r6id.16xlarge", "r6id.24xlarge", "r6id.32xlarge", "r6id.metal",
            "r6idn.large", "r6idn.xlarge", "r6idn.2xlarge", "r6idn.4xlarge", "r6idn.8xlarge", "r6idn.12xlarge", "r6idn.16xlarge", "r6idn.24xlarge", "r6idn.32xlarge", "r6idn.metal",
            "r6in.large", "r6in.xlarge", "r6in.2xlarge", "r6in.4xlarge", "r6in.8xlarge", "r6in.12xlarge", "r6in.16xlarge", "r6in.24xlarge", "r6in.32xlarge", "r6in.metal",
            "r7a.medium", "r7a.large", "r7a.xlarge", "r7a.2xlarge", "r7a.4xlarge", "r7a.8xlarge", "r7a.12xlarge", "r7a.16xlarge", "r7a.24xlarge", "r7a.32xlarge", "r7a.48xlarge", "r7a.metal-48xl",
            "r7g.medium", "r7g.large", "r7g.xlarge", "r7g.2xlarge", "r7g.4xlarge", "r7g.8xlarge", "r7g.12xlarge", "r7g.16xlarge", "r7g.metal",
            "r7gd.medium", "r7gd.large", "r7gd.xlarge", "r7gd.2xlarge", "r7gd.4xlarge", "r7gd.8xlarge", "r7gd.12xlarge", "r7gd.16xlarge", "r7gd.metal",
            "r7i.large", "r7i.xlarge", "r7i.2xlarge", "r7i.4xlarge", "r7i.8xlarge", "r7i.12xlarge", "r7i.16xlarge", "r7i.24xlarge", "r7i.48xlarge", "r7i.metal-24xl", "r7i.metal-48xl",
            "r7iz.large", "r7iz.xlarge", "r7iz.2xlarge", "r7iz.4xlarge", "r7iz.8xlarge", "r7iz.12xlarge", "r7iz.16xlarge", "r7iz.32xlarge", "r7iz.metal-16xl", "r7iz.metal-32xl",
            // Memory Optimized - X family (Extra large memory)
            "x1.16xlarge", "x1.32xlarge",
            "x1e.xlarge", "x1e.2xlarge", "x1e.4xlarge", "x1e.8xlarge", "x1e.16xlarge", "x1e.32xlarge",
            "x2gd.medium", "x2gd.large", "x2gd.xlarge", "x2gd.2xlarge", "x2gd.4xlarge", "x2gd.8xlarge", "x2gd.12xlarge", "x2gd.16xlarge", "x2gd.metal",
            "x2idn.16xlarge", "x2idn.24xlarge", "x2idn.32xlarge", "x2idn.metal",
            "x2iedn.xlarge", "x2iedn.2xlarge", "x2iedn.4xlarge", "x2iedn.8xlarge", "x2iedn.16xlarge", "x2iedn.24xlarge", "x2iedn.32xlarge", "x2iedn.metal",
            "x2iezn.2xlarge", "x2iezn.4xlarge", "x2iezn.6xlarge", "x2iezn.8xlarge", "x2iezn.12xlarge", "x2iezn.metal",
            // Memory Optimized - Z family (High frequency)
            "z1d.large", "z1d.xlarge", "z1d.2xlarge", "z1d.3xlarge", "z1d.6xlarge", "z1d.12xlarge", "z1d.metal",
            // Memory Optimized - U family (High memory)
            "u-3tb1.56xlarge", "u-6tb1.56xlarge", "u-6tb1.112xlarge", "u-6tb1.metal",
            "u-9tb1.112xlarge", "u-9tb1.metal", "u-12tb1.112xlarge", "u-12tb1.metal",
            "u-18tb1.112xlarge", "u-18tb1.metal", "u-24tb1.112xlarge", "u-24tb1.metal",
            // Storage Optimized - D family
            "d2.xlarge", "d2.2xlarge", "d2.4xlarge", "d2.8xlarge",
            "d3.xlarge", "d3.2xlarge", "d3.4xlarge", "d3.8xlarge",
            "d3en.xlarge", "d3en.2xlarge", "d3en.4xlarge", "d3en.6xlarge", "d3en.8xlarge", "d3en.12xlarge",
            // Storage Optimized - H family
            "h1.2xlarge", "h1.4xlarge", "h1.8xlarge", "h1.16xlarge",
            "hs1.8xlarge",
            // Storage Optimized - I family (High I/O)
            "i2.xlarge", "i2.2xlarge", "i2.4xlarge", "i2.8xlarge",
            "i3.large", "i3.xlarge", "i3.2xlarge", "i3.4xlarge", "i3.8xlarge", "i3.16xlarge", "i3.metal",
            "i3en.large", "i3en.xlarge", "i3en.2xlarge", "i3en.3xlarge", "i3en.6xlarge", "i3en.12xlarge", "i3en.24xlarge", "i3en.metal",
            "i4g.large", "i4g.xlarge", "i4g.2xlarge", "i4g.4xlarge", "i4g.8xlarge", "i4g.16xlarge",
            "i4i.large", "i4i.xlarge", "i4i.2xlarge", "i4i.4xlarge", "i4i.8xlarge", "i4i.16xlarge", "i4i.32xlarge", "i4i.metal",
            "im4gn.large", "im4gn.xlarge", "im4gn.2xlarge", "im4gn.4xlarge", "im4gn.8xlarge", "im4gn.16xlarge",
            "is4gen.medium", "is4gen.large", "is4gen.xlarge", "is4gen.2xlarge", "is4gen.4xlarge", "is4gen.8xlarge",
            // Accelerated Computing - P family (GPU)
            "p2.xlarge", "p2.8xlarge", "p2.16xlarge",
            "p3.2xlarge", "p3.8xlarge", "p3.16xlarge",
            "p3dn.24xlarge",
            "p4d.24xlarge",
            "p4de.24xlarge",
            "p5.48xlarge",
            // Accelerated Computing - G family (Graphics)
            "g2.2xlarge", "g2.8xlarge",
            "g3.4xlarge", "g3.8xlarge", "g3.16xlarge",
            "g3s.xlarge",
            "g4ad.xlarge", "g4ad.2xlarge", "g4ad.4xlarge", "g4ad.8xlarge", "g4ad.16xlarge",
            "g4dn.xlarge", "g4dn.2xlarge", "g4dn.4xlarge", "g4dn.8xlarge", "g4dn.12xlarge", "g4dn.16xlarge", "g4dn.metal",
            "g5.xlarge", "g5.2xlarge", "g5.4xlarge", "g5.8xlarge", "g5.12xlarge", "g5.16xlarge", "g5.24xlarge", "g5.48xlarge",
            "g5g.xlarge", "g5g.2xlarge", "g5g.4xlarge", "g5g.8xlarge", "g5g.16xlarge", "g5g.metal",
            "g6.xlarge", "g6.2xlarge", "g6.4xlarge", "g6.8xlarge", "g6.12xlarge", "g6.16xlarge", "g6.24xlarge", "g6.48xlarge",
            // Accelerated Computing - Inf family (Inferentia)
            "inf1.xlarge", "inf1.2xlarge", "inf1.6xlarge", "inf1.24xlarge",
            "inf2.xlarge", "inf2.8xlarge", "inf2.24xlarge", "inf2.48xlarge",
            // Accelerated Computing - Trn family (Trainium)
            "trn1.2xlarge", "trn1.32xlarge",
            "trn1n.32xlarge",
            // Accelerated Computing - F family (FPGA)
            "f1.2xlarge", "f1.4xlarge", "f1.16xlarge",
            // Accelerated Computing - VT family (Video transcoding)
            "vt1.3xlarge", "vt1.6xlarge", "vt1.24xlarge",
            // Accelerated Computing - DL family (Deep learning)
            "dl1.24xlarge",
            "dl2q.24xlarge",
            // High Performance Computing
            "hpc6a.48xlarge",
            "hpc6id.32xlarge",
            "hpc7a.12xlarge", "hpc7a.24xlarge", "hpc7a.48xlarge", "hpc7a.96xlarge",
            "hpc7g.4xlarge", "hpc7g.8xlarge", "hpc7g.16xlarge",
        ].iter().map(|s| s.to_string()).collect::<Vec<_>>();

        let filtered_resize_types = resize_instance_types.clone();
        let mut resize_selector_state = ListState::default();
        resize_selector_state.select(Some(0));

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
            show_instance_filter: false,
            instance_filter: String::new(),
            filtered_instance_indices: Vec::new(),
            animation_start: Instant::now(),
            color_sequence,
            show_resize_dialog: false,
            resize_instance_types,
            resize_type_filter: String::new(),
            filtered_resize_types,
            resize_selector_state,
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

        // When filter is active, navigate through filtered indices
        if self.is_filter_active() {
            if self.filtered_instance_indices.is_empty() {
                return;
            }

            // Selection represents position in filtered list (0, 1, 2, ...)
            let current_pos = self.table_state.selected().unwrap_or(0);
            let next_pos = if current_pos >= self.filtered_instance_indices.len() - 1 {
                0 // Wrap to beginning
            } else {
                current_pos + 1
            };

            self.table_state.select(Some(next_pos));
        } else {
            // No filter active, use normal navigation
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
    }

    fn previous_instance(&mut self) {
        if self.instances.is_empty() {
            return;
        }

        // When filter is active, navigate through filtered indices
        if self.is_filter_active() {
            if self.filtered_instance_indices.is_empty() {
                return;
            }

            // Selection represents position in filtered list (0, 1, 2, ...)
            let current_pos = self.table_state.selected().unwrap_or(0);
            let prev_pos = if current_pos == 0 {
                self.filtered_instance_indices.len() - 1 // Wrap to end
            } else {
                current_pos - 1
            };

            self.table_state.select(Some(prev_pos));
        } else {
            // No filter active, use normal navigation
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
    }

    fn toggle_selection(&mut self) {
        if let Some(i) = self.get_actual_instance_index() {
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

    fn open_instance_filter(&mut self) {
        self.show_instance_filter = true;
    }

    fn close_instance_filter(&mut self) {
        self.show_instance_filter = false;
    }

    fn clear_instance_filter(&mut self) {
        self.instance_filter.clear();
        self.filtered_instance_indices.clear();
        self.status_message = "Filter cleared".to_string();
    }

    fn update_instance_filter(&mut self) {
        if self.instance_filter.is_empty() {
            self.filtered_instance_indices.clear();
            return;
        }

        // Try to compile as regex
        match Regex::new(&self.instance_filter) {
            Ok(re) => {
                self.filtered_instance_indices = self.instances
                    .iter()
                    .enumerate()
                    .filter(|(_, instance)| self.instance_matches_regex(instance, &re))
                    .map(|(i, _)| i)
                    .collect();
            }
            Err(_) => {
                // If regex is invalid, treat as literal string match (case-insensitive)
                let filter_lower = self.instance_filter.to_lowercase();
                self.filtered_instance_indices = self.instances
                    .iter()
                    .enumerate()
                    .filter(|(_, instance)| self.instance_matches_literal(instance, &filter_lower))
                    .map(|(i, _)| i)
                    .collect();
            }
        }

        // Reset table selection if current selection is not in filtered results
        if let Some(selected) = self.table_state.selected() {
            if !self.filtered_instance_indices.contains(&selected) && !self.filtered_instance_indices.is_empty() {
                self.table_state.select(Some(self.filtered_instance_indices[0]));
            }
        }
    }

    fn instance_matches_regex(&self, instance: &Ec2Instance, re: &Regex) -> bool {
        // Check all instance properties
        if re.is_match(&instance.id) ||
           re.is_match(&instance.name) ||
           re.is_match(&instance.state) ||
           re.is_match(&instance.instance_type) ||
           re.is_match(&instance.public_ip) ||
           re.is_match(&instance.private_ip) ||
           re.is_match(&instance.ipv6) {
            return true;
        }

        // Check all tag keys and values
        for (key, value) in &instance.tags {
            if re.is_match(key) || re.is_match(value) {
                return true;
            }
        }

        false
    }

    fn instance_matches_literal(&self, instance: &Ec2Instance, filter: &str) -> bool {
        // Check all instance properties
        if instance.id.to_lowercase().contains(filter) ||
           instance.name.to_lowercase().contains(filter) ||
           instance.state.to_lowercase().contains(filter) ||
           instance.instance_type.to_lowercase().contains(filter) ||
           instance.public_ip.to_lowercase().contains(filter) ||
           instance.private_ip.to_lowercase().contains(filter) ||
           instance.ipv6.to_lowercase().contains(filter) {
            return true;
        }

        // Check all tag keys and values
        for (key, value) in &instance.tags {
            if key.to_lowercase().contains(filter) || value.to_lowercase().contains(filter) {
                return true;
            }
        }

        false
    }

    fn is_filter_active(&self) -> bool {
        !self.instance_filter.is_empty()
    }

    fn get_actual_instance_index(&self) -> Option<usize> {
        let selected = self.table_state.selected()?;

        if self.is_filter_active() {
            // When filter is active, map the table row position to the actual instance index
            self.filtered_instance_indices.get(selected).copied()
        } else {
            // No filter, the selection is the actual instance index
            Some(selected)
        }
    }

    fn open_resize_dialog(&mut self) {
        self.show_resize_dialog = true;
        self.resize_type_filter.clear();
        self.filtered_resize_types = self.resize_instance_types.clone();
        self.resize_selector_state.select(Some(0));
    }

    fn close_resize_dialog(&mut self) {
        self.show_resize_dialog = false;
        self.resize_type_filter.clear();
    }

    fn update_resize_type_filter(&mut self) {
        if self.resize_type_filter.is_empty() {
            self.filtered_resize_types = self.resize_instance_types.clone();
        } else {
            let filter_lower = self.resize_type_filter.to_lowercase();
            self.filtered_resize_types = self.resize_instance_types
                .iter()
                .filter(|itype| itype.to_lowercase().contains(&filter_lower))
                .cloned()
                .collect();
        }

        let current_selection = self.resize_selector_state.selected().unwrap_or(0);
        if current_selection >= self.filtered_resize_types.len() && !self.filtered_resize_types.is_empty() {
            self.resize_selector_state.select(Some(0));
        } else if self.filtered_resize_types.is_empty() {
            self.resize_selector_state.select(None);
        }
    }

    fn next_filtered_resize_type(&mut self) {
        if self.filtered_resize_types.is_empty() {
            return;
        }

        let i = match self.resize_selector_state.selected() {
            Some(i) => {
                if i >= self.filtered_resize_types.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.resize_selector_state.select(Some(i));
    }

    fn previous_filtered_resize_type(&mut self) {
        if self.filtered_resize_types.is_empty() {
            return;
        }

        let i = match self.resize_selector_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_resize_types.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.resize_selector_state.select(Some(i));
    }

    fn get_selected_resize_type(&self) -> Option<String> {
        if let Some(selected_idx) = self.resize_selector_state.selected() {
            if selected_idx < self.filtered_resize_types.len() {
                return Some(self.filtered_resize_types[selected_idx].clone());
            }
        }
        None
    }

    fn get_animated_border_color(&self) -> Color {
        // Calculate elapsed time in seconds
        let elapsed = self.animation_start.elapsed().as_secs_f32();

        // Cycle duration: 8 seconds per color (32 seconds total for 4 colors)
        let cycle_duration = 8.0;
        let total_duration = cycle_duration * self.color_sequence.len() as f32;

        // Get current position in the animation cycle
        let cycle_position = (elapsed % total_duration) / cycle_duration;
        let current_index = cycle_position.floor() as usize % self.color_sequence.len();
        let next_index = (current_index + 1) % self.color_sequence.len();

        // Get the interpolation factor (0.0 to 1.0) between current and next color
        let t = cycle_position.fract();

        // Use smooth interpolation (ease-in-out)
        let t_smooth = t * t * (3.0 - 2.0 * t);

        // Interpolate between current and next color
        self.interpolate_color(
            self.color_sequence[current_index],
            self.color_sequence[next_index],
            t_smooth
        )
    }

    fn interpolate_color(&self, color1: Color, color2: Color, t: f32) -> Color {
        let rgb1 = self.color_to_rgb(color1);
        let rgb2 = self.color_to_rgb(color2);

        let r = (rgb1.0 as f32 + (rgb2.0 as f32 - rgb1.0 as f32) * t) as u8;
        let g = (rgb1.1 as f32 + (rgb2.1 as f32 - rgb1.1 as f32) * t) as u8;
        let b = (rgb1.2 as f32 + (rgb2.2 as f32 - rgb1.2 as f32) * t) as u8;

        Color::Rgb(r, g, b)
    }

    fn color_to_rgb(&self, color: Color) -> (u8, u8, u8) {
        match color {
            Color::White => (255, 255, 255),
            Color::DarkGray => (128, 128, 128),
            Color::Cyan => (64, 224, 208),      // Teal color
            Color::Magenta => (147, 112, 219),  // Purple color
            Color::Rgb(r, g, b) => (r, g, b),
            _ => (255, 255, 255), // Default to white
        }
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

            // Collect all tags as key-value pairs
            let tags: Vec<(String, String)> = instance
                .tags()
                .iter()
                .filter_map(|tag| {
                    if let (Some(key), Some(value)) = (tag.key(), tag.value()) {
                        Some((key.to_string(), value.to_string()))
                    } else {
                        None
                    }
                })
                .collect();

            instances.push(Ec2Instance {
                id,
                name,
                state,
                instance_type,
                public_ip,
                private_ip,
                ipv6,
                tags,
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

async fn resize_instance(region: &str, instance_id: &str, new_instance_type: &str, log: &AppLog) -> Result<String> {
    log_message(log, LogLevel::Info, format!("Starting resize for instance {} to {} in region {}", instance_id, new_instance_type, region));

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()))
        .load()
        .await;

    let client = Ec2Client::new(&config);

    // Modify the instance attribute
    match client
        .modify_instance_attribute()
        .instance_id(instance_id)
        .instance_type(aws_sdk_ec2::types::AttributeValue::builder().value(new_instance_type).build())
        .send()
        .await {
            Ok(_) => {
                let msg = format!("Successfully resized instance {} to {}", instance_id, new_instance_type);
                log_message(log, LogLevel::Info, msg.clone());
                log_message(log, LogLevel::Info, "Note: You may need to stop and start the instance for changes to take effect".to_string());
                Ok(msg)
            }
            Err(e) => {
                let error_str = e.to_string();
                let error_msg = format!("Failed to resize instance {} to {}: {}", instance_id, new_instance_type, error_str);
                log_message(log, LogLevel::Error, error_msg.clone());

                // Extract detailed AWS service error information
                let mut error_code = None;
                let mut error_message = None;

                // Try to extract error details from the service error
                if let Some(service_err) = e.as_service_error() {
                    error_code = service_err.code().map(|s| s.to_string());
                    error_message = service_err.message().map(|s| s.to_string());

                    if let Some(code) = &error_code {
                        log_message(log, LogLevel::Error, format!("AWS Error Code: {}", code));
                    }

                    if let Some(msg) = &error_message {
                        log_message(log, LogLevel::Error, format!("AWS Error Message: {}", msg));
                    }

                    // Log request ID if available
                    if let Some(request_id) = service_err.meta().request_id() {
                        log_message(log, LogLevel::Error, format!("AWS Request ID: {}", request_id));
                    }
                }

                // Provide helpful error guidance based on error code or message
                let error_code_str = error_code.as_deref().unwrap_or("");
                let error_msg_str = error_message.as_deref().unwrap_or("");

                if error_code_str == "IncorrectInstanceState" || error_str.contains("IncorrectInstanceState") || error_msg_str.contains("must be 'stopped'") {
                    log_message(log, LogLevel::Error, "The instance must be in 'stopped' state before resizing. Please stop the instance first.".to_string());
                } else if error_code_str == "Unsupported" || error_str.contains("Unsupported") || error_msg_str.contains("not supported") {
                    log_message(log, LogLevel::Error, format!("Instance type '{}' may not be available in region '{}' or availability zone.", new_instance_type, region));
                    log_message(log, LogLevel::Error, "Try selecting a different instance type from the list.".to_string());
                } else if error_str.contains("spot") || error_str.contains("Spot") || error_msg_str.contains("spot") {
                    log_message(log, LogLevel::Error, "Spot instances cannot be resized. You must launch a new spot instance with the desired type.".to_string());
                } else if error_code_str == "InsufficientInstanceCapacity" || error_str.contains("InsufficientInstanceCapacity") {
                    log_message(log, LogLevel::Error, "AWS does not have sufficient capacity for this instance type in the availability zone.".to_string());
                    log_message(log, LogLevel::Error, "Try a different instance type or wait and retry later.".to_string());
                } else if error_code_str == "InvalidParameterCombination" || error_str.contains("InvalidParameterCombination") {
                    log_message(log, LogLevel::Error, "The selected instance type is not compatible with the current instance configuration.".to_string());
                    log_message(log, LogLevel::Error, "Some instance types have specific requirements (e.g., EBS-optimized, enhanced networking).".to_string());
                } else if error_code_str == "InvalidInstanceID.NotFound" || error_str.contains("InvalidInstanceID") {
                    log_message(log, LogLevel::Error, "The instance was not found. It may have been terminated.".to_string());
                } else if error_code_str == "UnauthorizedOperation" {
                    log_message(log, LogLevel::Error, "You do not have permission to modify this instance. Check your IAM permissions.".to_string());
                } else if error_code_str == "InvalidInstanceAttributeValue" {
                    log_message(log, LogLevel::Error, "The instance type value is invalid or not compatible with this instance.".to_string());
                } else {
                    log_message(log, LogLevel::Error, "Please check the application logs for details and try a different instance type.".to_string());
                }

                Err(anyhow::anyhow!(error_msg))
            }
        }
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

    // Get the current animated border color
    let border_color = app.get_animated_border_color();

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
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(border_color)).title("Info"));
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
                .border_style(Style::default().fg(border_color))
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
                .border_style(Style::default().fg(border_color))
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
            .border_style(Style::default().fg(border_color))
            .title("Controls")
            .style(Style::default().bg(Color::Black)));

        f.render_widget(instructions, selector_chunks[1]);
    } else if app.show_instance_filter {
        let filter_area = centered_rect(60, 30, f.area());

        let filter_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(filter_area);

        // Input box
        let input = Paragraph::new(app.instance_filter.as_str())
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title("Filter EC2 Instances (Regex)")
                .style(Style::default().bg(Color::Black)));
        f.render_widget(input, filter_chunks[0]);

        // Instructions
        let match_count = if app.instance_filter.is_empty() {
            format!("Type to filter instances by ID, name, state, type, IPs, or tags")
        } else {
            format!("Matching {} of {} instances", app.filtered_instance_indices.len(), app.instances.len())
        };

        let instructions = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Apply filter  "),
                Span::styled("Esc", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Cancel"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+X", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Clear filter  "),
                Span::styled("Backspace", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Delete char"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(match_count, Style::default().fg(Color::Yellow)),
            ]),
        ])
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title("Controls")
            .style(Style::default().bg(Color::Black)));

        f.render_widget(instructions, filter_chunks[1]);
    } else if app.show_resize_dialog {
        let resize_area = centered_rect(70, 80, f.area());

        let resize_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(11),
            ])
            .split(resize_area);

        // Instance type selector
        let type_items: Vec<ListItem> = app.filtered_resize_types
            .iter()
            .map(|itype| {
                ListItem::new(Line::from(vec![
                    Span::styled(itype.clone(), Style::default()),
                ]))
            })
            .collect();

        let title = if app.resize_type_filter.is_empty() {
            format!("Select New Instance Type (showing {} types)", app.filtered_resize_types.len())
        } else {
            format!("Select New Instance Type - Filter: '{}' (showing {} types)", app.resize_type_filter, app.filtered_resize_types.len())
        };

        let type_list = List::new(type_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(title)
                .style(Style::default().bg(Color::Black)))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        f.render_stateful_widget(type_list, resize_chunks[0], &mut app.resize_selector_state);

        // Warning and instructions
        let instructions = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("⚠ WARNING:", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw(" Instance must be STOPPED before resizing!"),
            ]),
            Line::from(vec![
                Span::styled("Read Amazon EC2 Instance Type Changes documentation", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Common failures:", Style::default().fg(Color::Yellow)),
                Span::raw(" Spot instances, wrong region/AZ, instance running"),
            ]),
            Line::from(vec![
                Span::raw("Check logs (press 'l') if resize fails. Try a different type."),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("↑/↓ or j/k", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Navigate  "),
                Span::styled("Type", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Filter"),
            ]),
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Resize  "),
                Span::styled("Esc", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" - Cancel"),
            ]),
        ])
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title("Controls & Warning")
            .style(Style::default().bg(Color::Black)));

        f.render_widget(instructions, resize_chunks[1]);
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
                .border_style(Style::default().fg(border_color))
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
            Line::from("  f             - Filter instances (regex search)"),
            Line::from("  r             - Refresh instance list"),
            Line::from("  s             - Start selected instances"),
            Line::from("  t             - Stop selected instances"),
            Line::from("  d             - Terminate selected instances"),
            Line::from("  a             - Create AMI from selected instances"),
            Line::from("  c             - Clear all selections"),
            Line::from("  Ctrl+4        - Copy IPv4 address to clipboard"),
            Line::from("  Ctrl+6        - Copy IPv6 address to clipboard"),
            Line::from("  Ctrl+R        - Resize selected instance type"),
            Line::from("  Ctrl+X        - Clear active filter"),
            Line::from("  l             - Toggle application logs"),
            Line::from("  h             - Toggle this help"),
            Line::from("  q or Ctrl+C   - Quit"),
        ];
        
        let help_area = centered_rect(60, 60, f.area());
        let help = Paragraph::new(help_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title("Help")
                .style(Style::default().bg(Color::Black)));
        f.render_widget(help, help_area);
    } else {
        // Instance table
        let selected_style = Style::default()
            .bg(border_color)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD);

        let header_cells = ["✓", "Instance ID", "Name", "State", "Type", "Public IP", "Private IP"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));

        let header = Row::new(header_cells).height(1).bottom_margin(1);

        // Determine which instances to display based on filter
        let instances_to_show: Vec<(usize, &Ec2Instance)> = if app.is_filter_active() {
            app.filtered_instance_indices
                .iter()
                .filter_map(|&i| app.instances.get(i).map(|inst| (i, inst)))
                .collect()
        } else {
            app.instances.iter().enumerate().collect()
        };

        let rows = instances_to_show.iter().map(|(i, inst)| {
            let checkbox = if app.selected_instances.get(*i).copied().unwrap_or(false) {
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

        // Customize title based on filter state
        let title = if app.is_filter_active() {
            format!("EC2 Instances [FILTERED: {} of {}] - Filter: '{}'",
                app.filtered_instance_indices.len(),
                app.instances.len(),
                app.instance_filter)
        } else {
            "EC2 Instances".to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));

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
        .block(block)
        .row_highlight_style(selected_style)
        .highlight_symbol(">> ");

        f.render_stateful_widget(table, chunks[1], &mut app.table_state);
    }

    // Status bar
    let status = Paragraph::new(app.status_message.clone())
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(border_color)).title("Status"));
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
            app.update_instance_filter(); // Reapply filter to new instances
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
                    if let Some(selected_idx) = app.get_actual_instance_index() {
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
                    if let Some(selected_idx) = app.get_actual_instance_index() {
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

                // Handle CTRL+X to clear filter
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('x') {
                    if app.is_filter_active() {
                        app.clear_instance_filter();
                        log_message(&app.log, LogLevel::Info, "Instance filter cleared".to_string());
                    }
                    continue;
                }

                // Handle CTRL+R to resize instance
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
                    if let Some(selected_idx) = app.get_actual_instance_index() {
                        if selected_idx < app.instances.len() {
                            app.open_resize_dialog();
                            app.status_message = "Select new instance type".to_string();
                        }
                    } else {
                        app.status_message = "No instance selected".to_string();
                    }
                    continue;
                }

                // Handle resize dialog input
                if app.show_resize_dialog {
                    match key.code {
                        KeyCode::Esc => {
                            app.close_resize_dialog();
                            app.status_message = "Resize cancelled".to_string();
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.next_filtered_resize_type();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.previous_filtered_resize_type();
                        }
                        KeyCode::Enter => {
                            if let Some(new_type) = app.get_selected_resize_type() {
                                if let Some(selected_idx) = app.get_actual_instance_index() {
                                    if let Some(instance) = app.instances.get(selected_idx) {
                                        let instance_id = instance.id.clone();
                                        let region = app.current_region.clone();

                                        app.close_resize_dialog();
                                        app.status_message = format!("Resizing instance {} to {}...", instance_id, new_type);
                                        terminal.draw(|f| ui(f, &mut app))?;

                                        match resize_instance(&region, &instance_id, &new_type, &app.log).await {
                                            Ok(msg) => {
                                                app.status_message = msg;
                                            }
                                            Err(e) => {
                                                app.status_message = format!("Error resizing instance: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            app.resize_type_filter.pop();
                            app.update_resize_type_filter();
                        }
                        KeyCode::Char(c) => {
                            if c != 'j' && c != 'k' {
                                app.resize_type_filter.push(c);
                                app.update_resize_type_filter();
                            }
                        }
                        _ => {}
                    }
                    continue;
                }

                // Handle instance filter input
                if app.show_instance_filter {
                    match key.code {
                        KeyCode::Esc => {
                            app.close_instance_filter();
                            app.status_message = "Filter cancelled".to_string();
                        }
                        KeyCode::Enter => {
                            app.update_instance_filter();
                            app.close_instance_filter();
                            if app.is_filter_active() {
                                app.status_message = format!("Filter applied: {} matches", app.filtered_instance_indices.len());
                                log_message(&app.log, LogLevel::Info, format!("Applied filter '{}': {} matches", app.instance_filter, app.filtered_instance_indices.len()));
                            } else {
                                app.status_message = "Filter is empty".to_string();
                            }
                        }
                        KeyCode::Backspace => {
                            app.instance_filter.pop();
                            app.update_instance_filter();
                        }
                        KeyCode::Char(c) => {
                            app.instance_filter.push(c);
                            app.update_instance_filter();
                        }
                        _ => {}
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
                                        app.update_instance_filter(); // Reapply filter to new instances
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
                    KeyCode::Esc => {
                        // Close help or logs if they're open
                        if app.show_help {
                            app.show_help = false;
                        } else if app.show_logs {
                            app.show_logs = false;
                        }
                    }
                    KeyCode::Char('g') => {
                        if !app.show_help && !app.is_loading {
                            app.open_region_selector();
                        }
                    }
                    KeyCode::Char('f') => {
                        if !app.show_help && !app.is_loading {
                            app.open_instance_filter();
                            app.status_message = "Enter filter pattern (regex)".to_string();
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
                                    app.update_instance_filter(); // Reapply filter to new instances
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
                                    app.update_instance_filter(); // Reapply filter to new instances
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
                                    app.update_instance_filter(); // Reapply filter to new instances
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
