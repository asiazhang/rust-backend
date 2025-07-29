//! Redis分布式锁工具模块
//!
//! 提供通用的Redis分布式锁功能，支持：
//! - 自动过期锁
//! - 锁获取和释放
//! - 基于Redis的SETNX命令实现

use redis::aio::ConnectionManager;
use redis::{AsyncCommands, ExistenceCheck, SetExpiry, SetOptions};
use std::time::Duration;

/// 分布式锁管理器
pub struct DistributedLock {
    conn: ConnectionManager,
    lock_key: String,
    lock_ttl: Duration,
}

impl DistributedLock {
    /// 创建新的分布式锁管理器
    pub fn new(conn: ConnectionManager, lock_key: String, lock_ttl: Duration) -> Self {
        Self {
            conn,
            lock_key,
            lock_ttl,
        }
    }

    /// 尝试获取锁
    /// 
    /// 返回 `LockGuard` 如果成功获取锁，否则返回 `None`
    pub async fn try_acquire(&mut self) -> Result<Option<LockGuard>, redis::RedisError> {
        let result: Option<String> = self
            .conn
            .set_options(
                &self.lock_key,
                "locked",
                SetOptions::default()
                    .conditional_set(ExistenceCheck::NX)
                    .get(true)
                    .with_expiration(SetExpiry::EX(self.lock_ttl.as_secs())),
            )
            .await?;

        if result.is_some() {
            Ok(Some(LockGuard::new(self.conn.clone(), self.lock_key.clone())))
        } else {
            Ok(None)
        }
    }

  }

/// 锁守卫，使用RAII模式自动释放锁
pub struct LockGuard {
    conn: ConnectionManager,
    lock_key: String,
}

impl LockGuard {
    fn new(conn: ConnectionManager, lock_key: String) -> Self {
        Self { conn, lock_key }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // 在析构时尝试释放锁
        // 注意：这可能会失败，但我们无法在Drop中处理错误
        let conn = self.conn.clone();
        let lock_key = self.lock_key.clone();
        
        // 使用spawn而不是block_on，避免阻塞当前线程
        tokio::spawn(async move {
            let mut conn = conn;
            if let Err(e) = conn.del::<&str, i32>(&lock_key).await {
                tracing::warn!("⚠️ 自动释放锁失败: {} (key: {})", e, lock_key);
            }
        });
    }
}

/// 便捷函数：执行带锁的操作
/// 
/// # 参数
/// - `conn`: Redis连接管理器
/// - `lock_key`: 锁的键名
/// - `lock_ttl`: 锁的过期时间
/// - `operation`: 需要在锁保护下执行的操作
/// 
/// # 返回值
/// 返回操作的结果，如果获取锁失败则返回 `None`
pub async fn execute_with_lock<T, F>(
    conn: &mut ConnectionManager,
    lock_key: &str,
    lock_ttl: Duration,
    operation: F,
) -> Result<Option<T>, redis::RedisError>
where
    F: std::future::Future<Output = T>,
{
    let mut lock_manager = DistributedLock::new(conn.clone(), lock_key.to_string(), lock_ttl);
    
    match lock_manager.try_acquire().await? {
        Some(_guard) => {
            // 锁获取成功，执行操作
            // 注意：guard会在作用域结束时自动释放锁
            Ok(Some(operation.await))
        }
        None => {
            // 锁获取失败
            Ok(None)
        }
    }
}


