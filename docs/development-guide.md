# 🛠️ 开发指南

本文档提供了 Rust 后端项目的详细开发指南，包括项目结构、开发命令、配置管理等内容。

## 📁 项目结构

```
rust-backend/
├── src/
│   └── main.rs              # 应用入口点
├── crates/                  # Workspace 模块
│   ├── shared-lib/          # 共享库
│   │   ├── src/
│   │   │   ├── models/      # 数据模型
│   │   │   ├── config.rs    # 配置管理
│   │   │   └── distributed_lock.rs
│   ├── web-service/         # Web API 服务
│   │   ├── src/
│   │   │   ├── routes/      # 路由定义
│   │   │   ├── services/    # 业务逻辑
│   │   │   └── models/      # Web 层模型
│   ├── consumer-service/    # 消息队列消费者
│   │   ├── src/
│   │   │   ├── task_type_a.rs
│   │   │   ├── task_type_b.rs
│   │   │   └── redis_interaction.rs
│   ├── cronjob-service/     # 定时任务服务
│   │   └── src/
│   │       └── jobs/        # 定时任务定义
│   └── database/            # 数据库层
│       ├── src/
│       │   ├── models/      # 数据库模型
│       │   ├── repositories/ # 数据访问层
│       │   └── connection.rs
├── migrations/              # 数据库迁移文件
├── db_helper/               # 本地开发环境
│   └── docker-compose.yml
├── docs/                    # 项目文档
└── Cargo.toml              # 项目配置
```

## 🚀 开发命令

### 基础命令

```bash
# 运行开发服务器
cargo run

# 构建生产版本
cargo build --release

# 代码检查
cargo check

# 代码格式化
cargo fmt

# 运行测试
cargo test
```

### 数据库操作

```bash
# 启动本地开发环境
cd db_helper && docker compose up -d

# 安装 sqlx-cli (首次)
cargo install sqlx-cli

# 创建数据库
sqlx database create

# 运行迁移
sqlx migrate run

# 创建新迁移
sqlx migrate add <migration_name>

# 验证 SQL 查询 (开发时)
cargo install sqlx-cli --features postgres
```

## ⚙️ 配置管理

### 环境变量配置

项目使用 `dotenvy` 管理环境变量，支持 `.env` 文件：

```env
# 数据库配置
DATABASE_URL=postgresql://username:password@localhost:5432/rust_backend

# Redis 配置
REDIS_URL=redis://localhost:6379

# 应用配置
RUST_LOG=debug
APP_PORT=8080
```

### 配置结构

- `AppConfig` - 应用主配置
- `DatabaseConfig` - 数据库配置
- `RedisConfig` - Redis 配置

## 🗄️ 数据库开发

### 迁移系统

- 使用 `sqlx` 迁移系统管理数据库版本
- 迁移文件位于 `migrations/` 目录
- 支持向上和向下迁移

### 数据模型

- 在 `crates/database/src/models/` 定义
- 使用 `sqlx::FromRow` 自动映射
- 支持关联关系和验证

### 仓储模式

- `ProjectRepositoryTrait` 定义数据访问接口
- 实现具体的 CRUD 操作
- 支持事务处理

## 📨 消息队列开发

### Redis Streams

- 使用 Redis Streams 作为消息队列
- 支持消费者组和消息确认
- 自动重试和错误处理

### 消费者类型

- `TaskTypeA` - A 类型消息处理器
- `TaskTypeB` - B 类型消息处理器
- 支持并发处理和批量操作

## 🌐 Web API 开发

### 路由设计

- 基于 Axum 的路由系统
- 支持嵌套路由和中间件
- 自动生成 OpenAPI 文档

### 服务层

- 业务逻辑与路由分离
- 使用 Trait 进行依赖注入
- 支持单元测试和集成测试

### 错误处理

- 统一的错误响应格式
- 支持自定义错误类型
- 自动 HTTP 状态码映射

## 🔄 开发流程

### 1. 环境准备

```bash
# 克隆项目
git clone <repository-url>
cd rust-backend

# 启动依赖服务
cd db_helper && docker compose up -d
cd ..

# 安装数据库工具
cargo install sqlx-cli

# 初始化数据库
sqlx database create
sqlx migrate run
```

### 2. 开发新功能

1. **创建分支**: `git checkout -b feature/new-feature`
2. **编写代码**: 遵循项目代码风格
3. **运行测试**: `cargo test`
4. **格式化代码**: `cargo fmt`
5. **检查代码**: `cargo clippy`
6. **提交代码**: 使用清晰的提交信息

### 3. 代码规范

- **格式化**: 使用 `cargo fmt` 格式化代码
- **代码检查**: 使用 `cargo clippy` 进行静态分析
- **测试**: 编写单元测试和集成测试
- **文档**: 更新相关文档

## 🧪 测试指南

### 单元测试

```bash
# 运行所有测试
cargo test

# 运行特定 crate 的测试
cargo test -p <crate_name>

# 运行特定测试
cargo test <test_name>
```

### 集成测试

```bash
# 运行集成测试
cargo test --test integration_tests

# 需要数据库的测试（确保数据库已启动）
cargo test --test database_tests
```

## 🚀 部署指南

### 开发环境部署

```bash
# 启动开发数据库和 Redis
cd db_helper
docker compose up -d

# 运行应用
cargo run
```

### 生产环境部署

```bash
# 构建优化的生产版本
cargo build --release

# 使用多阶段构建 Docker 镜像
docker build -t rust-backend .
```

### 环境配置

本地开发环境配置示例：

```env
RUST_LOG=debug
DATABASE_URL=postgresql://username:password@localhost:5432/rust_backend
REDIS_URL=redis://localhost:6379
APP_PORT=8080
```

### 监控和日志

- 使用 `tracing` 进行结构化日志记录
- 支持日志级别动态调整
- 集成外部监控系统 (Prometheus/Grafana)

## 📚 相关资源

### 📋 详细文档

- **[服务生命周期](./service-lifecycle.md)** - 架构设计和优雅关闭机制详解
- **[架构流程图](./mermaid_final.md)** - 完整的系统架构可视化图表
- **[技术选型 FAQ](./faq.md)** - 为什么选择这些技术栈的详细说明

---

*🎯 本文档为 Rust 后端项目的完整开发指南，涵盖了从环境搭建到生产部署的全流程。*