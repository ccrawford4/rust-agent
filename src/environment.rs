pub struct Environment {
    pub openai_api_key: String,
}

impl Environment {
    pub fn new() -> Self {
        let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        Environment { openai_api_key }
    }
}
