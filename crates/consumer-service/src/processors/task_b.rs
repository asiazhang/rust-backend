use serde_json::Value;
use tokio::time::{sleep, Duration};
use tracing::info;

pub async fn process(message: &Value) -> anyhow::Result<()> {
    info!("🔄 开始处理任务B");
    
    // 模拟任务处理
    sleep(Duration::from_secs(3)).await;
    
    info!("✅ 任务B处理完成");
    Ok(())
}
