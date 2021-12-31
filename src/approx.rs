use std::collections::HashMap;
use std::fs;
use std::time::Instant;
use std::path::Path;
use std::io::{Write, stdout};
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Local;

use subprocess::{Popen, PopenConfig, Redirection};

use termion::raw::IntoRawMode;

use indoc::indoc;

use threadpool::ThreadPool;

use crate::util::*;

mod notion;
mod data;
mod test_info;
mod test_log;
mod client_wrapper;

use data::*;
use test_info::*;

pub fn approx(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr approx [flags]

            Calls \"main [num]\" for each test, then with \"scorer [file_in] [file_ans]\"
            compares current output from *.out and best answer from *.ans and leaves the
            best one. In the end calls \"finalize\".

            Flags:
                --help              Display this message
                --norun             Apply changes from *.out files without running solution
        "};
        print!("{}", s);
        return;
    }

    let mut norun = false;
    for arg in args {
        if arg == "--norun" {
            norun = true;
        }
    }


    let config = read_config();

    let tests_info: Arc<Mutex<Vec<TestInfo>>> = Arc::new(Mutex::new(Vec::new()));
    let total_score_string: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let mut handle: Option<thread::JoinHandle<_>> = None;

    if config.notion.is_some() {
        let tests_info = tests_info.clone();
        let total_score = total_score_string.clone();
        let config = config.clone();
        handle = Some(thread::spawn(move || {
            notion::start_updates(config, tests_info, total_score);
        }));
    }

    let mut stdout = stdout().into_raw_mode().unwrap();
    stdout.suspend_raw_mode().unwrap();

    let total_score = Arc::new(Mutex::new(0.));

    let title = format!("| {: ^3} | {: ^12} | {: ^12} | {: ^12} | {: ^12} |", "", "time", "prev", "new", "delta");
    write!(stdout, "{}\n", title).unwrap();
    let splitter: String = title.chars().map(|c| if c == '|' { '|' } else { '-' }).collect();
    write!(stdout, "{}", splitter).unwrap();

    for _ in 0..config.tests {
        write!(stdout, "\n").unwrap();
    }

    let pool = ThreadPool::new(config.threads.unwrap());

    let stdout = Arc::new(Mutex::new(stdout));

    for test in 1..config.tests+1 {
        let index: usize = tests_info.lock().unwrap().len();
        let tests = config.tests;

        let config = config.clone();

        let skip = config.skip_tests.as_ref().unwrap().binary_search(&test).is_ok();
        let test_name = format!("{:0>3}", test);
        let mut test_info = TestInfo::new(test_name.clone());
        tests_info.lock().unwrap().push(test_info.clone());

        let tests_info = tests_info.clone();
        let total_score = total_score.clone();
        let norun = norun.clone();
        let stdout = stdout.clone();

        {
            let mut stdout = stdout.lock().unwrap();
            write!(stdout, "{}", termion::cursor::Up((tests - index) as u16)).unwrap();
            test_info.print(&config, &mut stdout);
            write!(stdout, "{}", termion::cursor::Down((tests - index) as u16)).unwrap();
            stdout.flush().unwrap();
        }

        pool.execute(move || {
            let update_tests_info = |test_info: &TestInfo| {
                {
                    let mut stdout = stdout.lock().unwrap();
                    write!(stdout, "{}", termion::cursor::Up((tests - index) as u16)).unwrap();
                    test_info.print(&config, &mut stdout);
                    write!(stdout, "{}", termion::cursor::Down((tests - index) as u16)).unwrap();
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
                filename_vec.push(format!("tests/{}.in",  test_name));
                filename_vec.push(format!("tests/{}.ans", test_name));

                let mut p = match Popen::create(&filename_vec[..], PopenConfig {
                    stdout: Redirection::Pipe,
                    stderr: Redirection::File(fs::File::create(format!("tests/{}.err", test_name)).unwrap()),
                    ..Default::default()
                }) {
                    Ok(x) => x,
                    Err(_) => {
                        eprintln!("Error when starting process {:?}", filename_vec);
                        std::process::exit(1)
                    }
                };

                let (out, _) = p.communicate(None).unwrap();
                p.wait().unwrap();
                let exit_status = p.poll().unwrap();
                if !exit_status.success() {
                    eprintln!("Scorer failed on {}.ans", test_name);
                    std::process::exit(1);
                }
                test_info.prev_score = Some(out.unwrap().trim().parse().expect("Can't parse score"));
                update_tests_info(&test_info);
                *total_score.lock().unwrap() += test_info.prev_score.unwrap();
            }

            if skip {
                test_info.time = format!("{:->12}", "");
                update_tests_info(&test_info);
                return;
            }

            // run solution
            test_info.time = Local::now().format("%H:%M:%S").to_string();
            test_info.state = TestState::Running;
            update_tests_info(&test_info);
            if !norun {
                let mut filename_vec = config.main.as_ref().unwrap().clone();
                filename_vec.push(test_name.clone());

                let now = Instant::now();
                let mut p = match Popen::create(&filename_vec[..], PopenConfig {
                    stdin: Redirection::Pipe,
                    stdout: Redirection::Pipe,
                    stderr: Redirection::File(fs::File::create(format!("tests/{}.err", test_name)).unwrap()),
                    ..Default::default()
                }) {
                    Ok(x) => x,
                    Err(_) => {
                        eprintln!("Error when starting process {:?}", filename_vec);
                        std::process::exit(1)
                    }
                };

                p.wait().unwrap();
                let exit_status = p.poll().unwrap();

                test_info.time = format!("{:.3}", now.elapsed().as_millis() as f64 / 1000.);
                update_tests_info(&test_info);

                if !exit_status.success() {
                    test_info.state = TestState::Failed;
                    update_tests_info(&test_info);
                    return;
                }
            } else {
                test_info.time = format!("{:->12}", "");
                update_tests_info(&test_info);
            }

            // calculate score from .out
            {
                let mut filename_vec = config.scorer.as_ref().unwrap().clone();
                filename_vec.push(format!("tests/{}.in",  test_name));
                filename_vec.push(format!("tests/{}.out", test_name));

                let mut p = match Popen::create(&filename_vec[..], PopenConfig {
                    stdout: Redirection::Pipe,
                    stderr: Redirection::File(fs::File::create(format!("tests/{}.err", test_name)).unwrap()),
                    ..Default::default()
                }) {
                    Ok(x) => x,
                    Err(_) => {
                        eprintln!("Error when starting process {:?}", filename_vec);
                        std::process::exit(1)
                    }
                };

                let (out, _) = p.communicate(None).unwrap();
                p.wait().unwrap();
                let exit_status = p.poll().unwrap();
                if !exit_status.success() {
                    test_info.state = TestState::WrongAnswer;
                    update_tests_info(&test_info);
                    return;
                }
                test_info.new_score = Some(out.unwrap().trim().parse().expect("Can't parse score"));
                test_info.state = TestState::Completed;
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
            }

            if test_info.result == TestResult::Better || test_info.prev_score.is_none() {
                fs::copy(format!("tests/{}.out", test_name), format!("tests/{}.ans", test_name)).unwrap();
                if let Some(prev_score) = test_info.prev_score {
                    *total_score.lock().unwrap() -= prev_score;
                }
                *total_score.lock().unwrap() += test_info.new_score.unwrap();
            }
        });
    }

    pool.join();
    let mut stdout = stdout.lock().unwrap();
    let mut total_score: f64 = total_score.lock().unwrap().clone();

    write!(stdout, "\n").unwrap();
    if config.result_func == "avg" {
        total_score /= config.tests as f64;
    }
    let total_score = format!("{:.prec$}", total_score, prec = config.precision.unwrap().max(10));
    writeln!(stdout, "Total: {}", &total_score).unwrap();
    *total_score_string.lock().unwrap() = Some(total_score);

    // finalize
    {
        let mut p = match Popen::create(&config.finalize.as_ref().unwrap().clone(), PopenConfig {
            stdout: Redirection::Pipe,
            stderr: Redirection::Pipe,
            ..Default::default()
        }) {
            Ok(x) => x,
            Err(_) => {
                eprintln!("Error when starting process {:?}", &config.finalize.as_ref().unwrap().clone());
                std::process::exit(1)
            }
        };

        p.wait().unwrap();
    }

    if let Some(handle) = handle {
        handle.join().unwrap();
    }
}

fn read_config() -> Config {
    let mut config: Config = match serde_json::from_str(&fs::read_to_string("config.json").unwrap()) {
        Ok(x) => x,
        Err(_) => {
            eprintln!("Can't parse json from \"config.json\"");
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
    if let Some(ref mut v) = config.skip_tests {
        v.sort();
    } else {
        config.skip_tests = Some(Vec::new());
    }
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

    config
}
