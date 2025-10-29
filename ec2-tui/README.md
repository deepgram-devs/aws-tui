# EC2 TUI - EC2 Instance Manager

An interactive terminal user interface (TUI) for managing AWS EC2 instances, built with Rust and ratatui.

## Features

- **Region Switching**: Navigate between AWS regions with arrow keys
- **Instance List**: View all EC2 instances in a table format with key details
- **Multi-Selection**: Select multiple instances using checkboxes
- **Bulk Actions**: Perform operations on multiple instances at once
  - Start instances
  - Stop instances
  - Terminate instances
- **Auto-Refresh**: Instance list automatically refreshes when switching regions
- **Manual Refresh**: Force refresh with keyboard shortcut
- **Color-Coded States**: Instance states are color-coded for easy identification
  - Green: Running
  - Red: Stopped
  - Yellow: Stopping
  - Cyan: Pending

## Prerequisites

- Rust (latest stable version)
- AWS credentials configured (via `~/.aws/credentials` or environment variables)
- Appropriate IAM permissions for EC2 operations:
  - `ec2:DescribeInstances`
  - `ec2:StartInstances`
  - `ec2:StopInstances`
  - `ec2:TerminateInstances`

## Installation

1. Clone the repository:

```bash
git clone <repository-url>
cd ec2-tui
```

1. Build the project:

```bash
cargo build --release
```

1. Run the application:

```bash
cargo run --release
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑` / `↓` or `j` / `k` | Navigate through instances |
| `Space` | Toggle instance selection (checkbox) |
| `←` / `→` | Switch between AWS regions |
| `r` | Refresh instance list |
| `s` | Start selected instances |
| `t` | Stop selected instances |
| `d` | Terminate selected instances |
| `c` | Clear all selections |
| `h` | Toggle help overlay |
| `q` or `Ctrl+C` | Quit application |

## Usage

1. **Launch the application**: Run `cargo run --release`
2. **Navigate regions**: Use left/right arrow keys to switch between AWS regions
3. **Select instances**: Use up/down arrows to navigate, press Space to select/deselect
4. **Perform actions**:
   - Press `s` to start selected instances
   - Press `t` to stop selected instances
   - Press `d` to terminate selected instances
5. **Refresh**: Press `r` to manually refresh the instance list
6. **Get help**: Press `h` to view the help overlay

## AWS Configuration

The application uses the AWS SDK for Rust and will automatically use your configured AWS credentials. Make sure you have either:

- AWS credentials file at `~/.aws/credentials`
- Environment variables: `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`
- IAM role (if running on EC2)

## Supported Regions

The application includes the following AWS regions by default:

- us-east-1
- us-east-2
- us-west-1
- us-west-2
- eu-west-1
- eu-central-1
- ap-southeast-1
- ap-northeast-1

## Display Information

The table displays the following information for each instance:

- **Checkbox**: Selection status
- **Instance ID**: The unique EC2 instance identifier
- **Name**: The instance name (from the "Name" tag)
- **State**: Current instance state (color-coded)
- **Type**: Instance type (e.g., t2.micro, m5.large)
- **Public IP**: Public IP address (if assigned)
- **Private IP**: Private IP address

## Error Handling

The application provides status messages for all operations:

- Success messages show the number of instances affected
- Error messages display AWS API errors
- Loading indicators show when operations are in progress

## Development

To run in development mode:

```bash
cargo run
```

To run tests:

```bash
cargo test
```

## License

[Add your license here]

## Contributing

[Add contribution guidelines here]
