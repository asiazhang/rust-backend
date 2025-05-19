# Rust backend

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/asiazhang/rust-backend)

使用Rust来开发后端服务，包含以下功能：

- `web-api`支持
- 后台任务消费者
- 定时任务

这个是一个单体应用，适合中小规模应用的开发。

# 技术栈

- [axum](https://github.com/tokio-rs/axum): Rust Web框架
- [utoipa](https://github.com/juhaku/utoipa): OpenAPI文档生成
- [sqlx](https://github.com/launchbadge/sqlx): 支持async的数据库工具
- [tracing](https://github.com/tokio-rs/tracing): 日志框架
- [dotenvy](https://github.com/allan2/dotenvy): 本地.env文件环境变量化支持
- [anyhow](https://github.com/dtolnay/anyhow): 错误处理支持
- [validator](https://github.com/Keats/validator): 模型校验器 

## 数据迁移

本地可通过`docker compose up`启动构建/测试所需的`Postgresql`和`Redis`。

- 先安装`sqlx-cli`工具：`cargo install sqlx-cli`
- 数据库创建：`sqlx database create`
- 数据迁移：`sqlx migrate run`