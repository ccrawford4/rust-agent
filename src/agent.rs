// TODO: add agent code here

use rig_derive::tool_macro;

//
// This should be a agent that can perform web searches and answer questinos about me!
//
//

enum Url {
    Home,
    About,
    Work,
}

#[derive(Deserialize)]
struct WebSearchArgs {
    url: Url,
}

#[derive(Deserialize, Serialize)]
struct WebSearch;

type ModelError = String;

/*
 * Continue with this example: https://docs.rig.rs/docs/concepts/tools
 *
 */

impl Tool for WebSearch {
    const NAME: &'static str = "web_search";
    type Error = ModelError;
    type Args = WebSearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition { name: "" }
    }
}
