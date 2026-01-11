use std::collections::HashMap;
use std::fs;
use std::io::{stdout, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use clap::Parser;
use subprocess::{Popen, PopenConfig, Redirection};

use crossterm::cursor;

use threadpool::ThreadPool;

use crate::util::*;

mod client_wrapper;
mod data;
mod mtime;
mod notion;
mod remote;
mod test_info;
mod test_log;

use data::*;
use test_info::*;

#[derive(Parser)]
pub struct ApproxArgs {
    /// Apply changes from *.out files without running solution
    #[arg(long)]
    norun: bool,

    /// Number of global iterations
    #[arg(short = 'n', long, default_value_t = 1)]
    iters: usize,

    /// Run main solution on remote host
    #[arg(long)]
    remote: bool,
}

pub fn approx(args: ApproxArgs, _params: &HashMap<String, String>) {
    let config = read_config();

    let tests_info: Arc<Mutex<Vec<TestInfo>>> = Arc::new(Mutex::new(Vec::new()));

    let total_info = Arc::new(Mutex::new(TestSuiteInfo {
        score: 0.,
        delta: 0.,
        cpu_time: 0,
        finished: false,
    }));

    let mut handle: Option<thread::JoinHandle<_>> = None;

    if config.notion.is_some() && !args.norun {
        let tests_info = tests_info.clone();
        let total_info = total_info.clone();
        let config = config.clone();
        handle = Some(thread::spawn(move || {
            notion::start_updates(config, tests_info, total_info);
        }));
    }

    let mut client = None;
    if args.remote {
        if config.remote.is_none() {
            eprintln!("Set \"remote\" config in config.json to use --remote");
            std::process::exit(1);
        }
        let client_str = remote::Client::new(config.remote.as_ref().unwrap());
        client_str.init();
        client = Some(Arc::new(client_str));
    }

    // let mut stdout = stdout().into_raw_mode().unwrap();
    let mut stdout = stdout();
    // stdout.suspend_raw_mode().unwrap();

    let title = format!(
        "| {: ^3} | {: ^12} | {: ^12} | {: ^12} | {: ^12} | {: ^12} |",
        "", "time", "prev", "new", "delta", "done"
    );
    write!(stdout, "{}\n", title).unwrap();
    let splitter: String = title.chars().map(|c| if c == '|' { '|' } else { '-' }).collect();
    write!(stdout, "{}", splitter).unwrap();

    for _ in 0..config.tests {
        write!(stdout, "\n").unwrap();
    }

    let threads = if args.remote {
        config.remote.as_ref().unwrap().threads
    } else {
        config.threads.unwrap()
    };
    let pool = ThreadPool::new(threads);

    let stdout = Arc::new(Mutex::new(stdout));

    let mut tasks = vec![vec![]; args.iters];

    for test in 1..config.tests + 1 {
        let index: usize = tests_info.lock().unwrap().len();
        let tests = config.tests;

        let config = config.clone();

        let skip = if !config.run_tests.is_empty() {
            !config.run_tests.binary_search(&test).is_ok()
        } else {
            config.skip_tests.binary_search(&test).is_ok()
        };
        let test_name = format!("{:0>3}", test);
        let mut test_info = TestInfo::new(test_name.clone(), args.iters);
        tests_info.lock().unwrap().push(test_info.clone());

        let tests_info = tests_info.clone();
        let total_info = total_info.clone();
        let stdout = stdout.clone();

        {
            let mut stdout = stdout.lock().unwrap();
            write!(stdout, "{}", cursor::MoveUp((tests - index) as u16)).unwrap();
            test_info.print(&config, &mut stdout);
            write!(stdout, "{}", cursor::MoveDown((tests - index) as u16)).unwrap();
            stdout.flush().unwrap();
        }

        let update_tests_info = |test_info: &TestInfo| {
            {
                let mut stdout = stdout.lock().unwrap();
                write!(stdout, "{}", cursor::MoveUp((tests - index) as u16)).unwrap();
                test_info.print(&config, &mut stdout);
                write!(stdout, "{}", cursor::MoveDown((tests - index) as u16)).unwrap();
                stdout.flush().unwrap();
            }
            tests_info.lock().unwrap()[index] = test_info.clone();
        };

        if skip {
            test_info.state = TestState::Skipped;
        }

        update_tests_info(&test_info);

        // calculate score from .ans
        if Path::new(&format!("tests/{}.ans", test_name)).exists() {
            let mut filename_vec = config.scorer.as_ref().unwrap().clone();
            filename_vec.push(format!("tests/{}.in", test_name));
            filename_vec.push(format!("tests/{}.ans", test_name));

            let mut p = match Popen::create(
                &filename_vec[..],
                PopenConfig {
                    stdout: Redirection::File(fs::File::create(format!("tests/{}.tmp", test_name)).unwrap()),
                    stderr: Redirection::File(fs::File::create(format!("tests/{}.err", test_name)).unwrap()),
                    ..Default::default()
                },
            ) {
                Ok(x) => x,
                Err(_) => {
                    eprintln!("Error when starting process {:?}", filename_vec);
                    std::process::exit(1)
                }
            };

            p.wait().unwrap();
            let exit_status = p.poll().unwrap();
            if !exit_status.success() {
                eprintln!("Scorer failed on {}.ans", test_name);
                std::process::exit(1);
            }
            let out = fs::read_to_string(format!("tests/{}.tmp", test_name)).unwrap();
            test_info.prev_score = Some(out.trim().parse().expect("Can't parse score"));
            update_tests_info(&test_info);
            total_info.lock().unwrap().score += test_info.prev_score.unwrap();
        }

        if skip {
            test_info.time = format!("{:->12}", "");
            update_tests_info(&test_info);
            continue;
        }

        test_info.time = mtime::get_time(config.time_offset.unwrap());
        test_info.state = TestState::Running;

        let test_info = Arc::new(Mutex::new(test_info));
        for run in 1..=args.iters {
            let ans_test_name = test_name.clone();
            let test_name = format!("{}-{:0>3}", test_name, run);
            let stdout = stdout.clone();
            let config = config.clone();
            let tests_info = tests_info.clone();
            let test_info = test_info.clone();
            let total_info = total_info.clone();
            let client = client.clone();
            tasks[run - 1].push(move || {
                let update_tests_info = |test_info: &TestInfo| {
                    {
                        let mut stdout = stdout.lock().unwrap();
                        write!(stdout, "{}", cursor::MoveUp((tests - index) as u16)).unwrap();
                        test_info.print(&config, &mut stdout);
                        write!(stdout, "{}", cursor::MoveDown((tests - index) as u16)).unwrap();
                        stdout.flush().unwrap();
                    }
                    tests_info.lock().unwrap()[index] = test_info.clone();
                };

                test_info.lock().unwrap().running += 1;

                // run solution
                update_tests_info(&*test_info.lock().unwrap());
                if !args.norun {
                    let mut filename_vec = config.main.as_ref().unwrap().clone();
                    filename_vec.push(format!("tests/{}.in", ans_test_name));
                    filename_vec.push(format!("tests/{}.out", test_name));

                    let now = Instant::now();
                    let success = if let Some(client) = client.as_ref() {
                        client.run(filename_vec)
                    } else {
                        let mut p = match Popen::create(
                            &filename_vec[..],
                            PopenConfig {
                                stderr: Redirection::File(
                                    fs::File::create(format!("tests/{}.err", test_name)).unwrap(),
                                ),
                                ..Default::default()
                            },
                        ) {
                            Ok(x) => x,
                            Err(_) => {
                                eprintln!("Error when starting process {:?}", filename_vec);
                                std::process::exit(1)
                            }
                        };

                        p.wait().unwrap();
                        p.poll().unwrap().success()
                    };

                    let time = now.elapsed().as_millis();
                    let mut test_info = test_info.lock().unwrap();
                    test_info.cpu_time += time as f64 / 1000.;
                    total_info.lock().unwrap().cpu_time += time;
                    update_tests_info(&test_info);

                    if !success {
                        test_info.state = TestState::Failed;
                        test_info.running -= 1;
                        test_info.runs += 1;
                        update_tests_info(&test_info);
                        return;
                    }
                } else {
                    let mut test_info = test_info.lock().unwrap();
                    test_info.time = format!("{:->12}", "");
                    update_tests_info(&test_info);
                }

                // calculate score from .out
                {
                    let mut filename_vec = config.scorer.as_ref().unwrap().clone();
                    filename_vec.push(format!("tests/{}.in", ans_test_name));
                    filename_vec.push(format!("tests/{}.out", test_name));

                    let success = match (client.as_ref(), config.remote.as_ref().map(|c| c.score)) {
                        (Some(client), Some(true)) => {
                            let success = client.run(vec![
                                "bash".to_string(),
                                "-c".to_string(),
                                filename_vec.join(" ") + &format!(" >tests/{}.tmp", test_name),
                            ]);
                            client.get_file(format!("tests/{}.tmp", test_name));
                            success
                        }
                        (client, _) => {
                            if let Some(client) = client {
                                client.get_file(format!("tests/{}.out", test_name));
                            }
                            let mut p = match Popen::create(
                                &filename_vec[..],
                                PopenConfig {
                                    stdout: Redirection::File(
                                        fs::File::create(format!("tests/{}.tmp", test_name)).unwrap(),
                                    ),
                                    stderr: Redirection::File(
                                        fs::File::create(format!("tests/{}.err", test_name)).unwrap(),
                                    ),
                                    ..Default::default()
                                },
                            ) {
                                Ok(x) => x,
                                Err(_) => {
                                    eprintln!("Error when starting process {:?}", filename_vec);
                                    std::process::exit(1)
                                }
                            };

                            p.wait().unwrap();
                            p.poll().unwrap().success()
                        }
                    };

                    let mut test_info = test_info.lock().unwrap();
                    if !success {
                        test_info.state = TestState::WrongAnswer;
                        test_info.running -= 1;
                        test_info.runs += 1;
                        update_tests_info(&test_info);
                        return;
                    }

                    let out = fs::read_to_string(format!("tests/{}.tmp", test_name)).unwrap();
                    let new_score = out.trim().parse().expect("Can't parse score");
                    let mut current_better = false;
                    let was_score = test_info.new_score.or(test_info.prev_score);
                    if test_info.new_score.is_none()
                        || (new_score > test_info.new_score.unwrap()) == (config.optimize == "max")
                    {
                        if let Some(score) = test_info.prev_score {
                            if (new_score > score) == (config.optimize == "max") {
                                current_better = true;
                            }
                        } else {
                            current_better = true;
                        }
                        test_info.new_score = Some(new_score);
                    }
                    if test_info.runs == test_info.total_runs && test_info.state == TestState::Running {
                        test_info.state = TestState::Completed;
                    }
                    update_tests_info(&test_info);
                    if let Some(prev_score) = test_info.prev_score {
                        let delta = test_info.new_score.unwrap() - prev_score;
                        if delta != 0. {
                            if (delta > 0.) == (config.optimize == "max") {
                                test_info.result = TestResult::Better;
                            } else {
                                test_info.result = TestResult::Worse;
                            }
                        }
                    }
                    update_tests_info(&test_info);

                    if current_better {
                        if let Some(client) = client.as_ref() {
                            client.get_file(format!("tests/{}.out", test_name));
                        }

                        fs::copy(
                            format!("tests/{}.out", test_name),
                            format!("tests/{}.ans", ans_test_name),
                        )
                        .unwrap();
                        let mut total_info = total_info.lock().unwrap();
                        if let Some(prev_score) = was_score {
                            total_info.score -= prev_score;
                            total_info.delta -= prev_score;
                        }
                        total_info.score += test_info.new_score.unwrap();
                        total_info.delta += test_info.new_score.unwrap();
                    }
                }
                if config.remote_outputs && run != 1 {
                    let output = format!("tests/{}.out", test_name);
                    if let Some(client) = client.as_ref() {
                        client.run(vec!["rm".to_string(), output.clone()]);
                    }
                    let _ = std::fs::remove_file(output);
                }

                {
                    let mut test_info = test_info.lock().unwrap();
                    test_info.running -= 1;
                    test_info.runs += 1;
                    update_tests_info(&test_info);
                }
            });
        }
    }

    for row in tasks.into_iter() {
        for task in row.into_iter() {
            pool.execute(task);
        }
    }

    pool.join();

    let mut stdout = stdout.lock().unwrap();
    write!(stdout, "{}", cursor::MoveUp((config.tests + 1) as u16)).unwrap();
    writeln!(stdout, "\r{}", title).unwrap();
    writeln!(stdout, "\r{}", splitter).unwrap();
    for test_info in tests_info.lock().unwrap().iter() {
        test_info.print(&config, &mut stdout);
        writeln!(stdout).unwrap();
    }

    write!(stdout, "\n").unwrap();
    if config.result_func == "avg" {
        let mut total_info = total_info.lock().unwrap();
        total_info.score /= config.tests as f64;
        total_info.delta /= config.tests as f64;
    }
    total_info.lock().unwrap().finished = true;
    let total_score = format!("{:.10}", total_info.lock().unwrap().score);
    writeln!(stdout, "Total: {}", &total_score).unwrap();

    // finalize
    {
        let mut p = match Popen::create(
            &config.finalize.as_ref().unwrap().clone(),
            PopenConfig {
                stdout: Redirection::Pipe,
                stderr: Redirection::Pipe,
                ..Default::default()
            },
        ) {
            Ok(x) => x,
            Err(_) => {
                eprintln!(
                    "Error when starting process {:?}",
                    &config.finalize.as_ref().unwrap().clone()
                );
                std::process::exit(1)
            }
        };

        let exit_status = p.wait().unwrap();
        if !exit_status.success() {
            eprintln!("finalize failed with status {:?}", exit_status);
            std::process::exit(1)
        }
    }

    if let Some(handle) = handle {
        handle.join().unwrap();
    }
}

fn read_config() -> Config {
    let mut config: Config = match serde_json::from_str(&fs::read_to_string("config.json").unwrap()) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Can't parse json from \"config.json\", {}", e);
            std::process::exit(1);
        }
    };
    if config.optimize != "min" && config.optimize != "max" {
        eprintln!("Optimize must be \"min\" or \"max\"");
        std::process::exit(1);
    }
    if config.result_func != "sum" && config.result_func != "avg" {
        eprintln!("Result function must be \"sum\" or \"avg\"");
        std::process::exit(1);
    }
    config.skip_tests.sort();
    config.run_tests.sort();
    if config.precision.is_none() {
        config.precision = Some(3);
    }

    if config.main.is_none() {
        config.main = Some(vec![fix_unix_filename("main")]);
    }
    if config.scorer.is_none() {
        config.scorer = Some(vec![fix_unix_filename("scorer")]);
    }
    if config.finalize.is_none() {
        config.finalize = Some(vec![fix_unix_filename("finalize")]);
    }

    if config.threads.is_none() {
        config.threads = Some(1);
    }

    if config.time_offset.is_none() {
        config.time_offset = Some(60 * 60 * 3);
    }

    config
}
