use std::sync::{Arc, Mutex};
use std::fs::File;
use std::fs;
use std::io::prelude::*;
use std::{thread, time};

use crate::approx::data::*;
use crate::approx::test_info::*;
use crate::approx::client_wrapper::*;
use crate::approx::test_log::*;
use crate::approx::mtime;

use reqwest::blocking::Client;

pub fn start_updates(
    config: Config,
    tests_info: Arc<Mutex<Vec<TestInfo>>>,
    total_info: Arc<Mutex<TestSuiteInfo>>
) {
    let mut file = File::create("err").unwrap();

    let client = ClientWrapper::new(Client::new());

    let block = match create_block(&config, &mut file, &client) {
        Some(x) => x,
        None => {
            return;
        }
    };

    let mut logs: Vec<TestLog> = Vec::new();

    while !total_info.lock().unwrap().finished {
        update_table(&config, &block, tests_info.lock().unwrap().clone(), &mut logs, &client, &mut file, None);
        thread::sleep(time::Duration::from_millis(1000));
    }
    let total_info = total_info.lock().unwrap().clone();
    update_table(&config, &block, tests_info.lock().unwrap().clone(), &mut logs, &client, &mut file, Some(&total_info));
    update_total_score(&config, &block, &mut file, &client, &total_info);
}

fn create_block(
    config: &Config,
    file: &mut File,
    client: &ClientWrapper
) -> Option<NotionBlock> {
    let notion = &config.notion.as_ref().unwrap();
    let db = &notion.database;
    let key = &notion.key;

    // creating page
    let data = serde_json::json!({
        "parent": { "database_id": db.clone() },
        "properties": {
            "Score": {
                "type": "rich_text",
                "rich_text": [{"type": "text", "text": {"content": "Running..."}}]
            },
            "Timestamp": {
                "type": "title",
                "title": [{"type": "text", "text": {"content": mtime::get_datetime(config.time_offset.unwrap())}}]
            }
        }
    });

    let response = client.client().post("https://api.notion.com/v1/pages")
        .header("Authorization", format!("Bearer {}", key))
        .header("Notion-Version", "2021-08-16")
        .json(&data)
        .build().unwrap();

    let response = client.execute(response, file);
    if !response.is_some() {
        file.write(b"Can't create page\n").unwrap();
        return None;
    }
    let data = response.unwrap();

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

    let response = client.client().patch(&format!("https://api.notion.com/v1/blocks/{}/children", page_id))
        .header("Authorization", format!("Bearer {}", key))
        .header("Notion-Version", "2021-08-16")
        .json(&data)
        .build().unwrap();

    let response = client.execute(response, file);
    if !response.is_some() {
        file.write(b"Can't create block\n").unwrap();
        return None;
    }
    let data = response.unwrap();

    let block_id = match data["results"][0]["id"].as_str() {
        Some(x) => x,
        None => {
            file.write(format!("Can't read block id from response {}\n", data).as_bytes()).unwrap();
            return None;
        }
    }.to_string();

    Some(NotionBlock {
        block_id,
        page_id,
    })
}

fn update_table(
    config: &Config,
    block: &NotionBlock,
    tests_info: Vec<TestInfo>,
    logs: &mut Vec<TestLog>,
    client: &ClientWrapper,
    file: &mut File,
    total_info: Option<&TestSuiteInfo>,
) {
    update_logs(config, block, &tests_info, logs, client, file);

    let mut content: Vec<NotionTextChunk> = Vec::new();

    let title = format!("| {: ^3} | {: ^12} | {: ^12} | {: ^12} | {: ^12} | {: ^12} |",
                             "",     "time",   "prev",    "new",  "delta",  "logs");
    let splitter: String = title.chars().map(|c| if c == '|' { '|' } else { '-' }).collect();
    content.push(NotionTextChunk::new(&(title + "\n" + &splitter + "\n"), "default"));

    for (i, test_info) in tests_info.iter().enumerate() {
        let mut current = test_info.print_to_notion(&config, &logs[i]);
        content.append(&mut current);
    }

    if let Some(total_info) = total_info {
        content.push(NotionTextChunk::new(&format!("\nTotal: {}", total_info.score), "default"));
    }
    content.push(NotionTextChunk::new(
        &format!("\nLast update: {}", mtime::get_datetime(config.time_offset.unwrap())),
        "default"
    ));

    let content = NotionTextChunk::fix_chunks_length(content);
    let data = NotionTextChunk::chunks_to_notion_content(content);

    let response = client.client().patch(&format!("https://api.notion.com/v1/blocks/{}", block.block_id))
        .header("Authorization", format!("Bearer {}", config.notion.as_ref().unwrap().key))
        .header("Notion-Version", "2021-08-16")
        .json(&data)
        .build().unwrap();

    let response = client.execute(response, file);
    if !response.is_some() {
        file.write(b"Can't update table\n").unwrap();
    }
}

fn update_total_score(
    config: &Config,
    block: &NotionBlock,
    file: &mut File,
    client: &ClientWrapper,
    total_info: &TestSuiteInfo,
) {
    let delta_color = if total_info.delta == 0.0 { "default" } else { "green" };
    let mut delta = format!("{:.10}", total_info.delta);
    if delta.chars().nth(0).unwrap() != '-' && delta_color == "green" {
        delta = "+".to_owned() + &delta;
    }
    let time = total_info.cpu_time / 1000;
    let time = format!(
        "{:0>2}:{:0>2}:{:0>2}",
        time / 60 / 60,
        time / 60 % 60,
        time % 60,
    );
    let data = serde_json::json!({
        "properties": {
            "Score": {
                "type": "rich_text",
                "rich_text":[{
                    "type": "text",
                    "text": {"content": format!("{:.10}", total_info.score)}
                }]
            },
            "Delta": {
                "type": "rich_text",
                "rich_text":[{
                    "type": "text",
                    "text": {"content": delta},
                    "annotations": {"color": delta_color}
                }]
            },
            "Cpu time": {
                "type": "rich_text",
                "rich_text":[{
                    "type": "text",
                    "text": {"content": time}
                }]
            },
        }
    });

    let response = client.client().patch(&format!("https://api.notion.com/v1/pages/{}", block.page_id))
        .header("Authorization", format!("Bearer {}", config.notion.as_ref().unwrap().key))
        .header("Notion-Version", "2021-08-16")
        .json(&data)
        .build().unwrap();

    let response = client.execute(response, file);
    if !response.is_some() {
        file.write(b"Can't update total score\n").unwrap();
    }
}

fn update_logs(
    config: &Config,
    block: &NotionBlock,
    tests_info: &Vec<TestInfo>,
    logs: &mut Vec<TestLog>,
    client: &ClientWrapper,
    file: &mut File,
) {
    while logs.len() < tests_info.len() {
        logs.push(TestLog::new(&tests_info[logs.len()].test_name));
    }

    for (i, log) in logs.iter_mut().enumerate() {
        if tests_info[i].state == TestState::Queue || tests_info[i].state == TestState::Skipped {
            continue
        }
        if let Ok(data) = fs::read_to_string(&format!("tests/{}", log.filename)) {
            if log.content.is_none() {
                log.create_page(config, block, client, file);
            }
            if log.content.is_none() {
                continue;
            }
            log.update_page(&data, config, block, client, file);
        }
    }
}
