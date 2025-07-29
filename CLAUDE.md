# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# 重要说明

- **总是使用中文来输出**
- 合理利用emoji表情来优化输出内容，增强表达

## Development Commands

### Build and Run
```bash
# Run the main application
cargo run

# Build in release mode
cargo build --release

# Check code
cargo check

# Format code (rustfmt.toml configured with max_width=140)
cargo fmt
```

### Database Operations
```bash
# Start local development database and Redis
cd db_helper && docker compose up

# Install sqlx-cli (one-time setup)
cargo install sqlx-cli

# Create database
sqlx database create

# Run migrations
sqlx migrate run

# Add new migration
sqlx migrate add <migration_name>
```

### Testing
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p <crate_name>
```

## Architecture Overview

This is a monolithic Rust backend application designed for small-to-medium scale applications, structured as a workspace with multiple crates:

### Core Architecture
- **Monolithic Design**: Single binary with modular internal structure
- **Async Runtime**: Built on Tokio with graceful shutdown
- **Service Isolation**: Separate crates for different concerns
- **Shared Configuration**: Centralized config through `shared-lib`

### Workspace Structure
```
crates/
├── shared-lib/        # Shared models, config, and utilities
├── web-service/       # HTTP API server (Axum)
├── consumer-service/  # Redis message queue consumers
├── cronjob-service/   # Scheduled task processor
└── database/          # Database layer with repositories
```

### Service Communication
- **Database**: PostgreSQL via sqlx (no ORM - direct SQL approach)
- **Message Queue**: Redis streams for async processing
- **HTTP API**: Axum web framework with OpenAPI documentation
- **Configuration**: Environment variables + .env files via dotenvy

### Key Design Patterns

#### Database Layer
- **Repository Pattern**: `ProjectRepositoryTrait` for data access abstraction
- **No ORM**: Direct SQL with sqlx for transparency and control
- **Connection Pooling**: Built-in sqlx connection management
- **Migration System**: sqlx migrations in `migrations/` directory

#### Message Processing
- **Redis Streams**: Used as message queue with consumer groups
- **Typed Consumers**: Separate consumer types (TaskTypeA, TaskTypeB) for different message types
- **Heartbeat System**: Consumer health monitoring via Redis
- **Graceful Shutdown**: All consumers respect shutdown signals

#### Web Service
- **State Management**: `AppState` with shared service instances
- **Service Layer**: Business logic separated from routing
- **Trait-based Design**: Services use traits for testability
- **Graceful Shutdown**: Axum's `with_graceful_shutdown` for clean termination

### Memory Management
- **Global Allocator**: Uses mimalloc for improved performance
- **Zero-copy**: Where possible, especially in message processing
- **Arc-based Sharing**: Shared state uses atomic reference counting

### Configuration System
Configuration is loaded from environment variables and `.env` files:
- `AppConfig`: Main application configuration
- `RedisConfig`: Redis connection and consumer settings
- `DatabaseConfig`: PostgreSQL connection settings

### Error Handling
- **color-eyre**: Enhanced error reporting with context
- **Custom Error Types**: Domain-specific error types in each crate
- **Graceful Degradation**: Automatic retry for transient failures
- **Structured Logging**: Tracing with levels and context

### Development Philosophy
- **Transparency over Abstraction**: Direct SQL rather than ORM complexity
- **Type Safety**: Leverage Rust's type system, especially for message processing
- **Simplicity**: Single binary deployment for easier operations
- **Performance**: Async processing, connection pooling, efficient memory usage

### Technology Choices (from FAQ)
- **PostgreSQL**: Chosen over MySQL for advanced features
- **Redis Streams**: Simple message queue suitable for medium-scale applications
- **sqlx over ORM**: Direct SQL access for transparency and control
- **Axum**: Modern, fast web framework built on Tokio

### Graceful Shutdown
All services implement coordinated shutdown:
1. Signal handler listens for Ctrl+C/SIGTERM
2. Broadcasts shutdown via `tokio::sync::watch`
3. Each service finishes current work
4. Connections are properly closed
5. Application exits cleanly
