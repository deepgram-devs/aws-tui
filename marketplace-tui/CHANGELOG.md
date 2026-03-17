# Changelog

## [0.1.0] - 2026-03-15

### Added

- Initial release of `marketplace-tui`
- **Product List View**: Browse all AWS Marketplace ML product listings with real-time regex filtering, visibility status color coding, and paginated navigation (↑↓, PgUp/PgDn, j/k)
- **Product Detail View**: Tabbed interface (Overview / Versions / Pricing / Allowlist) with lazy-loaded full product details
- **Version List View**: Browse delivery options with visibility status, ID copying, and restriction support
- **Create Product Wizard**: Multi-step wizard (Basic Info → Descriptions → Support & Compliance → Review) with per-step validation, progress indicator, and automatic tagging with creation date
- **Create Version Wizard**: Multi-step wizard with SageMaker Model Package ARN support
- **Metadata Editor**: In-place field editing for all product metadata with Ctrl+S save
- **Allowlist / Whitelist Manager**: Add/remove 12-digit AWS account IDs with real-time validation feedback
- **Pricing Editor**: Update usage-based pricing (price, currency, dimension key) with standard AWS EULA applied automatically
- **Restrict Version**: Deprecate delivery options with confirmation dialog
- **Background Changeset Polling**: All AWS Marketplace mutations submit changesets that are polled every 5 seconds in the background with status notifications
- **Change Set Status View**: Real-time view of all submitted changesets and their status (Preparing / Applying / Succeeded / Failed)
- **Command Palette**: Fuzzy-search interface for all application actions with keyboard shortcuts shown (`:` or `Space`)
- **Help Popup**: Scrollable keyboard shortcut reference (`?`)
- **Log Viewer**: Full application log accessible via `l` with scrolling support
- **Clipboard Support**: Copy entity IDs and version IDs to system clipboard (`y`)
- **Status Bar**: Persistent status messages with color-coded error/success states
- **Spinner Animation**: Visual feedback during loading and while changesets are in-flight
- AWS credential chain support (env vars, `~/.aws/credentials`, IAM roles)
- Mouse support: scroll wheel navigation and click selection
