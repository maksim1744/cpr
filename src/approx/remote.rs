use std::{collections::HashMap, os::unix::fs::MetadataExt, time::Duration};

use base64ct::Encoding as _;
use md5::{Digest as _, Md5};
use serde::Serialize;

use super::data::RemoteConfig;

#[derive(Serialize)]
struct RunRequest {
    workdir: String,
    cmd: Vec<String>,
}

#[derive(Serialize)]
struct OfferFilesRequest {
    workdir: String,
    hashes: HashMap<String, String>,
}

#[derive(Serialize)]
struct FileInfo {
    data: String,
    #[serde(default)]
    executable: bool,
}

#[derive(Serialize)]
struct SendFilesRequest {
    workdir: String,
    files: HashMap<String, FileInfo>,
}

#[derive(Serialize)]
struct GetFileRequest {
    workdir: String,
    path: String,
}

pub struct Client {
    url: String,
    workdir: String,
    copy: Vec<String>,
    run: Vec<Vec<String>>,
    client: reqwest::blocking::Client,
}

impl Client {
    pub fn new(config: &RemoteConfig) -> Self {
        Self {
            url: format!("http://localhost:{}", config.local_port),
            workdir: config.workdir.clone(),
            copy: config.copy.clone(),
            run: config.prerun.clone(),
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn init(&self) {
        self.wait_ping();
        self.copy_files();
        self.prerun();
    }

    pub fn run(&self, cmd: Vec<String>) -> bool {
        let id = self
            .post(
                "/run",
                &RunRequest {
                    cmd,
                    workdir: self.workdir.clone(),
                },
                32,
            )
            .unwrap();
        self.get(&format!("/wait-run/{id}"), 128)
            .map(|status| status == "ok")
            .unwrap_or(false)
    }

    pub fn get_file(&self, path: String) {
        let data = self
            .post(
                "/get-file",
                &GetFileRequest {
                    path: path.clone(),
                    workdir: self.workdir.clone(),
                },
                128,
            )
            .unwrap();
        let data = base64ct::Base64::decode_vec(&data).unwrap();
        std::fs::write(&path, data).unwrap();
    }

    fn wait_ping(&self) {
        let mut attempts = 0;
        while self.get("/ping", 1).is_err() {
            attempts += 1;
            eprint!("\rCan't reach server {} in {} attempts", self.url, attempts);
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    fn copy_files(&self) {
        eprintln!("Copying files...");
        let mut hashes = HashMap::new();
        for pattern in self.copy.iter() {
            for entry in glob::glob(&pattern).unwrap() {
                let path = entry.unwrap();
                let bytes = std::fs::read(&path).unwrap();
                let hash = base16ct::lower::encode_string(&Md5::digest(bytes));
                hashes.insert(path.to_str().unwrap().to_string(), hash);
            }
        }
        let need: Vec<String> = serde_json::from_str(
            &self
                .post(
                    "/offer-files",
                    &OfferFilesRequest {
                        hashes,
                        workdir: self.workdir.clone(),
                    },
                    1,
                )
                .unwrap(),
        )
        .unwrap();

        let mut files = HashMap::new();
        for file in need.into_iter() {
            let bytes = std::fs::read(&file).unwrap();
            let executable = std::fs::metadata(&file).unwrap().mode() & 0o111 != 0;
            files.insert(
                file,
                FileInfo {
                    data: base64ct::Base64::encode_string(&bytes),
                    executable,
                },
            );
        }
        self.post(
            "/send-files",
            &SendFilesRequest {
                files,
                workdir: self.workdir.clone(),
            },
            1,
        )
        .unwrap();
    }

    fn prerun(&self) {
        eprintln!("Executing preruns...");
        for cmd in self.run.iter() {
            self.run(cmd.clone());
        }
    }

    fn get(&self, path: &str, tries: u64) -> anyhow::Result<String> {
        for iter in 1..=tries {
            let res = self
                .client
                .get(self.url.clone() + path)
                .send()
                .and_then(|r| r.error_for_status())
                .map(|r| r.text().unwrap())
                .map_err(|e| e.into());
            if res.is_ok() || iter == tries {
                return res;
            }
            std::thread::sleep(Duration::from_secs(iter));
        }
        unreachable!()
    }

    fn post<J>(&self, path: &str, json: &J, tries: u64) -> anyhow::Result<String>
    where
        J: Serialize,
    {
        for iter in 1..=tries {
            let res = self
                .client
                .post(self.url.clone() + path)
                .json(json)
                .send()
                .and_then(|r| r.error_for_status())
                .map(|r| r.text().unwrap())
                .map_err(|e| e.into());
            if res.is_ok() || iter == tries {
                return res;
            }
            std::thread::sleep(Duration::from_secs(iter));
        }
        unreachable!()
    }
}
