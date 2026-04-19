use crate::environment::Environment;
use headless_chrome::Browser;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;
use std::fmt;
use std::time::Duration;
use tracing::*;

/// Valid URLs for the portfolio site sections.
#[derive(Debug, Clone, Serialize)]
pub enum ProfileUrl {
    About,
    Work,
    Projects,
    Contact,
}

fn get_portfolio_host() -> String {
    return "https://about.calum.sh".to_string();
}

impl ProfileUrl {
    pub fn as_url(&self) -> String {
        let host = get_portfolio_host();
        match self {
            ProfileUrl::About => format!("{}/?tab=About", host),
            ProfileUrl::Work => format!("{}/?tab=Work", host),
            ProfileUrl::Projects => format!("{}/?tab=Projects", host),
            ProfileUrl::Contact => format!("{}/?tab=Contact", host),
        }
    }
}

impl fmt::Display for ProfileUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_url())
    }
}

struct ProfileUrlVisitor;

impl<'de> Visitor<'de> for ProfileUrlVisitor {
    type Value = ProfileUrl;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid URL string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "https://about.calum.sh/?tab=About" | "http:///?tab=About" => Ok(ProfileUrl::About),
            "https://about.calum.sh/?tab=Work" | "http:///?tab=Work" => Ok(ProfileUrl::Work),
            "https://about.calum.sh/?tab=Projects" | "http:///?tab=Projects" => {
                Ok(ProfileUrl::Projects)
            }
            "https://about.calum.sh/?tab=Contact" | "http:///?tab=Contact" => {
                Ok(ProfileUrl::Contact)
            }
            _ => Err(de::Error::unknown_variant(
                value,
                &[
                    "https://about.calum.sh/?tab=About",
                    "https://about.calum.sh/?tab=Work",
                    "https://about.calum.sh/?tab=Projects",
                    "https://about.calum.sh/?tab=Contact",
                ],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for ProfileUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ProfileUrlVisitor)
    }
}

/// Arguments for the WebSearch tool
#[derive(Deserialize, Serialize)]
pub struct WebSearchArgs {
    url: ProfileUrl,
}

/// Tool for fetching content from portfolio website sections.
#[derive(Deserialize, Serialize)]
pub struct WebSearch;

/// Error type for tool execution failures
#[derive(Debug)]
pub struct ModelError(String);

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for ModelError {}

impl Tool for WebSearch {
    const NAME: &'static str = "web_search";
    type Error = ModelError;
    type Args = WebSearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "web_search",
            "description": "search the web for information about the user",
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "url to search"
                    }
                },
                "required": ["url"]
            }
        }))
        .unwrap_or_else(|e| {
            error!("Critical error: Failed to create tool definition: {}", e);
            panic!(
                "Invalid static tool definition - this is a programming error: {}",
                e
            );
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("Fetching web content from: {}", args.url);

        let response = reqwest::get(args.url.as_url()).await.map_err(|e| {
            error!("Error fetching URL {}: {}", args.url, e);

            let mut source = e.source();
            while let Some(err) = source {
                error!("  caused by: {}", err);
                source = err.source();
            }

            ModelError(e.to_string())
        })?;

        let body = response.text().await.map_err(|e| {
            error!("Error reading response body: {}", e);
            ModelError(e.to_string())
        })?;

        debug!(
            "Successfully fetched web page content ({} bytes)",
            body.len()
        );

        Ok(body)
    }
}

/// Tool for listing available portfolio URLs.
pub struct ProfileUrlList;

/// Arguments for the ProfileUrlList tool (no arguments required)
#[derive(Debug, Deserialize, Serialize)]
pub struct ProfileUrlListArgs {}

impl Tool for ProfileUrlList {
    const NAME: &'static str = "profile_url_list";
    type Error = ModelError;
    type Args = ProfileUrlListArgs;
    type Output = Vec<String>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "profile_url_list",
            "description": "list of available profile URLs about calum (cal) crawford",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }))
        .unwrap_or_else(|e| {
            error!("Critical error: Failed to create tool definition: {}", e);
            panic!(
                "Invalid static tool definition - this is a programming error: {}",
                e
            );
        })
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        debug!("Providing list of profile URLs");
        let result = vec![
            ProfileUrl::About.as_url(),
            ProfileUrl::Work.as_url(),
            ProfileUrl::Projects.as_url(),
            ProfileUrl::Contact.as_url(),
        ];
        debug!("Providing profile URL list: {:?}", result);

        Ok(result)
    }
}

/// Tool for fetching content from portfolio website sections with a headless browser.
#[derive(Deserialize, Serialize)]
pub struct WebSearchWithHeadlessBrowser;

impl Tool for WebSearchWithHeadlessBrowser {
    const NAME: &'static str = "web_search_with_headless_browser";
    type Error = ModelError;
    type Args = WebSearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "web_search_with_headless_browser",
            "description": "search the web using a headless browswer (for sites that require javascript)",
            "parameters": {
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "url to search"
                    }
                },
                "required": ["url"]
            }
        }))
        .unwrap_or_else(|e| {
            error!("Critical error: Failed to create tool definition: {}", e);
            panic!(
                "Invalid static tool definition - this is a programming error: {}",
                e
            );
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("Fetching web content from: {}", args.url);

        let browser = Browser::default().map_err(|e| {
            error!("Error launching headless browser: {}", e);
            ModelError("Could not spin up headless browser!".to_string())
        })?;

        let tab = browser.new_tab().map_err(|e| {
            error!("Error creating new tab in headless browser: {}", e);
            ModelError("Could not create new tab in headless browser!".to_string())
        })?;

        let _nav_result = tab.navigate_to(&args.url.to_string()).map_err(|e| {
            error!("Error navigating to URL {}: {}", args.url, e);
            ModelError("Could not navigate to url".to_string())
        })?;

        tab.wait_for_element_with_custom_timeout("main", Duration::from_secs(5))
            .map_err(|e| {
                error!("Timeout waiting for content to render: {}", e);
                ModelError("Content did not render in time".to_string())
            })?;

        std::thread::sleep(Duration::from_millis(200));

        tab.get_content().map_err(|e| {
            error!("Error getting content from URL {}: {}", args.url, e);
            ModelError("Could not read content from the site".to_string())
        })
    }
}
