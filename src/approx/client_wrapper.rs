use std::fs::File;
use std::io::Write;
use std::{thread, time};

use reqwest::blocking::{Client, Request};

use serde_json::Value;

pub struct ClientWrapper {
    client: Client
}

impl ClientWrapper {
    pub fn new(client: Client) -> Self {
        ClientWrapper {
            client,
        }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn execute(&self, request: Request, file: &mut File) -> Option<Value> {
        let num_tries = 10;
        for _ in 0..num_tries {
            let request = request.try_clone().unwrap();
            let response = self.client.execute(request);
            if let Ok(response) = response {
                if response.status().as_u16() != 429 {
                    if response.status().is_success() {
                        let response = response.text().unwrap();
                        let data: Value = match serde_json::from_str(&response) {
                            Ok(x) => x,
                            Err(e) => {
                                file.write(format!("Can't parse json from response, {}\n", e).as_bytes()).unwrap();
                                return None;
                            }
                        };
                        return Some(data);
                    } else {
                        file.write(format!("Return code {}, {}\n",
                            response.status().as_u16(),
                            response.text().unwrap()).as_bytes()).unwrap();
                        return None;
                    }
                } else {
                    file.write(format!("Hit ratelimit\n").as_bytes()).unwrap();
                }
            } else {
                file.write(format!("Can't send request, {}\n", response.unwrap_err()).as_bytes()).unwrap();
            }
            thread::sleep(time::Duration::from_millis(1000));
        }
        file.write(format!("All retries unsuccessful\n").as_bytes()).unwrap();
        None
    }
}
