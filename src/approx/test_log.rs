use std::fs::File;
use std::io::Write;

use crate::approx::client_wrapper::*;
use crate::approx::data::*;
use crate::approx::mtime;

#[derive(Debug)]
pub struct TestLog {
    pub filename: String,
    pub content: Option<String>,
    pub page_id: String,
    pub block_id: String,
    pub last_update: String,
}

impl TestLog {
    pub fn new(test_name: &str) -> Self {
        TestLog {
            filename: format!("{}.log", test_name),
            content: None,
            page_id: String::new(),
            block_id: String::new(),
            last_update: String::new(),
        }
    }

    pub fn create_page(
        &mut self,
        config: &Config,
        block: &NotionBlock,
        client: &ClientWrapper,
        file: &mut File,
    ) {
        let key = &config.notion.as_ref().unwrap().key;

        if self.page_id.is_empty() {
            let data = serde_json::json!({
                "parent": {"type": "page_id", "page_id": block.page_id.clone()},
                "properties": {
                    "title": {"title": [{"type": "text", "text": {"content": self.filename.clone()}}]}
                }
            });

            let response = client.client().post("https://api.notion.com/v1/pages")
                .header("Authorization", format!("Bearer {}", key))
                .header("Notion-Version", "2021-08-16")
                .json(&data)
                .build().unwrap();

            let response = client.execute(response, file);
            if !response.is_some() {
                file.write(b"Can't create log page\n").unwrap();
                return;
            }
            let data = response.unwrap();
            if let Some(page_id) = data["id"].as_str() {
                self.page_id = page_id.to_string();
            } else {
                return;
            }
        }

        if self.block_id.is_empty() {
            let data = serde_json::json!({
                "children": [{
                    "object": "block",
                    "type": "code",
                    "code": {
                        "text": [{"type": "text", "text": {"content": ""}}],
                        "language": "plain text"
                    }
                }]
            });

            let response = client.client().patch(&format!("https://api.notion.com/v1/blocks/{}/children", self.page_id))
                .header("Authorization", format!("Bearer {}", key))
                .header("Notion-Version", "2021-08-16")
                .json(&data)
                .build().unwrap();

            let response = client.execute(response, file);
            if !response.is_some() {
                file.write(b"Can't create log block\n").unwrap();
                return;
            }
            let data = response.unwrap();

            self.block_id = match data["results"][0]["id"].as_str() {
                Some(x) => x,
                None => {
                    file.write(format!("Can't read log block id from response {}\n", data).as_bytes()).unwrap();
                    return;
                }
            }.to_string();
        }

        self.content = Some(String::new());
    }

    pub fn update_page(
        &mut self,
        content: &str,
        config: &Config,
        _block: &NotionBlock,
        client: &ClientWrapper,
        file: &mut File,
    ) {
        if self.content.is_none() {
            return;
        }
        if self.content.as_ref().unwrap() == content {
            return;
        }

        let date = mtime::get_date(config.time_offset.unwrap());
        let time = mtime::get_time(config.time_offset.unwrap());

        let data = NotionTextChunk::chunks_to_notion_content(NotionTextChunk::fix_chunks_length(vec![
            NotionTextChunk::new(
                &(content.to_owned() + &format!("\nLast update: {} {}", date, time)),
                "default",
            )
        ]));

        let response = client.client().patch(&format!("https://api.notion.com/v1/blocks/{}", self.block_id))
            .header("Authorization", format!("Bearer {}", config.notion.as_ref().unwrap().key))
            .header("Notion-Version", "2021-08-16")
            .json(&data)
            .build().unwrap();

        let response = client.execute(response, file);
        if !response.is_some() {
            file.write(b"Can't update log file\n").unwrap();
            return
        }

        self.content = Some(content.to_string());
        self.last_update = time;
    }
}
