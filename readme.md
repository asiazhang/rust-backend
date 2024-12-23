# Rust backend

使用Rust来开发后端服务，包含以下功能：

- `web-api`支持
- 后台任务消费者
- 定时任务

# 技术栈

- [axum](https://github.com/tokio-rs/axum): Rust Web框架
- [utoipa](https://github.com/juhaku/utoipa): OpenAPI文档生成
- [sqlx](https://github.com/launchbadge/sqlx): 支持async的数据库工具
- [tracing](https://github.com/tokio-rs/tracing): 日志框架
- [dotenvy](https://github.com/allan2/dotenvy): 本地.env文件环境变量化支持
- [anyhow](https://github.com/dtolnay/anyhow): 错误处理支持
- [validator](https://github.com/Keats/validator): 模型校验器 