use crate::context;
use crate::redis;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde_json::json;
use tracing::*;

use super::portfolio_api_search::{ModelError, PortfolioAPISearch, PortfolioAPISearchArgs};

pub struct WrappedPortfolioAPISearch;

impl Tool for WrappedPortfolioAPISearch {
    const NAME: &'static str = "portfolio_api_search";
    type Error = ModelError;
    type Args = PortfolioAPISearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        PortfolioAPISearch.definition(_prompt).await
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if let Some(request_id) = context::get_request_id() {
            let tool_call = redis::ToolCall::new(
                Self::NAME.to_string(),
                serde_json::to_value(&args).unwrap_or(json!({})),
            );

            if let Err(e) = redis::write_tool_call(&request_id, tool_call).await {
                error!("Failed to write tool call to Redis: {}", e);
            }
        }

        PortfolioAPISearch.call(args).await
    }
}
