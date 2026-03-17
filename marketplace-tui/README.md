# marketplace-tui

A terminal user interface for managing [AWS Marketplace](https://aws.amazon.com/marketplace/) machine learning product listings.

## Features

- Browse, filter, and inspect ML product listings
- Create new ML products with a guided multi-step wizard
- Add new product versions (with optional SageMaker Model Package ARN)
- Restrict (deprecate) product versions
- Edit product metadata (title, descriptions, keywords, logo)
- Manage the product allowlist (12-digit AWS account IDs)
- Update usage-based pricing
- Background changeset tracking with real-time status polling
- Command Palette for quick access to all functionality
- Full keyboard navigation and mouse support

## Prerequisites

- AWS credentials configured (env vars, `~/.aws/credentials`, or IAM role)
- The credentials must have permissions for `aws-marketplace:*` on the Marketplace Catalog API
- Rust toolchain (for building from source)

## Installation

```bash
# From the workspace root
cargo build --release -p marketplace-tui

# Run
./target/release/marketplace-tui
```

## Usage

### Basic navigation

```bash
# Start the TUI
marketplace-tui
```

The application opens on the Product List. Use arrow keys to navigate, `Enter` to open a product, and `?` for the full help screen.

### Command Palette

Press `:` or `Space` from any view to open the Command Palette. Type to filter, use `↑↓` to navigate results, and `Enter` to execute.

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑ / ↓` or `j / k` | Navigate lists |
| `PgUp / PgDn` | Page through lists |
| `Enter` | Open / confirm |
| `Esc` | Go back / cancel |
| `c` | Create product |
| `v` | Add version to selected product |
| `V` | Open version list |
| `e` | Edit metadata |
| `w` | Manage allowlist |
| `p` | Edit pricing |
| `r` | Restrict selected version |
| `R` | Refresh |
| `y` | Copy entity/version ID to clipboard |
| `s` | View change set status |
| `l` | Toggle log viewer |
| `?` | Help |
| `:` or `Space` | Command Palette |
| `q` | Quit |

### Create a Product

1. Press `c` from the Product List
2. Fill in **Basic Info** (title, short description) — press `Tab` to switch fields, `Enter` to advance
3. Fill in **Descriptions** (long description, categories, keywords, logo URL)
4. Fill in **Support & Compliance** (support description, refund policy)
5. Review the summary and press `Enter` to submit

The product is created with:
- Initial pricing of **$0.01 USD**
- **Standard AWS Marketplace Contract** EULA
- A `CreatedDate` tag set to today's date

### Add a Version

1. Open a product (`Enter`)
2. Press `v` to open the version wizard
3. Enter version title, release notes, and optionally a SageMaker Model Package ARN
4. Review and press `Enter` to submit

### Manage the Allowlist

1. Open a product (`Enter`) and navigate to the **Allowlist** tab or press `w`
2. Press `a` to add a 12-digit AWS account ID (validated in real time)
3. Use `↑↓` to select accounts and `d` to remove them

### Background Change Sets

All mutating operations (create, update, restrict) submit an asynchronous AWS Marketplace changeset. The TUI polls changeset status every 5 seconds in the background and displays a spinner in the header when changesets are in-flight. Press `s` to view all tracked changesets and their current status.

## AWS Permissions

The following IAM permissions are required:

```json
{
    "Effect": "Allow",
    "Action": [
        "aws-marketplace:ListEntities",
        "aws-marketplace:DescribeEntity",
        "aws-marketplace:StartChangeSet",
        "aws-marketplace:DescribeChangeSet",
        "aws-marketplace:ListChangeSets"
    ],
    "Resource": "*"
}
```

## Architecture

```
marketplace-tui/src/
├── main.rs        — terminal setup, event loop, async operation dispatch
├── app.rs         — application state, wizard states, validation logic
├── aws.rs         — AWS Marketplace Catalog API wrapper
├── models.rs      — data types (ProductSummary, ProductDetail, DeliveryOption, etc.)
├── keyboard.rs    — keyboard input handling per application state
└── ui/
    ├── mod.rs     — main draw dispatch
    ├── common.rs  — shared layout, colors, utilities
    ├── views.rs   — product list, detail, version list, whitelist, pricing views
    ├── wizard.rs  — create product and create version wizard rendering
    └── popups.rs  — help, logs, changeset status, command palette, confirmations
```
