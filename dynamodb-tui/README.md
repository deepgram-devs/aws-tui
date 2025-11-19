# DynamoDB TUI

An interactive terminal user interface for managing AWS DynamoDB tables and items.

## Features

### Phase 1: Core Functionality ✅
- **Region Support**: Switch between all AWS regions with interactive selector
- **Table Listing**: Fast listing of all tables in the current region with lazy loading
- **Table Details**: View detailed information about tables including:
  - Hash and range keys
  - Billing mode (On-Demand or Provisioned)
  - Item count and table size
  - Table status

### Phase 2: Table Operations ✅
- **Create Tables**: Multi-step wizard for creating tables with:
  - Partition key (hash key) configuration
  - Optional sort key (range key)
  - Billing mode selection (On-Demand or Provisioned)
  - Capacity configuration for provisioned tables
- **Delete Tables**: Safe deletion with confirmation dialog
- **Region Switching**: Interactive region selector with filtering

### Phase 3: Item Operations ✅
- **View Items**: Browse items in any table with pagination
- **Create Items**: Add new items with:
  - Required primary key fields
  - Additional attributes
  - Support for multiple attribute types (String, Number, Boolean, etc.)
- **Edit Items**: Modify existing items (primary keys are read-only)
- **Delete Items**: Remove items with confirmation

## Installation

From the workspace root:

```bash
cargo build --package dynamodb-tui
```

## Usage

Run the application:

```bash
cargo run --package dynamodb-tui
```

Or install and run the binary:

```bash
cargo install --path dynamodb-tui
dynamodb-tui
```

## Keyboard Shortcuts

### Main Tables View
- `↑/↓` - Navigate table list
- `Enter` - View items in selected table
- `r` - Refresh table status and details
- `g` - Open region selector
- `c` - Create new table
- `d` - Delete selected table
- `?` - Show help screen
- `q` / `Esc` - Quit application

### Items View
- `↑/↓` - Navigate item list
- `i` - Add new item
- `e` / `Enter` - Edit selected item
- `d` - Delete selected item
- `Esc` - Return to tables view

### Item Editor
- `↑/↓` - Navigate attributes
- `Enter` - Edit attribute value
- `Ctrl+K` - Edit attribute name
- `Ctrl+T` - Change attribute type (cycles: String → Number → Binary)
- `Ctrl+N` - Add new attribute (defaults to String type)
- `Ctrl+D` - Delete attribute (except primary keys)
- `Ctrl+S` - Save item
- `Esc` - Cancel without saving

### Region Selector
- `↑/↓` - Navigate regions
- Type characters to filter regions
- `Enter` - Select region
- `Esc` - Cancel

### Table Creation Wizard
- `Tab` - Navigate between fields
- `Enter` - Next step
- `Backspace` - Previous step
- `Esc` - Cancel

## Architecture

The application follows a modular design:

- **models.rs**: Data structures for tables, items, and attribute values
- **aws.rs**: AWS SDK integration with DynamoDB operations
- **app.rs**: Application state management
- **ui.rs**: Ratatui-based UI rendering
- **keyboard.rs**: Input handling and keyboard shortcuts
- **main.rs**: Main event loop and async operation coordination

### Key Design Patterns

1. **Lazy Loading**: Table details are loaded on-demand when selected, improving initial load time
2. **Pagination**: Items are loaded in pages (50 items per page by default) to handle large tables
3. **State Machine**: Clean state transitions between different views and operations
4. **Type Safety**: Strong typing for AWS attribute types with conversion to/from SDK types

## AWS Permissions Required

The application requires the following IAM permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "dynamodb:ListTables",
        "dynamodb:DescribeTable",
        "dynamodb:CreateTable",
        "dynamodb:DeleteTable",
        "dynamodb:Scan",
        "dynamodb:GetItem",
        "dynamodb:PutItem",
        "dynamodb:UpdateItem",
        "dynamodb:DeleteItem"
      ],
      "Resource": "*"
    }
  ]
}
```

## Configuration

The application uses the AWS SDK for Rust, which follows standard AWS credential resolution:
1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. AWS credentials file (`~/.aws/credentials`)
3. IAM role (when running on EC2, ECS, or Lambda)

## Known Limitations

- Global Tables management is not yet implemented (planned for Phase 4)
- Complex nested attribute types (Map, List) have simplified editing
- Query operations are not yet supported (only Scan)
- GSI/LSI management not yet implemented
- No batch operations support

## Future Enhancements

### Phase 4: Global Tables (Planned)
- View global tables and their replicas
- Create global tables across regions
- Add/remove replicas
- Per-region status monitoring

### Phase 5: Advanced Features (Planned)
- Query builder for efficient item retrieval
- GSI/LSI creation and management
- Stream configuration
- TTL management
- Point-in-time recovery settings
- Encryption settings
- Export items to JSON/CSV
- Batch item operations
- Enhanced logging and filtering

## Dependencies

- **ratatui**: Terminal UI framework
- **crossterm**: Terminal manipulation
- **aws-sdk-dynamodb**: AWS DynamoDB SDK
- **tokio**: Async runtime
- **anyhow**: Error handling
- **chrono**: Timestamp formatting
- **regex**: Region filtering

## Contributing

This tool is part of the ec2-tui workspace. Follow the established patterns from ec2-tui, efs-tui, and cfn-tui when adding new features.

## License

(Add your license information here)
