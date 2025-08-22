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

## 📚 相关文档

- **[开发指南](docs/development-guide.md)** - 详细的项目结构、开发命令和配置管理
- **[服务生命周期](docs/service-lifecycle.md)** - 架构设计和优雅关闭机制详解
- **[架构流程图](docs/mermaid_final.md)** - 完整的系统架构可视化图表
- **[技术选型 FAQ](docs/faq.md)** - 为什么选择这些技术栈的详细说明

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
