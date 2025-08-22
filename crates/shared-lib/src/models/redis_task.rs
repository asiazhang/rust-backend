use serde::{Deserialize, Serialize};

/// Redis消费者心跳信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConsumerHeartBeat {
    /// Redis流名称
    pub stream_name: String,

    /// Redis消费者名称
    pub consumer_name: String,

    /// 此消费者上次心跳时间
    pub last_heartbeat: i64,
}
