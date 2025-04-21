
# Rust SIEM Administrative Center

A Security Information and Event Management (SIEM) solution implemented in Rust that complies with cybersecurity legislation requirements.

## Features

- **Security Event Collection**: Gather logs and security events from various network sources
- **Centralized Storage**: Store events with timestamps and normalization
- **Correlation Analysis**: Detect advanced threats and anomalies
- **Automated Alerts**: Generate security alerts based on configured rules
- **PowerShell Script Management**: Securely store and execute PowerShell scripts
- **Ticket System**: Track and manage IT support requests
- **Compliance Reporting**: Generate reports that meet regulatory requirements
- **Access Control**: Integration with Active Directory for permissions management

## Requirements

- Rust (2021 edition)
- PostgreSQL database
- Network access to monitored systems

## Configuration

The system uses a TOML configuration file (`config.toml`) for its settings. A default configuration is generated if none exists.

## Getting Started

1. Clone the repository
2. Install dependencies with `cargo build`
3. Configure your settings in `config.toml`
4. Run the application with `cargo run`

## Architecture

The project is organized into several modules:

- `api`: REST API endpoints for the web interface
- `config`: Configuration loading and management
- `models`: Data structures and database models
- `scripts`: PowerShell script management
- `security`: Authentication, encryption, and audit logging
- `tickets`: IT support ticket system
- `printers`: Output formatting utilities

## Security Features

- AES-256 encryption for sensitive data
- JWT authentication for API access
- Comprehensive audit logging
- Role-based access control

## Legal Compliance

This SIEM solution is designed to meet the requirements of:
- Czech Cybersecurity Act implementing EU NIS2 Directive
- Rapid incident reporting capabilities
- Risk management functionality
- Implementation of measures issued by national security authorities

## License

[Specify your license here]
