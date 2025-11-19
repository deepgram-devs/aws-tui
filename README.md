# AWS TUI Tools

A collection of Terminal User Interface (TUI) tools for managing various AWS services, built with Rust and Ratatui.

## Sub-projects

* **cfn-tui**: A TUI for viewing and managing AWS CloudFormation stacks.
* **dynamodb-tui**: A TUI for exploring and interacting with AWS DynamoDB tables.
* **ec2-tui**: A TUI for monitoring and managing AWS EC2 instances.
* **efs-tui**: A TUI for viewing AWS Elastic File System (EFS) resources.

## Prerequisites

* Rust and Cargo installed
* AWS credentials configured (e.g., via `~/.aws/credentials` or environment variables)

## Usage

Navigate to the specific directory and run with cargo:

```bash
cd [project-name]
cargo run
```
