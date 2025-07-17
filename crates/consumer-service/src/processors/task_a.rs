use serde_json::Value;
use tokio::time::{sleep, Duration};
use tracing::info;

pub async fn process(message: &Value) -> anyhow::Result<()> {
    info!("🔄 开始处理任务A");
    
    // 模拟任务处理
    sleep(Duration::from_secs(2)).await;
    
    info!("✅ 任务A处理完成");
    Ok(())
}
