````markdown
# CloudFormation TUI

A terminal user interface (TUI) for managing AWS CloudFormation stacks, built with Rust and ratatui.

## Features

- **List CloudFormation Stacks**: View all stacks in the selected AWS region
- **Delete Stacks**: Delete selected CloudFormation stacks
- **View Stack Events**: Display stack events in reverse chronological order with scrollable interface
- **View Stack Resources**: List all resources and their types in the selected stack
- **Copy Template to Clipboard**: Fetch and copy the CloudFormation template to clipboard
- **View Stack Exports**: List all stack exports in the current region
- **Region Switching**: Navigate between AWS regions with arrow keys or regex filtering
- **Application Logs**: View detailed application logs with optional text wrapping

## Installation

### Prerequisites

- Rust toolchain (1.70 or later)
- AWS credentials configured (via `~/.aws/credentials` or environment variables)

### Build from Source

```bash
cd cfn-tui
cargo build --release
````

The binary will be available at `target/release/cfn-tui`.

## Usage

Run the application:

```bash
cargo run --release
```

Or run the compiled binary:

```bash
./target/release/cfn-tui
```

## Keyboard Shortcuts

### Navigation

- `↑/↓` or `j/k` - Navigate items in lists and tables
- `←/→` - Switch between AWS regions
- `g` - Open region selector with regex filtering

### Stack Operations

- `r` - Refresh stack list
- `d` - Delete selected stack
- `e` - View stack events (reverse chronological, scrollable)
- `s` - View stack resources
- `x` - View stack exports
- `t` - Copy CloudFormation template to clipboard

### View Management

- `Esc` or `b` - Return to stack list from detail views
- `h` - Toggle help screen
- `l` - Toggle application logs
- `w` - Toggle log text wrapping (when logs are visible)
- `q` or `Ctrl+C` - Quit application

## Features in Detail

### Stack List View

The main view displays all CloudFormation stacks in the current region with:

- Stack name
- Status (color-coded: green for complete, red for failed, yellow for in-progress)
- Creation timestamp
- Description

### Stack Events View

Press `e` on a selected stack to view its events:

- Events displayed in reverse chronological order (newest first)
- Scrollable list with navigation keys
- Shows timestamp, resource type, logical ID, status, and status reason
- Color-coded status indicators

### Stack Resources View

Press `s` on a selected stack to view its resources:

- Lists all resources in the stack
- Shows logical ID, physical ID, resource type, and status
- Scrollable table interface

### Stack Exports View

Press `x` to view all exports in the current region:

- Export name
- Export value
- Exporting stack ID

### Template Copy

Press `t` on a selected stack to:

- Fetch the CloudFormation template
- Copy it to your system clipboard
- View confirmation in the status bar

## Design

This TUI is modeled after the EC2 TUI design, providing a consistent user experience across AWS service management tools. It uses:

- __ratatui__ for the terminal UI framework
- __AWS SDK for Rust__ for CloudFormation API interactions
- __crossterm__ for terminal manipulation
- __arboard__ for clipboard operations

## License

This project follows the same license as the parent repository.

```
```
