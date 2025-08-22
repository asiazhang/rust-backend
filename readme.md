# 🚀 Rust Backend

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/asiazhang/rust-backend)

基于 Rust 的现代化后端服务框架，专为中小规模应用设计。采用单体架构，集成 Web API、消息队列处理和定时任务功能。

## ✨ 核心特性

- 🌐 **Web API 服务** - 基于 Axum 框架，支持 OpenAPI 文档
- ⚡ **消息队列** - Redis Streams 异步消息处理
- ⏰ **定时任务** - 基于 tokio-cron-scheduler 的任务调度
- 🗄️ **数据库** - PostgreSQL + sqlx 无 ORM 数据访问
- 🛡️ **优雅关闭** - 完整的信号处理和资源清理
- 💾 **性能优化** - mimalloc 全局内存分配器
- 🔒 **分布式锁** - Redis 分布式锁支持

## 🚀 快速开始

### 环境要求

- **Rust**: 1.70+ (推荐使用最新稳定版)
- **PostgreSQL**: 13+
- **Redis**: 6+
- **Docker**: 用于本地开发环境

### 开发环境搭建

1. **克隆项目**
   ```bash
   git clone <repository-url>
   cd rust-backend
   ```

2. **启动依赖服务**
   ```bash
   cd db_helper && docker compose up -d
   cd ..
   ```

3. **安装数据库工具**
   ```bash
   cargo install sqlx-cli
   ```

4. **初始化数据库**
   ```bash
   sqlx database create
   sqlx migrate run
   ```

5. **配置环境变量**
   ```bash
   cp .env.example .env  # 如果有模板文件
   # 编辑 .env 文件配置数据库和Redis连接
   ```

6. **启动项目**
   ```bash
   cargo run
   ```

### 验证安装

- 🌐 **Web API**: http://localhost:8080
- 📖 **API文档**: http://localhost:8080/swagger-ui/
- 🗄️ **数据库**: localhost:5432
- 📨 **Redis**: localhost:6379

## 🏗️ 架构设计

### 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Rust Backend (单体应用)                     │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │  Web API    │  │  Job Consumer│  │   Cron Tasks    │   │
│  │   Service   │  │    Service   │  │     Service      │   │
│  │  (Axum)     │  │   (Redis)    │  │  (Scheduler)     │   │
│  └──────┬──────┘  └──────┬───────┘  └────────┬─────────┘   │
│         │                 │                    │            │
│  ┌──────┴─────────────────┴────────────────────┴──────┐    │
│  │                 Shared Library                   │    │
│  │  • Config • Models • Traits • Utils              │    │
│  └─────────────────┬─────────────────────────────────┘    │
│                    │                                    │
│  ┌─────────────────┴─────────────────────────────────┐    │
│  │                Database Layer                     │    │
│  │  • Connection • Repositories • Models           │    │
│  └───────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                            │
              ┌─────────────┼─────────────┐
              │             │             │
        ┌─────▼─────┐ ┌─────▼─────┐ ┌─────▼─────┐
        │PostgreSQL │ │   Redis   │ │  Redis    │
        │  Database │ │  Streams  │ │   Lock    │
        └───────────┘ └───────────┘ └───────────┘
```

### 技术栈详解

#### 核心框架
- **[axum](https://github.com/tokio-rs/axum)** - 现代、快速的 Web 框架，基于 Tokio
- **[tokio](https://github.com/tokio-rs/tokio)** - 异步运行时，提供高性能 I/O
- **[sqlx](https://github.com/launchbadge/sqlx)** - 编译时 SQL 检查的异步数据库库
- **[redis](https://github.com/redis-rs/redis-rs)** - Redis 客户端，支持连接池

#### API & 文档
- **[utoipa](https://github.com/juhaku/utoipa)** - OpenAPI 3.0 文档生成
- **[validator](https://github.com/Keats/validator)** - 数据验证和序列化
- **[serde](https://github.com/serde-rs/serde)** - JSON 序列化/反序列化

#### 开发工具
- **[tracing](https://github.com/tokio-rs/tracing)** - 结构化日志记录
- **[color-eyre](https://github.com/eyre-rs/color-eyre)** - 彩色错误报告
- **[dotenvy](https://github.com/allan2/dotenvy)** - 环境变量管理
- **[mimalloc](https://github.com/microsoft/mimalloc)** - 高性能内存分配器

#### 任务调度
- **[tokio-cron-scheduler](https://github.com/mvniekerk/tokio-cron-scheduler)** - Cron 任务调度
- **[chrono](https://github.com/chronotope/chrono)** - 日期时间处理
- **[time](https://github.com/time-rs/time)** - 时间处理库

### 设计理念

- **简洁高效** - 单体应用，部署简单，维护成本低
- **异步优先** - 基于 Tokio 的全异步架构，高并发处理
- **类型安全** - 充分利用 Rust 类型系统，编译时错误检查
- **性能优化** - mimalloc 分配器、连接池、零拷贝设计
- **优雅关闭** - 完整的信号处理和资源清理机制

## 🛠️ 开发指南

### 项目结构

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

### 开发命令

#### 基础命令
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

#### 数据库操作
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

### 配置管理

#### 环境变量配置
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

#### 配置结构
- `AppConfig` - 应用主配置
- `DatabaseConfig` - 数据库配置
- `RedisConfig` - Redis 配置

### 数据库开发

#### 迁移系统
- 使用 `sqlx` 迁移系统管理数据库版本
- 迁移文件位于 `migrations/` 目录
- 支持向上和向下迁移

#### 数据模型
- 在 `crates/database/src/models/` 定义
- 使用 `sqlx::FromRow` 自动映射
- 支持关联关系和验证

#### 仓储模式
- `ProjectRepositoryTrait` 定义数据访问接口
- 实现具体的 CRUD 操作
- 支持事务处理

### 消息队列开发

#### Redis Streams
- 使用 Redis Streams 作为消息队列
- 支持消费者组和消息确认
- 自动重试和错误处理

#### 消费者类型
- `TaskTypeA` - A 类型消息处理器
- `TaskTypeB` - B 类型消息处理器
- 支持并发处理和批量操作

### Web API 开发

#### 路由设计
- 基于 Axum 的路由系统
- 支持嵌套路由和中间件
- 自动生成 OpenAPI 文档

#### 服务层
- 业务逻辑与路由分离
- 使用 Trait 进行依赖注入
- 支持单元测试和集成测试

#### 错误处理
- 统一的错误响应格式
- 支持自定义错误类型
- 自动 HTTP 状态码映射

## 🚀 部署指南

### 开发环境部署

#### 本地 Docker 环境
```bash
# 启动开发数据库和 Redis
cd db_helper
docker compose up -d

# 运行应用
cargo run
```

### 生产环境部署

#### 构建优化
```bash
# 构建优化的生产版本
cargo build --release

# 使用多阶段构建 Docker 镜像
docker build -t rust-backend .
```

#### 环境配置
生产环境建议使用以下配置：
```env
RUST_LOG=info
DATABASE_URL=postgresql://user:pass@prod-db:5432/rust_backend
REDIS_URL=redis://prod-redis:6379
APP_PORT=8080
```

#### 监控和日志
- 使用 `tracing` 进行结构化日志记录
- 支持日志级别动态调整
- 集成外部监控系统 (Prometheus/Grafana)

## 📖 API 文档

### 访问方式

启动服务后，可通过以下方式访问 API 文档：

- **Swagger UI**: http://localhost:8080/swagger-ui/
- **OpenAPI JSON**: http://localhost:8080/api-docs/openapi.json
- **ReDoc**: http://localhost:8080/redoc/

### 主要端点

#### 项目管理
- `GET /api/projects` - 获取项目列表
- `POST /api/projects` - 创建新项目
- `GET /api/projects/{id}` - 获取项目详情
- `PUT /api/projects/{id}` - 更新项目
- `DELETE /api/projects/{id}` - 删除项目

#### 用户管理
- `GET /api/users` - 获取用户列表
- `POST /api/users` - 创建用户
- `GET /api/users/{id}` - 获取用户详情

## 📚 相关文档

### 📋 详细文档

- **[服务生命周期](docs/service-lifecycle.md)** - 架构设计和优雅关闭机制详解
- **[架构流程图](docs/mermaid_final.md)** - 完整的系统架构可视化图表
- **[技术选型 FAQ](docs/faq.md)** - 为什么选择这些技术栈的详细说明

### 🔗 技术资源

#### 核心框架
- [Axum 官方文档](https://docs.rs/axum/latest/axum/)
- [Tokio 官方文档](https://tokio.rs/docs)
- [SQLx 文档](https://github.com/launchbadge/sqlx)
- [Redis 文档](https://redis.io/documentation)

#### 开发工具
- [Rust 官方文档](https://doc.rust-lang.org/)
- [Cargo 使用指南](https://doc.rust-lang.org/cargo/)
- [OpenAPI 规范](https://swagger.io/specification/)

### 🤝 贡献指南

#### 开发规范
- 遵循 Rust 官方代码风格
- 使用 `cargo fmt` 格式化代码
- 编写单元测试和集成测试
- 更新相关文档

#### 提交规范
- 使用清晰的提交信息
- 包含相关的 issue 编号
- 遵循 [Conventional Commits](https://www.conventionalcommits.org/) 规范

### 📄 许可证

本项目采用 MIT 许可证，详情请参见 [LICENSE](LICENSE) 文件。

---

<div align="center">

**🚀 用 Rust 构建现代化后端服务**

[⭐ Star](../../stargazers) | [🐛 报告问题](../../issues) | [💡 功能请求](../../issues/new)

Made with ❤️ by [AsiaZhang](https://github.com/asiazhang)

</div>
