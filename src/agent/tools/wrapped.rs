use crate::context;
use crate::redis;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde_json::json;
use tracing::*;

use super::web_search::{
    ModelError, ProfileUrlList, ProfileUrlListArgs, WebSearchArgs, WebSearchWithHeadlessBrowser,
};

pub struct WrappedWebSearchWithHeadlessBrowser;

impl Tool for WrappedWebSearchWithHeadlessBrowser {
    const NAME: &'static str = "web_search_with_headless_browser";
    type Error = ModelError;
    type Args = WebSearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        WebSearchWithHeadlessBrowser.definition(_prompt).await
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

        WebSearchWithHeadlessBrowser.call(args).await
    }
}

pub struct WrappedProfileUrlList;

impl Tool for WrappedProfileUrlList {
    const NAME: &'static str = "profile_url_list";
    type Error = ModelError;
    type Args = ProfileUrlListArgs;
    type Output = Vec<String>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ProfileUrlList.definition(_prompt).await
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

        ProfileUrlList.call(args).await
    }
}
