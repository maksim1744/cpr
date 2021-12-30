use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::prelude::*;
use std::{thread, time};

use serde_json::Value;

use chrono::Local;

use crate::approx::data::*;
use crate::approx::test_info::*;

#[derive(Debug, Clone)]
struct NotionBlock {
    pub block_id: String,
    pub page_id: String,
    pub score_id: String,
}

pub fn start_updates(
    config: Config,
    tests_info: Arc<Mutex<Vec<TestInfo>>>,
    total_score: Arc<Mutex<Option<String>>>
) {
    let mut file = File::create("err").unwrap();

    let client = reqwest::blocking::Client::new();

    let block = match create_block(&config.notion.as_ref().unwrap(), &mut file, &client) {
        Some(x) => x,
        None => {
            return;
        }
    };

    while total_score.lock().unwrap().is_none() {
        update_table(&config, &block, tests_info.lock().unwrap().clone(), &client, &mut file, &None);
        thread::sleep(time::Duration::from_millis(1000));
    }
    let total_score = total_score.lock().unwrap().clone();
    update_table(&config, &block, tests_info.lock().unwrap().clone(), &client, &mut file, &total_score);
    update_total_score(&config, &block, &mut file, &client, &total_score.unwrap());
}

fn create_block(
    notion: &NotionConfig,
    file: &mut File,
    client: &reqwest::blocking::Client
) -> Option<NotionBlock> {
    let db = &notion.database;
    let key = &notion.key;

    // querying db for columns
    let response = client.get(&format!("https://api.notion.com/v1/databases/{}", db))
        .header("Authorization", format!("Bearer {}", key))
        .header("Notion-Version", "2021-08-16")
        .send().unwrap();

    if !response.status().is_success() {
        file.write(b"Can't read database\n").unwrap();
        return None;
    }

    let data: Value = match serde_json::from_str(&response.text().unwrap()) {
        Ok(x) => x,
        Err(e) => {
            file.write(format!("Can't parse json from response, {}\n", e).as_bytes()).unwrap();
            return None;
        }
    };

    let timestamp_id = match data["properties"]["Timestamp"]["id"].as_str() {
        Some(x) => x,
        None => {
            file.write(b"Can't read timestamp id from response\n").unwrap();
            return None;
        }
    }.to_string();
    let score_id = match data["properties"]["Score"]["id"].as_str() {
        Some(x) => x,
        None => {
            file.write(b"Can't read score id from response\n").unwrap();
            return None;
        }
    }.to_string();

    // creating page
    let data = serde_json::json!({
        "parent": { "database_id": db.clone() },
        "properties": {
            "Score": {
                "id": score_id.clone(),
                "type": "rich_text",
                "rich_text": [{"type": "text", "text": {"content": "Running..."}}]
            },
            "Timestamp": {
                "id": timestamp_id.clone(),
                "type": "title",
                "title": [{"type": "text", "text": {"content": Local::now().format("%Y-%m-%d %H:%M:%S").to_string()}}]
            }
        }
    });

    let response = client.post("https://api.notion.com/v1/pages")
        .header("Authorization", format!("Bearer {}", key))
        .header("Notion-Version", "2021-08-16")
        .json(&data)
        .send().unwrap();

    if !response.status().is_success() {
        file.write(b"Can't create page\n").unwrap();
        return None;
    }

    let data: Value = match serde_json::from_str(&response.text().unwrap()) {
        Ok(x) => x,
        Err(e) => {
            file.write(format!("Can't parse json from create-page response, {}\n", e).as_bytes()).unwrap();
            return None;
        }
    };

    let page_id = match data["id"].as_str() {
        Some(x) => x,
        None => {
            file.write(b"Can't read page id from response\n").unwrap();
            return None;
        }
    }.to_string();

    // creating block
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

    let response = client.patch(&format!("https://api.notion.com/v1/blocks/{}/children", page_id))
        .header("Authorization", format!("Bearer {}", key))
        .header("Notion-Version", "2021-08-16")
        .json(&data)
        .send().unwrap();

    if !response.status().is_success() {
        file.write(b"Can't create block\n").unwrap();
        return None;
    }

    let response = response.text().unwrap();
    let data: Value = match serde_json::from_str(&response) {
        Ok(x) => x,
        Err(e) => {
            file.write(format!("Can't parse json from create-block response, {}\n", e).as_bytes()).unwrap();
            return None;
        }
    };

    let block_id = match data["results"][0]["id"].as_str() {
        Some(x) => x,
        None => {
            file.write(format!("Can't read block id from response {}\n", response).as_bytes()).unwrap();
            return None;
        }
    }.to_string();

    Some(NotionBlock {
        block_id,
        page_id,
        score_id,
    })
}

fn update_table(
    config: &Config,
    block: &NotionBlock,
    tests_info: Vec<TestInfo>,
    client: &reqwest::blocking::Client,
    file: &mut File,
    total_score: &Option<String>,
) {
    let mut content: Vec<NotionTextChunk> = Vec::new();

    let title = format!("| {: ^3} | {: ^12} | {: ^12} | {: ^12} | {: ^12} |", "", "time", "prev", "new", "delta");
    let splitter: String = title.chars().map(|c| if c == '|' { '|' } else { '-' }).collect();
    content.push(NotionTextChunk{
        text: title + "\n" + &splitter + "\n",
        color: "default".to_string(),
    });

    for test_info in tests_info {
        let mut current = test_info.print_to_notion(&config);
        content.append(&mut current);
    }

    if let Some(score) = total_score {
        content.push(NotionTextChunk::new(&format!("\nTotal: {}", score), "default"));
    }
    content.push(NotionTextChunk::new(&format!("\nLast update: {}", Local::now().format("%Y-%m-%d %H:%M:%S")), "default"));

    let mut merged_chunks: Vec<NotionTextChunk> = Vec::new();
    for chunk in content.into_iter() {
        if merged_chunks.is_empty() || merged_chunks.last().unwrap().color != chunk.color {
            merged_chunks.push(chunk);
        } else {
            merged_chunks.last_mut().unwrap().text += &chunk.text;
        }
    }

    let mut json_content: Vec<Value> = Vec::new();
    for item in merged_chunks {
        let data = serde_json::json!({
            "type": "text", "text": {"content": item.text.clone()}, "annotations": {"color": item.color.clone()}
        });
        json_content.push(data);
    }

    let data = serde_json::json!({
        "code": {
            "text": json_content,
            "language": "plain text"
        }
    });

    let response = client.patch(&format!("https://api.notion.com/v1/blocks/{}", block.block_id))
        .header("Authorization", format!("Bearer {}", config.notion.as_ref().unwrap().key))
        .header("Notion-Version", "2021-08-16")
        .json(&data)
        .send().unwrap();

    if !response.status().is_success() {
        file.write(format!(
            "[{}] Can't update table, {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            response.text().unwrap()).as_bytes()).unwrap();
    }
}

fn update_total_score(
    config: &Config,
    block: &NotionBlock,
    file: &mut File,
    client: &reqwest::blocking::Client,
    total_score: &str,
) {
    let data = serde_json::json!({
        "properties": {
            "Score": {
                "id": block.score_id.clone(),
                "type": "rich_text",
                "rich_text":[{"type": "text", "text": {"content": total_score.to_string().clone()}}]
            }
        }
    });

    for _ in 0..3 {
        let response = client.patch(&format!("https://api.notion.com/v1/pages/{}", block.page_id))
            .header("Authorization", format!("Bearer {}", config.notion.as_ref().unwrap().key))
            .header("Notion-Version", "2021-08-16")
            .json(&data)
            .send().unwrap();

        if response.status().is_success() {
            break;
        }

        file.write(&format!("Can't update page, {}\n", response.text().unwrap()).as_bytes()).unwrap();
        thread::sleep(time::Duration::from_millis(1000));
    }
}
