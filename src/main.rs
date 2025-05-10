use approx::ApproxArgs;
use clap::Parser;
use draw::DrawArgs;
use serde::{Deserialize, Serialize};
use subprocess::{ExitStatus, Popen, PopenConfig, Redirection};

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, BufReader, Write};
use std::path::Path;
use std::time::Instant;

use soup::prelude::*;

use chrono::Local;

use serde_json::map::Map;
use serde_json::Value;

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use std::io::prelude::*;
use std::net::TcpListener;

use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use threadpool::ThreadPool;

mod approx;
mod draw;
mod util;

use crate::util::*;

const LOCAL_PARAMS_NAME: &str = "params";

#[cfg(not(target_os = "windows"))]
const PRECOMPILED_PATH: &str = "/home/maksim/tools/precompiled/O2";
#[cfg(not(target_os = "windows"))]
const SETTINGS_FILE: &str = "/home/maksim/tools/settings/settings.json";
const RUST_LIBS_PATH: &str = "/mnt/c/RLibs";

#[cfg(target_os = "windows")]
const PRECOMPILED_PATH: &str = "C:/MyPath/precompiled/O2";
#[cfg(target_os = "windows")]
const SETTINGS_FILE: &str = "C:/Users/magor/AppData/Local/cp_rust/settings.json";

const DEFAULT_OPEN_FILE_CMD: &str = "subl.exe [file]:[line]:[char]";
const DEFAULT_FILE_NAME: &str = "main";
const DEFAULT_FILE_EXTENSION: &str = "cpp";
const OPEN_FILE_ON_CREATION: bool = true;
const DEFAULT_TIMEOUT: f64 = 5.;

enum ProblemSource {
    None,
    Codeforces(String, String),
    CodeChef(String, String),
    AtCoder(String, String),
    CodinGamePuzzle(String),
    Cses(String),
}

#[derive(Default, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    open_file_cmd: Option<String>,
    #[serde(default, rename = ".vscode")]
    vscode: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct LoginInfo {
    login: String,
    password: String,
}

#[derive(Default, Serialize, Deserialize)]
struct Settings {
    #[serde(default)]
    profile: String,
    #[serde(default)]
    config: HashMap<String, Config>,
    #[serde(default)]
    auth: HashMap<String, LoginInfo>,
}

impl Settings {
    fn config(&self) -> &Config {
        &self.config[&self.profile]
    }

    fn config_mut(&mut self) -> &mut Config {
        self.config.get_mut(&self.profile).unwrap()
    }
}

#[derive(Parser)]
struct StressTestArgs {
    /// Main executable to run
    filename: Option<String>,

    /// Don't display anything, except the index of current test
    #[arg(short, long, default_value_t)]
    quiet: bool,

    /// Random seed for the first case. After each case it will be increased by 1
    #[arg(short, long, default_value_t = 0)]
    seed: i32,

    /// Run with "check.exe" instead of "easy.exe" to check
    /// output, if different answers are possible. In that case,
    /// programs are executed in the order "gen.exe",
    /// "[filename].exe", "check.exe". "check.exe" has to
    /// read input, then output of the program and return 0 if
    /// check is successful and not 0 otherwise. Merged input
    /// and output will be written to "inout", where you can
    /// see it.
    #[arg(long, default_value_t)]
    check: bool,

    /// Command line for easy solution
    #[arg(long)]
    easy: Option<String>,

    /// Command line for gen solution
    #[arg(long)]
    gen: Option<String>,

    /// Command line for checker solution
    #[arg(long)]
    checkf: Option<String>,

    /// Epsilon for comparison
    #[arg(short, long)]
    eps: Option<f64>,

    /// Timeout in seconds
    #[arg(short, long, default_value_t = DEFAULT_TIMEOUT)]
    timeout: f64,
}

fn stress_test(args: StressTestArgs, _params: &HashMap<String, String>) {
    let mut seed: i32 = args.seed;
    let filename = args.filename.unwrap_or(String::from(DEFAULT_FILE_NAME));
    let easy_str = args.easy.unwrap_or(String::from("easy"));
    let gen_str = args.gen.unwrap_or(String::from("gen"));
    let check_str = args.checkf.unwrap_or(String::from("check"));

    let mut case = 1;

    loop {
        print!("Case #{}:  ", case);
        io::stdout().flush().unwrap();
        let result = run_and_wait(
            &[&fix_unix_filename(&gen_str), &seed.to_string()],
            "",
            "in",
            Some(args.timeout),
        );
        if !result.success() {
            println!("X  [seed = {}]", seed);
            break;
        }
        print!(".");
        io::stdout().flush().unwrap();

        if !args.check {
            let result = run_and_wait(&[&easy_str], "in", "ans", Some(args.timeout));
            if !result.success() {
                println!("X  [seed = {}]", seed);
                break;
            }
            print!(".");
            io::stdout().flush().unwrap();
        }

        let result = run_and_wait(&[&filename], "in", "out", Some(args.timeout));
        if !result.success() {
            println!("X  [seed = {}]", seed);
            break;
        }
        print!(".");
        io::stdout().flush().unwrap();

        if args.check {
            let inout = [fs::read_to_string("in").unwrap(), fs::read_to_string("out").unwrap()].concat();
            fs::File::create("inout").unwrap().write(inout.as_bytes()).unwrap();

            let result = run_and_wait(&[&check_str], "inout", "ans", Some(args.timeout));
            if !result.success() {
                println!("X  [seed = {}]", seed);

                if !args.quiet {
                    println!("========== in  ==========");
                    println!("{}", read_lines_trim("in").join("\n"));
                    println!("========== out ==========");
                    println!("{}", read_lines_trim("out").join("\n"));
                    println!("========== ans ==========");
                    println!("{}", read_lines_trim("ans").join("\n"));
                }

                break;
            }
            print!(".");
            io::stdout().flush().unwrap();
        }

        if !args.check && !compare_output("out", "ans", args.eps) {
            println!("   failed   [seed = {}]", seed);
            if !args.quiet {
                println!("========== in  ==========");
                println!("{}", read_lines_trim("in").join("\n"));
                println!("========== ans ==========");
                println!("{}", read_lines_trim("ans").join("\n"));
                println!("========== out ==========");
                println!("{}", read_lines_trim("out").join("\n"));
            }
            break;
        }

        seed += 1;
        case += 1;
        print!("\r                                    \r");
    }
    if !args.quiet {
        print!("{}", fs::read_to_string("err").unwrap());
    }
}

#[derive(Parser)]
struct IStressTestArgs {
    /// Main executable to run
    filename: Option<String>,

    /// Don't display anything, except the index of current test
    #[arg(short, long, default_value_t)]
    quiet: bool,

    /// Random seed for the first case. After each case it will be increased by 1
    #[arg(short, long, default_value_t = 0)]
    seed: i32,

    /// Run with "check.exe" instead of "easy.exe" to check
    /// output, if different answers are possible. In that case,
    /// programs are executed in the order "gen.exe",
    /// "[filename].exe", "check.exe". "check.exe" has to
    /// read input, then output of the program and return 0 if
    /// check is successful and not 0 otherwise. Merged input
    /// and output will be written to "inout", where you can
    /// see it.
    #[arg(long, default_value_t)]
    check: bool,

    /// Command line for easy solution
    #[arg(long)]
    easy: Option<String>,

    /// Command line for gen solution
    #[arg(long)]
    gen: Option<String>,

    /// Command line for checker solution
    #[arg(long)]
    checkf: Option<String>,

    /// Epsilon for comparison
    #[arg(short, long)]
    eps: Option<f64>,
}

fn stress_test_inline(args: IStressTestArgs, _params: &HashMap<String, String>) {
    let seed: i32 = args.seed;
    let filename = args.filename.unwrap_or(String::from(DEFAULT_FILE_NAME));
    let easy_str = args.easy.unwrap_or(String::from("easy"));
    let gen_str = args.gen.unwrap_or(String::from("gen"));
    let check_str = args.checkf.unwrap_or(String::from("check"));

    let mut template_file = get_templates_path();
    if args.check {
        template_file.push("stress_test_check_template.cpp");
    } else {
        template_file.push("stress_test_template.cpp");
    }
    let template_file = template_file.to_str().unwrap();
    let template = fs::read_to_string(template_file).unwrap().trim().to_string();
    let template = template.split('\n').map(|x| x.trim_end()).collect::<Vec<_>>();

    let mut result: Vec<String> = Vec::new();
    let mut headers: Vec<String> = vec![
        "#include \"bits/stdc++.h\"".to_string(),
        "using namespace std;".to_string(),
    ];
    for line in template.iter() {
        if line.starts_with("//->settings") {
            if let Some(eps) = args.eps {
                result.push(["const double eps = ".to_string(), eps.to_string(), ";".to_string()].concat());
                result.push("const bool use_eps = true;".to_string());
            } else {
                result.push("const double eps = 0;".to_string());
                result.push("const bool use_eps = false;".to_string());
            }
            if args.quiet {
                result.push("const bool quiet = true;".to_string());
            } else {
                result.push("const bool quiet = false;".to_string());
            }
            result.push(["const int start_seed = ".to_string(), seed.to_string(), ";".to_string()].concat());
        } else if line.starts_with("//->") {
            let name = &line[4..];
            let file = match name {
                "main" => filename.clone(),
                "easy" => easy_str.clone(),
                "gen" => gen_str.clone(),
                "check" => check_str.clone(),
                _ => {
                    eprintln!("wrong template file");
                    std::process::exit(1);
                }
            };

            let lines = fs::read_to_string(&[file, ".cpp".to_string()].concat())
                .unwrap()
                .trim()
                .to_string();
            let lines = lines.split('\n').map(|x| x.trim_end()).collect::<Vec<_>>();

            for line2 in lines.iter() {
                if line2.starts_with("#include") {
                    if !headers.contains(&line2.to_string()) {
                        if !line2.ends_with("/print.cpp\"") {
                            headers.push(line2.to_string().clone());
                        }
                    }
                } else {
                    result.push(line2.to_string().clone());
                }
            }
        } else {
            result.push(line.to_string().clone());
        }
    }

    headers.extend(result);
    let mut file = fs::File::create("cpr_tmp_file.cpp").unwrap();
    file.write(&headers.join("\n").as_bytes()).unwrap();

    if !compile_cpr_tmp_file().is_ok() {
        return;
    }

    print!("\r                                    ");
    print!("\rStarting...");
    io::stdout().flush().unwrap();

    let _result = Popen::create(
        &[fix_unix_filename("cpr_tmp_file")],
        PopenConfig { ..Default::default() },
    )
    .unwrap()
    .wait();
}

#[derive(Parser)]
struct TestArgs {
    /// Main executable to run
    filename: Option<String>,

    /// Don't display content of failed tests
    #[arg(short, long, default_value_t)]
    quiet: bool,

    /// Which tests to run. Argument can be a string,
    /// such as "1-5,8,9-20,7" (no spaces, no quotes)
    #[arg(short, long)]
    indices: Option<String>,

    /// Run checker on output instead of comparing with ans
    #[arg(long, default_value_t)]
    check: bool,

    /// Command line for checker solution
    #[arg(long)]
    checkf: Option<String>,

    /// Epsilon for comparison
    #[arg(short, long)]
    eps: Option<f64>,

    /// Timeout in seconds
    #[arg(short, long, default_value_t = DEFAULT_TIMEOUT)]
    timeout: f64,

    /// Print output and answer side by side
    #[arg(long)]
    near: bool,
}

fn run_tests(args: TestArgs, _params: &HashMap<String, String>) {
    let filename = args.filename.unwrap_or(String::from(DEFAULT_FILE_NAME));
    let check_str = args.checkf.unwrap_or(String::from("check"));

    let mut tests = get_available_tests();
    tests.sort();

    if let Some(indices) = args.indices {
        let mut mask: HashSet<i32> = HashSet::new();
        for token in indices.split(",") {
            let token: Vec<_> = token.split("-").collect();
            if token.len() == 1 {
                mask.insert(match token[0].parse() {
                    Ok(x) => x,
                    Err(_) => {
                        panic!("Wrong mask format after \"-i\"");
                    }
                });
            } else if token.len() == 2 {
                let l: i32 = match token[0].parse() {
                    Ok(x) => x,
                    Err(_) => {
                        panic!("Wrong mask format after \"-i\"");
                    }
                };
                let r: i32 = match token[1].parse() {
                    Ok(x) => x,
                    Err(_) => {
                        panic!("Wrong mask format after \"-i\"");
                    }
                };
                for i in l..r + 1 {
                    mask.insert(i);
                }
            }
        }
        tests.retain(|x| mask.contains(x));
    }

    for test in tests.iter() {
        let now = Instant::now();
        print!("Case #{:<6}", format!("{}:", test));
        io::stdout().flush().unwrap();

        let result = run_and_wait(
            &[&filename],
            &["in", &test.to_string()].concat(),
            &["out", &test.to_string()].concat(),
            Some(args.timeout),
        );
        let duration = now.elapsed().as_millis();
        print!("{:>5} ms   ", duration);

        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        if let ExitStatus::Other(0) = result {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
            writeln!(&mut stdout, "failed with TLE").unwrap();
            stdout.set_color(&ColorSpec::new()).unwrap();

            if !args.quiet {
                println!("========== in  ==========");
                println!("{}", read_lines_trim(&["in", &test.to_string()].concat()).join("\n"));
            }
        } else if !result.success() {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
            writeln!(&mut stdout, "failed with status {:?}", result).unwrap();
            stdout.set_color(&ColorSpec::new()).unwrap();

            if !args.quiet {
                println!("========== in  ==========");
                println!("{}", read_lines_trim(&["in", &test.to_string()].concat()).join("\n"));
                println!("========== out ==========");
                println!("{}", read_lines_trim(&["out", &test.to_string()].concat()).join("\n"));
                println!("========== err ==========");
                println!("{}", read_lines_trim("err").join("\n"));
            }
        } else if args.check {
            let mut in_string = fs::read_to_string(&["in", &test.to_string()].concat()).unwrap();
            if in_string.len() != 0 && in_string.as_bytes()[in_string.len() - 1] != b'\n' {
                in_string += "\n";
            }
            let out_string = fs::read_to_string(&["out", &test.to_string()].concat()).unwrap();
            let inout = [in_string, out_string].concat();
            fs::File::create(&["inout", &test.to_string()].concat())
                .unwrap()
                .write(inout.as_bytes())
                .unwrap();

            let result = run_and_wait(
                &[&check_str],
                &["inout", &test.to_string()].concat(),
                &["ans", &test.to_string()].concat(),
                Some(args.timeout),
            );
            if !result.success() {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
                writeln!(&mut stdout, "failed").unwrap();
                stdout.set_color(&ColorSpec::new()).unwrap();

                if !args.quiet {
                    println!("========== in  ==========");
                    println!("{}", read_lines_trim(&["in", &test.to_string()].concat()).join("\n"));
                    println!("========== out ==========");
                    println!("{}", read_lines_trim(&["out", &test.to_string()].concat()).join("\n"));
                    println!("========== ans ==========");
                    println!("{}", read_lines_trim(&["ans", &test.to_string()].concat()).join("\n"));
                }
            } else {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
                writeln!(&mut stdout, "OK").unwrap();
                stdout.set_color(&ColorSpec::new()).unwrap();
            }
        } else if !Path::new(&["ans", &test.to_string()].concat()).exists() {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).unwrap();
            writeln!(&mut stdout, "?").unwrap();
            stdout.set_color(&ColorSpec::new()).unwrap();
        } else if !compare_output(
            &["out", &test.to_string()].concat(),
            &["ans", &test.to_string()].concat(),
            args.eps,
        ) {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
            writeln!(&mut stdout, "failed").unwrap();
            stdout.set_color(&ColorSpec::new()).unwrap();

            if !args.quiet {
                println!("========== in  ==========");
                println!("{}", read_lines_trim(&["in", &test.to_string()].concat()).join("\n"));

                if args.near {
                    let out_lines = read_lines_trim(&["out", &test.to_string()].concat());
                    let ans_lines = read_lines_trim(&["ans", &test.to_string()].concat());
                    let mut width: usize = 0;
                    for line in out_lines.iter() {
                        width = width.max(line.len());
                    }
                    for line in ans_lines.iter() {
                        width = width.max(line.len());
                    }
                    width += 1;
                    println!("{:=^width$}", "=", width = width * 2 + 9);
                    println!("|   |{:^width$}|{:^width$}|", "out", "ans", width = width + 1);
                    println!("-----{:-^width$}-{:-^width$}-", "", "", width = width + 1);
                    for i in 0..out_lines.len().max(ans_lines.len()) {
                        print!("|");
                        if i >= out_lines.len() || i >= ans_lines.len() || out_lines[i] != ans_lines[i] {
                            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
                            write!(&mut stdout, "{:^3}", i + 1).unwrap();
                            stdout.set_color(&ColorSpec::new()).unwrap();
                        } else {
                            write!(&mut stdout, "{:^3}", i + 1).unwrap();
                        }
                        print!("| ");
                        if i < out_lines.len() {
                            print!("{:width$}", out_lines[i], width = width);
                        } else {
                            print!("{:width$}", "", width = width);
                        }
                        print!("| ");
                        if i < ans_lines.len() {
                            print!("{:width$}", ans_lines[i], width = width);
                        } else {
                            print!("{:width$}", "", width = width);
                        }
                        println!("|");
                    }
                } else if args.eps.is_some() {
                    println!("========== out ==========");
                    println!("{}", read_lines_trim(&["out", &test.to_string()].concat()).join("\n"));
                    println!("========== ans ==========");
                    println!("{}", read_lines_trim(&["ans", &test.to_string()].concat()).join("\n"));
                } else {
                    let out_lines = read_lines_trim(&["out", &test.to_string()].concat());
                    let ans_lines = read_lines_trim(&["ans", &test.to_string()].concat());
                    println!("========== out ==========");
                    for i in 0..out_lines.len() {
                        if i >= ans_lines.len() || out_lines[i] != ans_lines[i] {
                            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
                            writeln!(&mut stdout, "{}", &out_lines[i]).unwrap();
                            stdout.set_color(&ColorSpec::new()).unwrap();
                        } else {
                            writeln!(&mut stdout, "{}", &out_lines[i]).unwrap();
                        }
                    }
                    println!("========== ans ==========");
                    println!("{}", read_lines_trim(&["ans", &test.to_string()].concat()).join("\n"));
                }
            }
        } else {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap();
            writeln!(&mut stdout, "OK").unwrap();
            stdout.set_color(&ColorSpec::new()).unwrap();
        }
    }
}

#[derive(Parser)]
struct InteractArgs {
    /// Main executable to run
    filename: Option<String>,

    /// Random seed for the first case. After each case it will be increased by 1
    #[arg(short, long, default_value_t = 0)]
    seed: i32,

    /// Don't display anything, except for the index of the current test
    #[arg(short, long, default_value_t)]
    quiet: bool,

    /// Interactor filename ("interact" by default)
    #[arg(long)]
    interactf: Option<String>,

    /// Run interactor and main once, printing each line
    #[arg(long)]
    debug: bool,

    /// Number of spaces before printing "judge:"
    #[arg(long, default_value_t = 20)]
    tab_size: usize,
}

fn interact(args: InteractArgs, _params: &HashMap<String, String>) {
    let mut seed = args.seed;
    let filename = args.filename.unwrap_or(String::from(DEFAULT_FILE_NAME));
    let interact = args.interactf.unwrap_or(String::from("interact"));

    let mut filename_vec: Vec<String> = Vec::new();
    filename_vec.extend(filename.split_whitespace().map(|x| String::from(x)).collect::<Vec<_>>());

    if cfg!(unix) {
        filename_vec[0] = ["./", &filename_vec[0]].concat().to_string();
    }

    let mut interact_vec: Vec<String> = Vec::new();
    interact_vec.extend(interact.split_whitespace().map(|x| String::from(x)).collect::<Vec<_>>());

    if cfg!(unix) {
        interact_vec[0] = ["./", &interact_vec[0]].concat().to_string();
    }

    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    let mut case = 1;

    loop {
        let (mut child_shell1, mut child1_in, rx_out1, rx_err1, tx_end11, tx_end12) =
            run_interactive(&filename_vec[..].join(" "));
        let (mut child_shell2, mut child2_in, rx_out2, rx_err2, tx_end21, tx_end22) =
            run_interactive(&[&interact_vec[..], &[seed.to_string()]].concat().join(" "));

        if !args.debug {
            print!("\rCase #{}: [seed = {}] ", case, seed);
            io::stdout().flush().unwrap();
        }

        let mut end1 = false;
        let mut end2 = false;
        loop {
            if let Ok(x) = child_shell1.try_wait() {
                if !end1 && !x.is_none() {
                    if !x.unwrap().success() {
                        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
                        writeln!(&mut stdout, "main exitted with {}", x.unwrap()).unwrap();
                        stdout.set_color(&ColorSpec::new()).unwrap();
                        std::process::exit(0);
                    }
                    end1 = true;
                }
            }
            if let Ok(x) = child_shell2.try_wait() {
                if !end2 && !x.is_none() {
                    if !x.unwrap().success() {
                        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
                        writeln!(&mut stdout, "judge exitted with {}", x.unwrap()).unwrap();
                        stdout.set_color(&ColorSpec::new()).unwrap();
                        std::process::exit(0);
                    }
                    end2 = true;
                }
            }
            if end1 && end2 {
                tx_end11.send(0).unwrap();
                tx_end12.send(0).unwrap();
                tx_end21.send(0).unwrap();
                tx_end22.send(0).unwrap();
                break;
            }
            if let Ok(line) = rx_err1.try_recv() {
                if args.debug {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).unwrap();
                    writeln!(&mut stdout, "main: {}", line.trim()).unwrap();
                    stdout.set_color(&ColorSpec::new()).unwrap();
                }
            }
            if let Ok(line) = rx_out1.try_recv() {
                if args.debug {
                    writeln!(&mut stdout, "main: {}", line.trim()).unwrap();
                }
                child2_in.write(line.as_bytes()).unwrap();
            }
            if let Ok(line) = rx_err2.try_recv() {
                if args.debug {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).unwrap();
                    writeln!(&mut stdout, "{:w$}judge: {}", "", line.trim(), w = args.tab_size).unwrap();
                    stdout.set_color(&ColorSpec::new()).unwrap();
                } else if !args.quiet {
                    writeln!(&mut stdout, "{}", line.trim()).unwrap();
                }
            }
            if let Ok(line) = rx_out2.try_recv() {
                if args.debug {
                    writeln!(&mut stdout, "{:w$}judge: {}", "", line.trim(), w = args.tab_size).unwrap();
                }
                child1_in.write(line.as_bytes()).unwrap();
            }
        }

        seed += 1;
        case += 1;
        if args.debug {
            break;
        }
    }
}

#[derive(Parser, Default)]
struct ParseArgs {
    /// Ignore all settings and listen on port
    #[arg(short, long)]
    force: bool,

    /// Parse contest. You also need to specify exactly one of --nA, --na, --n1
    #[arg(long)]
    contest: bool,

    /// Number of problems, with the first problem named "A"
    #[arg(long = "nA")]
    n_upper_a: Option<usize>,

    /// Number of problems, with the first problem named "a"
    #[arg(long = "na")]
    n_lower_a: Option<usize>,

    /// Number of problems, with the first problem named "1"
    #[arg(long = "n1")]
    n_one: Option<usize>,

    /// Print full responses
    #[arg(long)]
    echo: bool,
}

fn parse(args: ParseArgs, params: &HashMap<String, String>) {
    let mut url: Option<String> = if params.contains_key("url") {
        Some(params.get("url").unwrap().clone())
    } else {
        None
    };

    let mut problem_names: Vec<String> = Vec::new();
    if args.contest {
        let mut fill_from = |first: char, count: usize| {
            for i in 0..count {
                problem_names.push(String::from((first as usize + i) as u8 as char));
            }
        };
        match (args.n_one, args.n_upper_a, args.n_lower_a) {
            (Some(x), None, None) => fill_from('1', x),
            (None, Some(x), None) => fill_from('A', x),
            (None, None, Some(x)) => fill_from('a', x),
            _ => panic!("Exactly one of --nA, --na, --n1 must be specified"),
        }
    }

    if let Some(ref url) = url {
        if url.contains("codeforces.com") {
            let response = reqwest::blocking::get(url).unwrap().text().unwrap();
            let soup = Soup::new(&response);

            let inputs: Vec<_> = soup
                .tag("div")
                .class("input")
                .find_all()
                .map(|x| x.tag("pre").find().unwrap().display())
                .collect();
            let inputs: Vec<_> = inputs
                .iter()
                .map(|x| {
                    x.replace("<br>", "")
                        .replace("</br>", "\n")
                        .replace("<pre>", "")
                        .replace("</pre>", "")
                })
                .collect();

            let answers: Vec<_> = soup
                .tag("div")
                .class("output")
                .find_all()
                .map(|x| x.tag("pre").find().unwrap().display())
                .collect();
            let answers: Vec<_> = answers
                .iter()
                .map(|x| {
                    x.replace("<br>", "")
                        .replace("</br>", "\n")
                        .replace("<pre>", "")
                        .replace("</pre>", "")
                })
                .collect();

            for i in 0..inputs.len() {
                let test = first_available_test();
                fs::File::create(&["in", &test.to_string()].concat())
                    .unwrap()
                    .write(inputs[i].as_bytes())
                    .unwrap();
                if i < answers.len() {
                    fs::File::create(&["ans", &test.to_string()].concat())
                        .unwrap()
                        .write(answers[i].as_bytes())
                        .unwrap();
                }
            }

            println!("Parsed {} tests from codeforces", inputs.len());
            return;
        }
    }

    let create_tests_from_json = |data: &Value| {
        if data["interactive"].as_bool().unwrap() {
            println!("This is an interactive problem");
            return;
        }

        let tests = data["tests"].as_array().unwrap();

        for test in tests.iter() {
            let index = first_available_test();
            let input = test["input"].as_str().unwrap();
            let answer = test["output"].as_str().unwrap();

            fs::File::create(&["in", &index.to_string()].concat())
                .unwrap()
                .write(input.as_bytes())
                .unwrap();
            if !answer.is_empty() {
                fs::File::create(&["ans", &index.to_string()].concat())
                    .unwrap()
                    .write(answer.as_bytes())
                    .unwrap();
            }
        }

        println!("Parsed {} tests", tests.len());
    };

    if args.contest {
        println!("Creating problems: {:?}", problem_names);
        url = None;
    }

    if !args.contest && !args.force {
        let preparsed_samples = fs::read_to_string("../.preparsed_samples");
        if let Ok(preparsed_samples) = preparsed_samples {
            let preparsed: Value = match serde_json::from_str(&preparsed_samples) {
                Ok(x) => x,
                Err(_) => {
                    eprintln!("Can't parse json from \"settings.json\"");
                    std::process::exit(1);
                }
            };
            let path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
            let parts: Vec<&str>;
            if path.contains("\\") {
                parts = path.split("\\").collect();
            } else {
                parts = path.split("/").collect();
            }
            let problem = parts[parts.len() - 1];

            if let Some(data) = preparsed.get(problem) {
                create_tests_from_json(data);
            } else {
                for suffix in 1.. {
                    if let Some(data) = preparsed.get(&[problem, &suffix.to_string()].concat()) {
                        create_tests_from_json(data);
                    } else {
                        break;
                    }
                }
            }
            return;
        } else {
            println!("Can't find ../.preparsed_samples");
        }
    }

    if let Some(ref url) = url {
        println!("Expecting url \"{}\"", url);
    } else {
        println!("Accepting from any url")
    }

    let listener = TcpListener::bind("127.0.0.1:10046").unwrap();

    let mut problem_iter = 0 as usize;
    let mut listener_iter = listener.incoming();
    let mut contest_data: Map<String, Value> = Map::new();

    let mut parsed_problems: HashSet<String> = HashSet::new();

    loop {
        let mut problem_name: String = String::new();
        if args.contest {
            if problem_iter == problem_names.len() {
                break;
            } else {
                problem_name = problem_names[problem_iter].clone();
                problem_iter += 1;
            }
        }

        let mut stream = listener_iter.next().unwrap().unwrap();

        let mut buffer = [0; 4096];

        stream.read(&mut buffer).unwrap();

        let response = String::from_utf8_lossy(&buffer[..]);
        if args.echo {
            println!("{:?}", response);
        }
        let json_start = response.find("\r\n\r\n");
        if json_start.is_none() {
            println!("Empty response");
            problem_iter -= 1;
            continue;
        }
        let response = response[json_start.unwrap() + 4..].to_string();
        let response = response.trim_matches(char::from(0));

        let data = serde_json::from_str::<Value>(&response);
        if !data.is_ok() {
            eprintln!("Can't read json from [{}]", response);
            problem_iter -= 1;
            continue;
        }
        let data = data.unwrap();

        let response_url = data["url"].as_str().unwrap().to_string();

        if let Some(ref url) = url {
            if &response_url != url {
                println!("Skipping url \"{}\"", response_url);
                continue;
            }
        }

        println!("Got url \"{}\"", response_url);

        if args.contest {
            if parsed_problems.contains(&response_url) {
                println!("duplicate");
                problem_iter -= 1;
            } else {
                contest_data.insert(problem_name, data);
                parsed_problems.insert(response_url);
            }
            let data = Value::Object(contest_data.clone());
            fs::File::create(".preparsed_samples")
                .unwrap()
                .write(serde_json::to_string(&data).unwrap().as_bytes())
                .unwrap();
        } else {
            create_tests_from_json(&data);
            return;
        }
    }
}

#[derive(Parser)]
struct MakeFileArgs {
    filename: Option<String>,

    /// Use "tstart" template for multitest
    #[arg(short, long)]
    test: bool,

    /// Use "gstart" template for generator
    #[arg(short, long)]
    gen: bool,

    /// Use "gcj" template for multitest with GCJ output format
    #[arg(long)]
    gcj: bool,
}

fn make_file(args: MakeFileArgs, params: &mut HashMap<String, String>) {
    let mut filename = args.filename.unwrap_or(String::from(DEFAULT_FILE_NAME));
    let mut extension = get_default_file_extension();
    if let Some((name, ext)) = filename.split_once('.') {
        extension = ext.to_string();
        filename = name.to_string();
    }

    enum TemplateType {
        Start,
        Tstart,
        Gcj,
        Gstart,
    }

    let template_type = match (args.test, args.gen, args.gcj) {
        (false, false, false) => TemplateType::Start,
        (true, false, false) => TemplateType::Tstart,
        (false, true, false) => TemplateType::Gstart,
        (false, false, true) => TemplateType::Gcj,
        _ => panic!("At most one of -t, --gen, --gcj must be specified"),
    };

    if extension == "rs" {
        init_rust_directory();
        filename = format!(
            "src/bin/{}_{}",
            filename,
            std::env::current_dir().unwrap().file_name().unwrap().to_str().unwrap()
        );
    }
    let full_name = &[&filename, ".", &extension].concat();
    if Path::new(full_name).exists() {
        print!("File already exists. Overwrite? (y/n) ");
        io::stdout().flush().unwrap();
        let mut ans = String::new();
        io::stdin().read_line(&mut ans).unwrap();
        ans = ans.trim().to_string();
        if ans == "y" || ans == "Y" {
            println!("Overwriting");
        } else {
            println!("Cancelling");
            std::process::exit(1);
        }
    }

    let mut file = fs::File::create(&full_name).unwrap();

    let template_type = match template_type {
        TemplateType::Start => "start",
        TemplateType::Tstart => "tstart",
        TemplateType::Gcj => "gcj",
        TemplateType::Gstart => "gstart",
    };

    let mut template_path = get_templates_path();
    template_path.push("start");
    template_path.push(format!("{}.txt", extension));

    let mut position = (0, 0);

    let template_base = fs::read_to_string(template_path);
    if let Ok(template_base) = template_base {
        let template_base = template_base.trim().to_string();
        let template_base = template_base.split('\n').map(|x| x.trim_end()).collect::<Vec<_>>();

        let get_filtered_line = |line: &str| -> Option<String> {
            let mut ind: usize = 0;
            while ind < line.len() {
                if !line[ind..].starts_with("[[") {
                    return Some(line[ind..].to_string());
                }
                let next_ind = line[ind..].find("]]").unwrap() + ind;
                let filter = line[ind + 2..next_ind].split(':').collect::<Vec<_>>();
                let filter_type = filter[0];
                let values = filter[1].split('|').collect::<Vec<_>>();

                let good_line = match filter_type {
                    "os" => values.contains(&env::consts::OS),
                    "type" => values.contains(&template_type),
                    _ => false,
                };
                if !good_line {
                    return None;
                }
                ind = next_ind + 2;
            }
            return Some("".to_string());
        };

        let mut template = String::new();

        for line in template_base.iter() {
            if let Some(line) = get_filtered_line(line) {
                template += &line;
                template.push('\n');
            }
        }

        let now = Local::now();
        template = template
            .replace("\\$", "$")
            .replace("${1:date}", &now.format("%d.%m.%Y %H:%M:%S").to_string())
            .to_string();

        let mut cursor_expr = "$0";
        if template.contains("${0:}") {
            cursor_expr = "${0:}";
        }

        if template.contains(cursor_expr) {
            position.0 = template[..template.find(cursor_expr).unwrap()].matches("\n").count();
            position.1 = template.split("\n").collect::<Vec<_>>()[position.0]
                .find(cursor_expr)
                .unwrap();

            position.0 += 1;
            position.1 += 1;

            template = template.replace(cursor_expr, "").to_string();
        }

        file.write(&template.as_bytes()).unwrap();
    }

    if !params.contains_key("main") {
        add_param("main", full_name, params);
    }

    if OPEN_FILE_ON_CREATION {
        let open_file_cmd = get_settings()
            .config()
            .open_file_cmd
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or_else(|| DEFAULT_OPEN_FILE_CMD)
            .replace("[file]", full_name)
            .replace("[line]", &position.0.to_string())
            .replace("[char]", &position.1.to_string());
        let parts = open_file_cmd.split_whitespace().collect::<Vec<_>>();
        let mut command = std::process::Command::new(parts[0]);
        for part in parts[1..].into_iter() {
            command.arg(part);
        }
        command.output().unwrap();
    }
}

fn init_task(args: MakeFileArgs, params: &mut HashMap<String, String>) {
    make_file(args, params);
    parse(ParseArgs::default(), params);
}

#[derive(Parser)]
struct MakeTestArgs {
    /// Explicity set index for the new test
    index: Option<i32>,

    /// Copy "in", "ans" to the new test instead of readin from stdin
    #[arg(short = '0')]
    from_zero: bool,
}

fn make_test(args: MakeTestArgs, _params: &HashMap<String, String>) {
    let index = args.index.unwrap_or(first_available_test());

    if args.from_zero {
        fs::File::create(&["in", &index.to_string()].concat())
            .unwrap()
            .write(fs::read_to_string("in").unwrap().as_bytes())
            .unwrap();
        if let Ok(answer) = fs::read_to_string("ans") {
            fs::File::create(&["ans", &index.to_string()].concat())
                .unwrap()
                .write(answer.as_bytes())
                .unwrap();
        }
        return;
    }

    let mut lines: Vec<Vec<String>> = vec![Vec::new(); 1];

    while lines.len() <= 2 {
        let mut line = String::new();
        let read_result = io::stdin().read_line(&mut line);
        if let Err(_) = read_result {
            eprintln!("Can't read line");
            return;
        }
        if let Ok(0) = read_result {
            return;
        }

        if line.trim() == "`" {
            lines.push(Vec::new());
        } else {
            lines.last_mut().unwrap().push(line);
        }
    }

    while lines.len() <= 2 {
        lines.push(Vec::new());
    }

    let input = &lines[0];
    let answer = &lines[1];

    fs::File::create(&["in", &index.to_string()].concat())
        .unwrap()
        .write(input.join("").as_bytes())
        .unwrap();
    if !answer.is_empty() {
        fs::File::create(&["ans", &index.to_string()].concat())
            .unwrap()
            .write(answer.join("").as_bytes())
            .unwrap();
    }
}

#[derive(Parser)]
struct SplitTestArgs {
    filename: String,
    input: String,
}

fn split_test(args: SplitTestArgs, _params: &HashMap<String, String>) {
    let filename = args.filename;
    let input = args.input;

    let filename = [filename, String::from("."), get_default_file_extension()].concat();

    let mut new_main: Vec<String> = Vec::new();
    let file = fs::read_to_string(filename).unwrap().trim().to_string();
    let file = file.split('\n').map(|x| x.trim_end()).collect::<Vec<_>>();

    let mut input_end_found = false;

    for &line in file.iter() {
        let mut token = String::new();
        let mut res_line = String::new();

        for c in line.chars() {
            if c.is_ascii_alphanumeric() || c == '_' {
                token.push(c);
            } else {
                if token == "cin" {
                    token = "fake_cin".to_string();
                }
                token.push(c);
                res_line += &token;
                token.clear();
            }
        }
        res_line += &token;

        new_main.push(res_line);

        if line == "using namespace std;" {
            new_main.push("stringstream fake_cin;".to_string());
        } else if line.starts_with("int main(") {
            new_main.push("    {".to_string());
            new_main.push("        ios_base::sync_with_stdio(false); cin.tie(0); cout.tie(0);".to_string());
            new_main.push("        string tmp;".to_string());
            new_main.push("        while (getline(std::cin, tmp)) {".to_string());
            new_main.push("            fake_cin << tmp << '\\n';".to_string());
            new_main.push("        }".to_string());
            new_main.push("    }".to_string());
        } else if line.contains("/* input-end */") {
            new_main.push("    cerr << fake_cin.tellg() << endl;".to_string());
            new_main.push("    return;".to_string());
            input_end_found = true;
        }
    }

    if !input_end_found {
        eprintln!("/* input-end */ not found");
        std::process::exit(1);
    }

    let mut file = fs::File::create("cpr_tmp_file.cpp").unwrap();
    file.write(&new_main.join("\n").as_bytes()).unwrap();

    if !compile_cpr_tmp_file().is_ok() {
        return;
    }

    print!("\r                                    ");
    print!("\rRunning...");
    io::stdout().flush().unwrap();

    run_and_wait(&["cpr_tmp_file"], &input, "", None);

    print!("\rCreating tests...");

    let split_positions = fs::read_to_string("err").unwrap().trim().to_string();
    let split_positions = split_positions.split('\n').map(|x| x.trim_end()).collect::<Vec<_>>();
    let mut split_positions = split_positions
        .iter()
        .map(|x| x.parse::<usize>().unwrap())
        .collect::<Vec<usize>>();
    split_positions.insert(0, 0);

    let input = fs::read_to_string(input).unwrap().trim().to_string();
    split_positions.insert(1, input.find('\n').unwrap());

    fs::create_dir_all("tests").unwrap();
    fs::remove_dir_all("tests").unwrap();
    fs::create_dir_all("tests").unwrap();

    for i in 1..split_positions.len() - 1 {
        let test = input[split_positions[i] + 1..split_positions[i + 1]].to_string();

        let mut file = fs::File::create(format!("tests/{:0>3}", i)).unwrap();
        file.write(&["1\n".to_string(), test, "\n".to_string()].concat().as_bytes())
            .unwrap();
    }

    print!("\r                                    ");
    println!("\rCreated {} tests", split_positions.len() - 2);
}

#[derive(Parser)]
struct MultirunArgs {
    /// Main executable to run
    filename: Option<String>,

    /// Number of threads
    #[arg(short, long, default_value_t = 8)]
    threads: usize,

    /// Output filename, can be used to write concatenated *.out results
    #[arg(short, long)]
    output: Option<String>,
}

fn multirun(args: MultirunArgs, _params: &HashMap<String, String>) {
    let mut filename = args.filename.unwrap_or(String::from(DEFAULT_FILE_NAME));

    filename = [filename, String::from("."), get_default_file_extension()].concat();

    let mut new_main: Vec<String> = Vec::new();
    let file = fs::read_to_string(filename).unwrap().trim().to_string();
    let file = file.split('\n').map(|x| x.trim_end()).collect::<Vec<_>>();

    for &line in file.iter() {
        if line.starts_with("int main(") {
            new_main.push("int main(int argc, char *argv[]) {".to_string());
        } else if line.contains("cout << \"Case #\"") {
            new_main.push(format!(
                "{}cout << \"Case #\" << stoi(argv[1]) << \": \";",
                String::from_utf8(vec![b' '; line.find('c').unwrap()]).unwrap()
            ));
        } else {
            new_main.push(line.to_string());
        }
    }

    let mut file = fs::File::create("cpr_tmp_file.cpp").unwrap();
    file.write(&new_main.join("\n").as_bytes()).unwrap();

    if !compile_cpr_tmp_file().is_ok() {
        return;
    }

    print!("\r                                    ");
    print!("\r");
    io::stdout().flush().unwrap();

    let mut tests = fs::read_dir("tests")
        .unwrap()
        .map(|x| x.unwrap())
        .map(|x| {
            (
                x.path().file_name().unwrap().to_str().unwrap().to_string(),
                x.metadata().unwrap().len(),
            )
        })
        .collect::<Vec<_>>();
    tests.sort_by(|a, b| b.1.cmp(&a.1));
    let tests = tests
        .into_iter()
        .map(|(x, _)| x)
        .filter(|x| !x.contains('_'))
        .collect::<Vec<_>>();

    print!(" ");
    for i in 0..tests.len() {
        print!("{}", (i + 1) % 10);
    }
    println!();

    let result_string_mutex = Arc::new(Mutex::new(Vec::<u8>::new()));
    for c in format!("[{}]", String::from_utf8(vec![b' '; tests.len()]).unwrap()).chars() {
        result_string_mutex.lock().unwrap().push(c as u8);
    }
    print!(
        "{}",
        String::from_utf8(result_string_mutex.lock().unwrap().to_vec()).unwrap()
    );
    io::stdout().flush().unwrap();

    let failed_tests = Arc::new(Mutex::new(0));

    let pool = ThreadPool::new(args.threads);

    for input in tests.iter() {
        let input = input.clone();
        let local_result_string = result_string_mutex.clone();
        let local_failed_tests = failed_tests.clone();

        pool.execute(move || {
            let test_num = input.parse::<usize>().unwrap();

            local_result_string.lock().unwrap()[test_num] = b'.';
            print!(
                "\r{}",
                String::from_utf8(local_result_string.lock().unwrap().to_vec()).unwrap()
            );
            io::stdout().flush().unwrap();

            let result = run_and_wait(
                &[&fix_unix_filename("cpr_tmp_file"), &input],
                &format!("tests/{}", input),
                &format!("tests/{}_out", input),
                None,
            );

            if result.success() {
                local_result_string.lock().unwrap()[test_num] = b'+';
            } else {
                local_result_string.lock().unwrap()[test_num] = b'X';
                *local_failed_tests.lock().unwrap() += 1;
            }
            print!(
                "\r{}",
                String::from_utf8(local_result_string.lock().unwrap().to_vec()).unwrap()
            );
            io::stdout().flush().unwrap();
        });
    }

    pool.join();
    println!("");

    if *failed_tests.lock().unwrap() != 0 {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
        writeln!(&mut stdout, "failed {} tests", failed_tests.lock().unwrap()).unwrap();
        stdout.set_color(&ColorSpec::new()).unwrap();
    }

    if let Some(output) = args.output {
        let mut tests = tests;
        tests.sort();
        let mut res = String::new();
        for input in tests.iter() {
            res += &fs::read_to_string(&format!("tests/{}_out", input)).unwrap().to_string();
        }
        let mut file = fs::File::create(output).unwrap();
        file.write(&res.as_bytes()).unwrap();
    }
}

#[derive(Parser)]
struct ConfigArgs {
    name: String,
    value: String,
}

fn config(args: ConfigArgs, _params: &HashMap<String, String>) {
    let mut settings: Settings = get_settings();

    if &args.name == "lang" {
        settings.config_mut().lang = Some(args.value);
    } else if &args.name == "open_file_cmd" {
        settings.config_mut().open_file_cmd = Some(args.value);
    } else {
        eprintln!("Unknown param_name [{}]", &args.name);
        std::process::exit(1);
    }

    std::fs::create_dir_all(Path::new(SETTINGS_FILE).parent().unwrap()).unwrap();
    fs::write(SETTINGS_FILE, serde_json::to_string_pretty(&settings).unwrap()).unwrap();
}

#[derive(Parser)]
struct ProfileArgs {
    profile: String,
}

fn profile(args: ProfileArgs, _params: &HashMap<String, String>) {
    let mut settings: Settings = get_settings();
    let profile = args.profile;
    if !settings.config.contains_key(&profile) {
        panic!(
            "No such profile {}, available: {:?}",
            profile,
            settings.config.keys().collect::<Vec<_>>()
        );
    }

    settings.profile = profile;

    std::fs::create_dir_all(Path::new(SETTINGS_FILE).parent().unwrap()).unwrap();
    fs::write(SETTINGS_FILE, serde_json::to_string_pretty(&settings).unwrap()).unwrap();
}

#[derive(Parser)]
struct WorkspaceArgs {}

fn workspace(_args: WorkspaceArgs, _params: &HashMap<String, String>) {
    let settings: Settings = get_settings();
    let config = settings.config();
    let lang = config.lang.as_ref().unwrap();
    if lang == "cpp" {
        println!("Nothing to do for cpp");
        return;
    }
    if lang != "rs" {
        panic!("Don't know what to do for {}", lang);
    }
    let cargo_path = std::path::Path::new("Cargo.toml");
    if cargo_path.exists() {
        panic!("Cargo.toml already exists here");
    }

    let lines = fs::read_to_string(get_templates_path().join("Cargo_workspace.toml")).unwrap();
    let mut file = fs::File::create(cargo_path).unwrap();
    file.write_all(lines.as_bytes()).unwrap();

    for (key, value) in config.vscode.iter() {
        let _ = std::fs::create_dir_all(".vscode");
        fs::File::create(format!(".vscode/{key}"))
            .unwrap()
            .write_all(serde_json::to_string_pretty(value).unwrap().as_bytes())
            .unwrap();
    }
}

// ************************************* main *************************************

#[derive(Parser)]
enum Args {
    /// Run stress test
    ///
    /// Runs "gen.exe" to generate input, then "easy.exe" to generate answer to that
    /// input then "[filename].exe" ("main.exe" if not specified) to get output and
    /// then compares output to the answer.
    /// "gen.exe" should accept random seed in the first argument.
    /// All programs have to read and write using stdin, stdout.
    /// It uses files "in", "out", "ans" for corresponding info.
    Stress(StressTestArgs),

    /// Run stress test, but compile everything into one file
    ///
    /// Combines "main.cpp", "easy.cpp" and "gen.cpp" into one file to run
    /// stress tests. All programs have to read and write using stdin, stdout
    /// using cin and cout. Saves test into "in", "out", "ans".
    #[command(name = "istress")]
    IStress(IStressTestArgs),

    /// Run solution on the predefined set of tests.
    ///
    /// Each test consists of input in the file "in[index]" and correct output in "ans[index]"
    Test(TestArgs),

    /// Stress test for interactive problems
    Interact(InteractArgs),

    /// Parses samples using competitive companion (port 10046)
    Parse(ParseArgs),

    /// Creates file with template
    ///
    /// Default filename is "main"
    #[command(name = "mk")]
    MakeFile(MakeFileArgs),

    /// Executes "cpr mk [flags]" and "cpr parse". See "cpr mk --help" for more info
    #[command(about = "cpr mk + cpr parse")]
    Init(MakeFileArgs),

    /// Make new test
    ///
    /// Creates test from input. Split input and answer with ` (on the new line),
    /// answer can be empty. Input and answer will be written to "in[index]" and
    /// "ans[index]". If index is not specified, it will be chosen as the least
    /// number such that "in[index]" does not exist.
    #[command(name = "mktest")]
    MakeTest(MakeTestArgs),

    /// Draws
    ///
    /// Use "cpr draw [type] --help" for more info.
    Draw(DrawArgs),

    /// Split a file with multiple testcases into separate files
    ///
    /// Splits multitest from [input] into single tests and puts them in folder "tests".
    /// Needs C++ solution [filename] which has "/* input-end */" after reading input
    /// for each test.
    #[command(name = "splittest")]
    SplitTest(SplitTestArgs),

    /// Runs the solution on tests using multiple threads. First run "splittest"
    Multirun(MultirunArgs),

    /// Run solution once on every input file and update better answers overall
    ///
    /// Calls "main [num]" for each test, then with "scorer [file_in] [file_ans]"
    /// compares current output from *.out and best answer from *.ans and leaves the
    /// best one. In the end calls "finalize".
    Approx(ApproxArgs),

    /// Update config value
    Config(ConfigArgs),

    /// Change language profile
    Profile(ProfileArgs),

    /// Setup workspace
    Workspace(WorkspaceArgs),
}

fn main() {
    let args = Args::parse();
    let mut params = get_params();
    match args {
        Args::Stress(args) => stress_test(args, &params),
        Args::IStress(args) => stress_test_inline(args, &params),
        Args::Test(args) => run_tests(args, &params),
        Args::Interact(args) => interact(args, &params),
        Args::Parse(args) => parse(args, &params),
        Args::MakeFile(args) => make_file(args, &mut params),
        Args::Init(args) => init_task(args, &mut params),
        Args::MakeTest(args) => make_test(args, &params),
        Args::Draw(args) => draw::draw(args, &params),
        Args::SplitTest(args) => split_test(args, &params),
        Args::Multirun(args) => multirun(args, &params),
        Args::Approx(args) => approx::approx(args, &params),
        Args::Config(args) => config(args, &params),
        Args::Profile(args) => profile(args, &params),
        Args::Workspace(args) => workspace(args, &params),
    };
}

// *********************************** internal ***********************************

fn run_and_wait(filename: &[&str], fin: &str, fout: &str, timeout: Option<f64>) -> ExitStatus {
    let stdin = match fin {
        "" => Redirection::Pipe,
        name => Redirection::File(fs::File::open(name).unwrap()),
    };
    let stdout = match fout {
        "" => Redirection::Pipe,
        name => Redirection::File(fs::File::create(name).unwrap()),
    };

    let mut filename_vec: Vec<String> = Vec::new();
    for &item in filename.iter() {
        filename_vec.extend(item.split(" ").map(|x| String::from(x)).collect::<Vec<_>>());
    }

    fix_unix_filename_vec(&mut filename_vec);

    let mut p = match Popen::create(
        &filename_vec[..],
        PopenConfig {
            stdin: stdin,
            stdout: stdout,
            stderr: Redirection::File(fs::File::create("err").unwrap()),
            ..Default::default()
        },
    ) {
        Ok(x) => x,
        Err(_) => {
            eprintln!("Error when starting process {:?}", filename_vec);
            std::process::exit(1)
        }
    };

    if let Some(timeout) = timeout {
        p.wait_timeout(std::time::Duration::from_millis((timeout * 1000.0).round() as u64))
            .unwrap();
    } else {
        p.wait().unwrap();
    }

    if let None = p.poll() {
        p.terminate().unwrap();
        return ExitStatus::Other(0);
    }
    p.poll().unwrap()
}

fn read_lines_trim(filename: &str) -> Vec<String> {
    let mut res = fs::read_to_string(filename)
        .unwrap()
        .trim_end()
        .split("\n")
        .map(String::from)
        .collect::<Vec<_>>();
    for i in 0..res.len() {
        res[i] = res[i].trim_end().to_string();
    }
    res
}

fn compare_output(fout: &str, fans: &str, eps: Option<f64>) -> bool {
    let fout_lines = read_lines_trim(fout);
    let fans_lines = read_lines_trim(fans);

    if eps.is_none() {
        return fout_lines == fans_lines;
    }
    let eps = eps.unwrap();

    if fout_lines.len() != fans_lines.len() {
        return false;
    }

    for i in 0..fout_lines.len() {
        let fout_line = fout_lines[i].split_whitespace().collect::<Vec<_>>();
        let fans_line = fans_lines[i].split_whitespace().collect::<Vec<_>>();
        if fout_line.len() != fans_line.len() {
            return false;
        }
        for j in 0..fout_line.len() {
            let fout_val = fout_line[j].parse::<f64>();
            let fans_val = fans_line[j].parse::<f64>();
            if fout_val.is_ok() != fans_val.is_ok() {
                return false;
            }
            if !fout_val.is_ok() {
                if fout_line[j] != fans_line[j] {
                    return false;
                }
            } else {
                if (fout_val.unwrap() - fans_val.unwrap()).abs() > eps {
                    return false;
                }
            }
        }
    }
    true
}

fn get_available_tests() -> Vec<i32> {
    let mut v: Vec<_> = fs::read_dir(".")
        .unwrap()
        .map(|x| x.unwrap().path().file_name().unwrap().to_str().unwrap().to_string())
        .collect();
    v.retain(|x| {
        x.starts_with("in")
            && match x[2..].parse::<i32>() {
                Ok(_) => true,
                Err(_) => false,
            }
    });
    v.iter().map(|x| x[2..].parse().unwrap()).collect()
}

fn get_params() -> HashMap<String, String> {
    let mut outer_params = read_params(&["../", LOCAL_PARAMS_NAME].concat());
    let inner_params = read_params(LOCAL_PARAMS_NAME);
    if outer_params.contains_key("url") && !inner_params.contains_key("url") {
        let mut url = outer_params.get("url").unwrap().clone();
        if url.as_bytes().last().unwrap() != &b'/' {
            url.push('/');
        }
        let folder = std::env::current_dir()
            .unwrap()
            .as_path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        url += &folder;
        outer_params.insert("url".to_string(), url);
    }
    if !outer_params.contains_key("url") {
        if let Some(url) = guess_url_from_path() {
            outer_params.insert("url".to_string(), url);
        }
    }
    outer_params.extend(inner_params);
    outer_params
}

fn read_params(filename: &str) -> HashMap<String, String> {
    if !Path::new(filename).exists() {
        return HashMap::new();
    }
    let mut res = HashMap::new();
    let lines = read_lines_trim(filename);
    for item in lines.iter() {
        let idx = item.find(':').unwrap();
        res.insert(item[..idx].to_string(), item[idx + 1..].to_string());
    }
    res
}

fn write_params(filename: &str, params: &mut HashMap<String, String>) {
    let mut file = fs::File::create(filename).unwrap();
    for (key, value) in params.iter() {
        file.write(&format!("{}:{}\n", key, value).as_bytes()).unwrap();
    }
}

fn add_param(key: &str, value: &str, params: &mut HashMap<String, String>) {
    params.insert(key.to_string(), value.to_string());

    write_params("params", params);
}

fn first_available_test() -> i32 {
    let mut tests = get_available_tests();
    tests.sort();
    for i in 1.. {
        if i - 1 >= tests.len() || tests[i - 1] != i as i32 {
            return i as i32;
        }
    }
    -1
}

fn get_problem_source() -> ProblemSource {
    let path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
    let parts: Vec<&str>;
    if path.contains("\\") {
        parts = path.split("\\").collect();
    } else {
        parts = path.split("/").collect();
    }
    if path.to_lowercase().contains("codeforces") {
        let mut contest = match parts[parts.len() - 2].parse::<i32>() {
            Ok(x) => x,
            Err(_) => -1,
        };
        let mut problem = parts[parts.len() - 1].to_string();
        if contest == -1 {
            contest = 0;
            let mut still_contest = true;
            problem.clear();
            for &c in parts[parts.len() - 1].as_bytes() {
                if b'0' <= c && c <= b'9' && still_contest {
                    contest = contest * 10 + (c - b'0') as i32;
                } else {
                    still_contest = false;
                    problem.push(c as char);
                }
            }
        }
        if !problem.is_empty() && contest > 0 {
            return ProblemSource::Codeforces(contest.to_string(), problem);
        }
    } else if path.to_lowercase().contains("codechef") {
        let contest = parts[parts.len() - 2];
        let problem = parts[parts.len() - 1];
        return ProblemSource::CodeChef(contest.to_string(), problem.to_string());
    } else if path.to_lowercase().contains("atcoder") {
        let contest = parts[parts.len() - 2];
        let problem = parts[parts.len() - 1];
        return ProblemSource::AtCoder(contest.to_string(), problem.to_string());
    } else if path.to_lowercase().contains("codingame") && path.to_lowercase().contains("puzzles") {
        let problem = parts[parts.len() - 1];
        return ProblemSource::CodinGamePuzzle(problem.to_string());
    } else if path.to_lowercase().contains("cses") && path.to_lowercase().contains("problemset") {
        let problem = parts[parts.len() - 1];
        return ProblemSource::Cses(problem.to_string());
    }
    ProblemSource::None
}

fn guess_url_from_path() -> Option<String> {
    let problem_source = get_problem_source();
    if let ProblemSource::Codeforces(contest, problem) = problem_source {
        if contest.parse::<i32>().unwrap() < 100000 {
            return Some(format!(
                "https://codeforces.com/contest/{}/problem/{}",
                contest, problem
            ));
        } else {
            return Some(format!("https://codeforces.com/gym/{}/problem/{}", contest, problem));
        }
    } else if let ProblemSource::CodeChef(contest, problem) = problem_source {
        return Some(format!("https://www.codechef.com/{}/problems/{}", contest, problem));
    } else if let ProblemSource::AtCoder(contest, problem) = problem_source {
        return Some(format!(
            "https://atcoder.jp/contests/{0}/tasks/{0}_{1}",
            contest,
            problem.to_lowercase()
        ));
    } else if let ProblemSource::CodinGamePuzzle(problem) = problem_source {
        return Some(format!("https://www.codingame.com/ide/puzzle/{}", problem));
    } else if let ProblemSource::Cses(problem) = problem_source {
        return Some(format!("https://cses.fi/problemset/task/{}", problem));
    }
    None
}

fn get_default_file_extension() -> String {
    let settings = get_settings();
    settings
        .config()
        .lang
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or(DEFAULT_FILE_EXTENSION)
        .to_string()
}

fn run_interactive(
    name: &str,
) -> (
    std::process::Child,
    std::process::ChildStdin,
    Receiver<String>,
    Receiver<String>,
    Sender<i32>,
    Sender<i32>,
) {
    let (txout, rxout): (Sender<String>, Receiver<String>) = mpsc::channel();
    let (txerr, rxerr): (Sender<String>, Receiver<String>) = mpsc::channel();
    let (txend1, rxend1): (Sender<i32>, Receiver<i32>) = mpsc::channel();
    let (txend2, rxend2): (Sender<i32>, Receiver<i32>) = mpsc::channel();

    let parts = name.split(' ').collect::<Vec<_>>();
    let name = parts[0];
    let parts = match parts.len() {
        1 => &[],
        _ => &parts[1..],
    };
    let mut child = Command::new(name)
        .args(parts)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let input = child.stdin.take().unwrap();
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();

    thread::spawn(move || {
        let mut reader = BufReader::new(&mut out);
        loop {
            if rxend1.try_recv().is_ok() {
                break;
            }
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line.is_empty() {
                continue;
            }
            txout.send(line).unwrap();
        }
    });
    thread::spawn(move || {
        let mut reader = BufReader::new(&mut err);
        loop {
            if rxend2.try_recv().is_ok() {
                break;
            }
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line.is_empty() {
                continue;
            }
            txerr.send(line).unwrap();
        }
    });

    (child, input, rxout, rxerr, txend1, txend2)
}

fn compile_cpr_tmp_file() -> Result<(), ()> {
    print!("Compiling...");
    io::stdout().flush().unwrap();

    let mut p = Popen::create(
        &format!(
            "g++ --std=c++20 -O2 cpr_tmp_file.cpp -o cpr_tmp_file -DHOUSE -Winvalid-pch {} -I{}",
            if cfg!(unix) { "" } else { "-Wl,-stack,1073741824" },
            PRECOMPILED_PATH
        )
        .split_whitespace()
        .collect::<Vec<_>>(),
        PopenConfig { ..Default::default() },
    )
    .unwrap();
    p.wait().unwrap();
    if let None = p.poll() {
        p.terminate().unwrap();
        return Err(());
    }
    let result = p.poll().unwrap();

    if !result.success() {
        return Err(());
    }
    print!("\r");
    Ok(())
}

fn get_templates_path() -> std::path::PathBuf {
    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop();
    exe_path.pop();
    exe_path.pop();
    exe_path.push("templates");
    exe_path
}

fn init_rust_directory() {
    std::fs::create_dir_all("src/bin").unwrap();
    if !std::path::Path::new("Cargo.toml").exists() {
        let lines = fs::read_to_string(get_templates_path().join("Cargo.toml")).unwrap();
        let mut file = fs::File::create("Cargo.toml").unwrap();
        let current_dir = std::env::current_dir()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        for line in lines.trim().split('\n') {
            if line.trim() == "[name]" {
                let mut name = current_dir.clone();
                if name.chars().next().unwrap().is_digit(10) {
                    name = format!("p_{}", name);
                }
                let name = format!("name = \"{}\"\n", name);
                file.write(name.as_bytes()).unwrap();
            } else if line.trim() == "[rlib]" {
                for folder in [
                    &[RUST_LIBS_PATH, "/rlib"].concat(),
                    &[RUST_LIBS_PATH, "/external"].concat(),
                ] {
                    let mut libs: Vec<(String, String)> = Vec::new();
                    for path in fs::read_dir(folder).unwrap() {
                        let path = path.unwrap().path();
                        if !path.is_dir() {
                            continue;
                        }
                        if path.join("Cargo.toml").exists() {
                            let lines = fs::read_to_string(path.join("Cargo.toml")).unwrap();
                            if let Some(name) = lines.trim().split('\n').find(|line| line.starts_with("name")) {
                                let name = name.split('=').skip(1).next().unwrap().trim();
                                libs.push((
                                    name[1..name.len() - 1].to_string(),
                                    path.file_name().unwrap().to_str().unwrap().to_string(),
                                ));
                            }
                        }
                    }
                    libs.sort();

                    for (name, path) in libs.into_iter() {
                        file.write(format!("{} = {{ \"path\" = \"{}/{}\" }}\n", name, folder, path).as_bytes())
                            .unwrap();
                    }
                    file.write(&[b'\n']).unwrap();
                }
            } else {
                file.write(line.as_bytes()).unwrap();
                file.write(&[b'\n']).unwrap();
            }
        }
        let workspace = std::env::current_dir().unwrap().parent().unwrap().join("Cargo.toml");
        if workspace.exists() {
            let lines = std::fs::read_to_string(&workspace).unwrap();
            let mut result_lines: Vec<String> = Vec::new();
            for line in lines.split('\n') {
                if line.starts_with("members") {
                    let mut members = line
                        .split_once('[')
                        .unwrap()
                        .1
                        .split_once(']')
                        .unwrap()
                        .0
                        .split(',')
                        .map(|t| t.trim())
                        .filter(|t| !t.is_empty())
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>();
                    members.push(format!("\"{}\"", current_dir));
                    result_lines.push(format!("members = [{}]", members.join(", ")));
                } else {
                    result_lines.push(line.to_string());
                }
            }
            let mut file = fs::File::create(workspace).unwrap();
            file.write_all(result_lines.join("\n").as_bytes()).unwrap();
        }
    }
    if !std::path::Path::new("rustfmt.toml").exists() {
        fs::File::create("rustfmt.toml")
            .unwrap()
            .write(
                &fs::read_to_string(get_templates_path().join("rustfmt.toml"))
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
    }
}

fn get_settings() -> Settings {
    match serde_json::from_str::<Settings>(&match fs::read_to_string(SETTINGS_FILE) {
        Ok(x) => x,
        Err(_) => "{}".to_string(),
    }) {
        Ok(x) => {
            if !x.config.contains_key(&x.profile) {
                panic!("No config for profile {}", x.profile);
            }
            x
        }
        Err(_) => {
            eprintln!("Can't parse json from \"settings.json\"");
            std::process::exit(1);
        }
    }
}
