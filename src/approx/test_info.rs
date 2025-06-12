use std::io::{Stdout, Write};

use crate::approx::data::*;
use crate::approx::test_log::*;

use crossterm::style::*;
use crossterm::ExecutableCommand;

#[derive(Debug, PartialEq, Clone)]
pub enum TestState {
    Running,
    Failed,
    WrongAnswer,
    Completed,
    Skipped,
    Queue,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TestResult {
    Better,
    Same,
    Worse,
}

#[derive(Debug, Clone)]
pub struct TestInfo {
    pub test_name: String,
    pub state: TestState,
    pub prev_score: Option<f64>,
    pub new_score: Option<f64>,
    pub time: String,
    pub cpu_time: f64,
    pub runs: usize,
    pub running: usize,
    pub total_runs: usize,
    pub result: TestResult,
}

impl TestInfo {
    pub fn new(test_name: String, total_runs: usize) -> Self {
        TestInfo {
            test_name,
            state: TestState::Queue,
            prev_score: None,
            new_score: None,
            time: String::new(),
            cpu_time: 0.0,
            runs: 0,
            running: 0,
            total_runs,
            result: TestResult::Same,
        }
    }

    pub fn print(&self, config: &Config, stdout: &mut Stdout) {
        let precision = config.precision.unwrap();
        write!(stdout, "\r").unwrap();
        write!(stdout, "| ").unwrap();
        write!(stdout, "{}", self.test_name).unwrap();
        write!(stdout, " | ").unwrap();
        write!(
            stdout,
            "{: >12}",
            if self.runs == self.total_runs {
                format!("{:.3}", self.cpu_time)
            } else {
                self.time.clone()
            }
        )
        .unwrap();
        write!(stdout, " | ").unwrap();
        match self.prev_score {
            Some(score) => write!(stdout, "{: >12.prec$}", score, prec = precision).unwrap(),
            None => write!(stdout, "{: >12}", "").unwrap(),
        };
        write!(stdout, " | ").unwrap();
        match self.new_score {
            Some(score) => write!(stdout, "{: >12.prec$}", score, prec = precision).unwrap(),
            None => {
                if self.state == TestState::Skipped {
                    write!(stdout, "{:->12}", "").unwrap();
                } else if self.state == TestState::Failed || self.state == TestState::WrongAnswer {
                    stdout.execute(SetForegroundColor(Color::Red)).unwrap();
                    write!(
                        stdout,
                        "{: >12}",
                        if self.state == TestState::Failed { "error" } else { "WA" }
                    )
                    .unwrap();
                } else {
                    write!(stdout, "{: >12}", "").unwrap();
                }
            }
        };
        stdout.execute(SetForegroundColor(Color::Reset)).unwrap();
        write!(stdout, " | ").unwrap();
        let mut delta = String::new();
        if self.prev_score.is_some() && self.new_score.is_some() {
            delta = format!(
                "{:.prec$}",
                self.new_score.unwrap() - self.prev_score.unwrap(),
                prec = precision
            );
            if delta.as_bytes()[0] != b'-' && self.result != TestResult::Same {
                delta = "+".to_string() + &delta;
            }
        }
        stdout
            .execute(match self.result {
                TestResult::Better => SetForegroundColor(Color::Green),
                TestResult::Worse => SetForegroundColor(Color::Red),
                _ => SetForegroundColor(Color::Reset),
            })
            .unwrap();
        write!(stdout, "{: >12}", delta).unwrap();
        stdout.execute(SetForegroundColor(Color::Reset)).unwrap();
        write!(stdout, " | ").unwrap();
        write!(
            stdout,
            "{: >12}",
            if self.state == TestState::Skipped {
                String::new()
            } else {
                let running = if self.running > 0 {
                    format!(" ({})", self.running)
                } else {
                    String::new()
                };
                format!("{}/{}{}", self.runs, self.total_runs, running)
            }
        )
        .unwrap();
        write!(stdout, " |").unwrap();
    }

    pub fn print_to_notion(&self, config: &Config, test_log: &TestLog) -> Vec<NotionTextChunk> {
        let precision = config.precision.unwrap();

        let mut result = Vec::new();
        result.push(NotionTextChunk::new(
            &format!("| {} | {: >12} | ", self.test_name, self.time),
            "default",
        ));

        match self.prev_score {
            Some(score) => result.push(NotionTextChunk::new(
                &format!("{: >12.prec$}", score, prec = precision),
                "default",
            )),
            None => result.push(NotionTextChunk::new(&format!("{: >12}", ""), "default")),
        };
        result.push(NotionTextChunk::new(" | ", "default"));
        match self.new_score {
            Some(score) => result.push(NotionTextChunk::new(
                &format!("{: >12.prec$}", score, prec = precision),
                "default",
            )),
            None => {
                if self.state == TestState::Skipped {
                    result.push(NotionTextChunk::new(&format!("{:->12}", ""), "default"));
                } else if self.state == TestState::Failed || self.state == TestState::WrongAnswer {
                    result.push(NotionTextChunk::new(&format!("{: >12}", "error"), "red"));
                } else {
                    result.push(NotionTextChunk::new(&format!("{: >12}", ""), "default"));
                }
            }
        };
        result.push(NotionTextChunk::new(" | ", "default"));
        let mut delta = String::new();
        if self.prev_score.is_some() && self.new_score.is_some() {
            delta = format!(
                "{:.prec$}",
                self.new_score.unwrap() - self.prev_score.unwrap(),
                prec = precision
            );
            if delta.as_bytes()[0] != b'-' && self.result != TestResult::Same {
                delta = "+".to_string() + &delta;
            }
        }
        result.push(NotionTextChunk::new(
            &format!("{: >12}", delta),
            match self.result {
                TestResult::Better => "green",
                TestResult::Worse => "red",
                TestResult::Same => "default",
            },
        ));

        result.push(NotionTextChunk::new(" | ", "default"));
        if test_log.content.is_some() {
            result.push(NotionTextChunk {
                text: format!("log-{}", test_log.last_update),
                color: "default".to_string(),
                link: Some(format!(
                    "/{}",
                    test_log.page_id.chars().filter(|c| *c != '-').collect::<String>()
                )),
            });
        } else {
            result.push(NotionTextChunk::new(&format!("{: >12}", ""), "default"));
        }

        result.push(NotionTextChunk::new(" |\n", "default"));

        result
    }
}
