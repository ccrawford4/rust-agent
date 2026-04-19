use once_cell::sync::Lazy;
use redis::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn};

static REDIS_CLIENT: Lazy<Option<Client>> = Lazy::new(|| {
    match std::env::var("REDIS_URL") {
        Ok(redis_url) => match Client::open(redis_url.as_str()) {
            Ok(client) => {
                debug!("Redis client created successfully");
                Some(client)
            }
            Err(e) => {
                error!("Failed to create Redis client: {}", e);
                None
            }
        },
        Err(_) => {
            debug!("REDIS_URL not set, Redis support disabled");
            None
        }
    }
});

pub struct ToolCall {
    pub name: String,
    pub args: serde_json::Value,
}

impl ToolCall {
    pub fn new(name: String, args: serde_json::Value) -> Self {
        Self { name, args }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCallRecord {
    pub name: String,
    pub args: serde_json::Value,
    pub timestamp: String,
}

pub async fn write_tool_call(request_id: &str, tool_call: ToolCall) -> Result<(), Box<dyn std::error::Error>> {
    let Some(client) = &*REDIS_CLIENT else {
        debug!("Redis not configured, skipping tool call write");
        return Ok(());
    };

    let mut conn = match client.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to get Redis connection: {}", e);
            return Err(Box::new(e));
        }
    };

    let record = ToolCallRecord {
        name: tool_call.name,
        args: tool_call.args,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let json_str = serde_json::to_string(&record)?;

    match redis::cmd("RPUSH")
        .arg(format!("request:{}:tool_calls", request_id))
        .arg(&json_str)
        .query_async::<_, ()>(&mut conn)
        .await
    {
        Ok(_) => {
            debug!("Tool call written to Redis for request_id: {}", request_id);
            Ok(())
        }
        Err(e) => {
            error!("Failed to write tool call to Redis: {}", e);
            Err(Box::new(e))
        }
    }
}

pub async fn get_tool_calls(request_id: &str) -> Result<Vec<ToolCallRecord>, Box<dyn std::error::Error>> {
    let Some(client) = &*REDIS_CLIENT else {
        debug!("Redis not configured, returning empty list");
        return Ok(Vec::new());
    };

    let mut conn = match client.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to get Redis connection: {}", e);
            return Err(Box::new(e));
        }
    };

    let json_strings: Vec<String> = redis::cmd("LRANGE")
        .arg(format!("request:{}:tool_calls", request_id))
        .arg(0)
        .arg(-1)
        .query_async(&mut conn)
        .await?;

    let mut records = Vec::new();
    for json_str in json_strings {
        match serde_json::from_str::<ToolCallRecord>(&json_str) {
            Ok(record) => records.push(record),
            Err(e) => {
                warn!("Failed to deserialize tool call record: {}", e);
            }
        }
    }

    Ok(records)
}
