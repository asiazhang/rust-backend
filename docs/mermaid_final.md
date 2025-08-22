# 🏗️ Rust 后端服务架构流程图

## 📋 系统架构概览

本图展示了 Rust 后端服务的完整架构，包含三个主要板块：Web API、Redis Worker 和 Cron Tasks。

## 🎯 Mermaid 流程图

```mermaid
%%{init: {'theme': 'base', 'themeVariables': { 'primaryColor': '#ff6b6b', 'primaryTextColor': '#fff', 'primaryBorderColor': '#ff6b6b', 'lineColor': '#333', 'secondaryColor': '#4ecdc4', 'tertiaryColor': '#45b7d1'}}}%%
flowchart TD
    %% 外部系统
    Client[👤 客户端<br/>Client]
    OS[💻 操作系统<br/>OS/Signals]
    
    %% Web API 板块
    subgraph WebAPI["🌐 Web API 服务"]
        direction TB
        WebServer[🚀 Web 服务器<br/>Axum Server<br/>:8080]
        AppState[📦 应用状态<br/>App State]
        Router[🔀 路由器<br/>API Router]
        Middleware[🛡️ 中间件<br/>Middleware]
        
        WebServer --> AppState
        AppState --> Router
        Router --> Middleware
    end
    
    %% Redis Worker 板块
    subgraph RedisWorker["⚡ Redis Worker 服务"]
        direction TB
        TaskConsumerA[🔄 任务消费者 A<br/>TaskTypeA Consumer]
        TaskConsumerB[🔄 任务消费者 B<br/>TaskTypeB Consumer]
        WorkerLoop[🔁 工作循环<br/>Worker Loop]
        Heartbeat[💓 心跳机制<br/>Heartbeat<br/>30s 间隔]
        
        TaskConsumerA --> WorkerLoop
        TaskConsumerB --> WorkerLoop
        WorkerLoop --> Heartbeat
    end
    
    %% Cron Tasks 板块
    subgraph CronTasks["⏰ Cron Tasks 服务"]
        direction TB
        Scheduler[📅 任务调度器<br/>Job Scheduler]
        RebalanceTask[⚖️ 重平衡任务<br/>Rebalance Task<br/>每 10 秒]
        CronWorker[🔧 定时任务执行器<br/>Cron Worker]
        
        Scheduler --> RebalanceTask
        RebalanceTask --> CronWorker
    end
    
    %% 数据存储
    subgraph Storage["🗄️ 数据存储"]
        direction TB
        Postgres[(🐘 PostgreSQL<br/>Database)]
        Redis[(🔴 Redis<br/>Message Queue)]
        
        Postgres -.-> Redis
    end
    
    %% 信号处理
    subgraph SignalHandler["🛑 信号处理"]
        direction TB
        ShutdownSignal[🚨 关闭信号<br/>Shutdown Signal]
        WatchChannel[📡 监听通道<br/>Watch Channel]
        GracefulShutdown[🤝 优雅关闭<br/>Graceful Shutdown]
        
        ShutdownSignal --> WatchChannel
        WatchChannel --> GracefulShutdown
    end
    
    %% 连接关系
    Client ==>|HTTP 请求| WebServer
    WebServer ==>|SQL 查询| Postgres
    WebServer ==>|发布消息| Redis
    
    Redis ==>|消息队列| TaskConsumerA
    Redis ==>|消息队列| TaskConsumerB
    Redis ==>|存储心跳| Heartbeat
    
    CronWorker ==>|重平衡操作| Redis
    
    OS ==>|信号| ShutdownSignal
    
    %% 优雅关闭连接
    GracefulShutdown -.->|关闭信号| WebServer
    GracefulShutdown -.->|关闭信号| TaskConsumerA
    GracefulShutdown -.->|关闭信号| TaskConsumerB
    GracefulShutdown -.->|关闭信号| Scheduler
    
    %% 样式定义
    classDef webapi fill:#e3f2fd,stroke:#1976d2,stroke-width:2px,color:#0d47a1
    classDef worker fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px,color:#4a148c
    classDef cron fill:#fff3e0,stroke:#f57c00,stroke-width:2px,color:#e65100
    classDef storage fill:#e8f5e8,stroke:#388e3c,stroke-width:2px,color:#1b5e20
    classDef signal fill:#ffebee,stroke:#d32f2f,stroke-width:2px,color:#c62828
    classDef external fill:#fafafa,stroke:#616161,stroke-width:2px,color:#424242
    
    %% 应用样式
    class WebServer,AppState,Router,Middleware webapi
    class TaskConsumerA,TaskConsumerB,WorkerLoop,Heartbeat worker
    class Scheduler,RebalanceTask,CronWorker cron
    class Postgres,Redis storage
    class ShutdownSignal,WatchChannel,GracefulShutdown signal
    class Client,OS external
```

## 🎨 架构说明

### 🌐 Web API 服务
- **功能**: 处理 HTTP 请求，提供 REST API 接口
- **端口**: 8080
- **组件**: Axum Web 服务器 + 路由器 + 中间件
- **特性**: 支持 OpenAPI 文档生成，优雅关闭

### ⚡ Redis Worker 服务  
- **功能**: 处理异步任务消费
- **消费者**: TaskTypeA 和 TaskTypeB 两种任务类型
- **特性**: 心跳机制（30秒间隔）、并发处理（最多5条消息）、自动重连

### ⏰ Cron Tasks 服务
- **功能**: 执行定时任务
- **任务**: Redis 消息重平衡（每10秒执行）
- **调度器**: 基于 tokio-cron-scheduler

### 🗄️ 数据存储
- **PostgreSQL**: 主数据库，存储业务数据
- **Redis**: 消息队列 + 缓存，支持任务队列和心跳存储

### 🛑 信号处理
- **监听**: Ctrl+C 和 SIGTERM 信号
- **通知**: 通过 watch::channel 广播关闭信号
- **优雅关闭**: 确保所有服务完成当前工作后再退出

## 🔧 预览方式

您可以通过以下方式预览此 Mermaid 图：

1. **🌐 在线预览**: 
   - 访问 [mermaid.live](https://mermaid.live) 
   - 复制上述 Mermaid 代码到编辑器

2. **💻 本地预览**:
   - GitHub/GitLab 仓库中直接渲染
   - VS Code 安装 Mermaid 预览插件
   - 支持 Mermaid 的 Markdown 编辑器

3. **📱 移动端**:
   - GitHub Mobile 应用
   - 支持 Mermaid 的移动端 Markdown 查看器

## 🚀 技术特性

- ✅ **并发处理**: 使用 `tokio::try_join!` 实现真正的并发启动
- ✅ **优雅关闭**: 避免数据丢失和资源泄露  
- ✅ **健康检查**: Redis 消费者定期发送心跳
- ✅ **错误处理**: 使用 `color-eyre` 提供详细的错误信息
- ✅ **性能优化**: 消息批处理、并发限制、连接复用

---

*🎯 此流程图展示了现代异步 Rust 应用的最佳实践架构。*
