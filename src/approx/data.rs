use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone)]
pub struct NotionConfig {
    pub key: String,
    pub database: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RemoteConfig {
    pub local_port: u16,
    pub workdir: String,
    pub copy: Vec<String>,
    #[serde(default)]
    pub prerun: Vec<Vec<String>>,
    pub threads: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub tests: usize,
    pub optimize: String,    // "min" or "max"
    pub result_func: String, // "sum" or "avg"
    pub skip_tests: Option<Vec<usize>>,
    pub precision: Option<usize>,

    pub threads: Option<usize>,

    pub time_offset: Option<i64>,

    // cmds
    pub main: Option<Vec<String>>,
    pub scorer: Option<Vec<String>>,
    pub finalize: Option<Vec<String>>,

    pub notion: Option<NotionConfig>,
    pub remote: Option<RemoteConfig>,
}

#[derive(Debug, Clone)]
pub struct TestSuiteInfo {
    pub score: f64,
    pub delta: f64,
    pub cpu_time: u128,
    pub finished: bool,
}

#[derive(Debug, Clone)]
pub struct NotionBlock {
    pub block_id: String,
    pub page_id: String,
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

    pub fn fix_chunks_length(chunks: Vec<NotionTextChunk>) -> Vec<NotionTextChunk> {
        // first merge consecutive chunks with same color
        let mut merged_chunks: Vec<NotionTextChunk> = Vec::new();
        for chunk in chunks.into_iter() {
            if merged_chunks.is_empty()
                || merged_chunks.last().unwrap().color != chunk.color
                || merged_chunks.last().unwrap().link.is_some()
                || chunk.link.is_some()
            {
                merged_chunks.push(chunk);
            } else {
                merged_chunks.last_mut().unwrap().text += &chunk.text;
            }
        }

        let mut result: Vec<NotionTextChunk> = Vec::new();
        let max_chunk_length: usize = 1000;
        for chunk in merged_chunks.into_iter() {
            for i in (0..chunk.text.len()).step_by(max_chunk_length) {
                result.push(NotionTextChunk {
                    text: chunk.text[i..(i + max_chunk_length).min(chunk.text.len())].to_string(),
                    color: chunk.color.clone(),
                    link: chunk.link.clone(),
                })
            }
        }
        result
    }

    pub fn chunks_to_notion_content(chunks: Vec<NotionTextChunk>) -> Value {
        let mut json_content: Vec<Value> = Vec::new();
        for item in chunks {
            let mut data = serde_json::json!({
                "type": "text", "text": {"content": item.text.clone()}, "annotations": {"color": item.color.clone()}
            });
            if let Some(link) = item.link {
                data["text"]["link"]["url"] = serde_json::Value::String(link.clone());
            }
            json_content.push(data);
        }

        serde_json::json!({
            "code": {
                "text": json_content,
                "language": "plain text"
            }
        })
    }
}
