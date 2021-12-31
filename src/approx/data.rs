use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct NotionConfig {
    pub key: String,
    pub database: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub tests: usize,
    pub optimize: String,  // "min" or "max"
    pub result_func: String,  // "sum" or "avg"
    pub skip_tests: Option<Vec<usize>>,
    pub precision: Option<usize>,

    pub threads: Option<usize>,

    pub time_offset: Option<i64>,

    // cmds
    pub main:     Option<Vec<String>>,
    pub scorer:   Option<Vec<String>>,
    pub finalize: Option<Vec<String>>,

    pub notion: Option<NotionConfig>,
}




#[derive(Debug, Clone)]
pub struct NotionTextChunk {
    pub text: String,
    pub color: String,
    pub link: Option<String>,
}

impl NotionTextChunk {
    pub fn new(text: &str, color: &str) -> Self {
        NotionTextChunk {
            text: text.to_string(),
            color: color.to_string(),
            link: None,
        }
    }
}



#[derive(Debug, Clone)]
pub struct NotionBlock {
    pub block_id: String,
    pub page_id: String,
    pub score_id: String,
}
