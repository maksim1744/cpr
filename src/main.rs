use subprocess::{ExitStatus, Popen, PopenConfig, Redirection};

use std::collections::{HashSet, HashMap};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use soup::prelude::*;

use chrono::{Local};

use indoc::indoc;

use serde_json::Value;
use serde_json::map::Map;

use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use std::io::prelude::*;
use std::net::TcpListener;

use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use std::process::{Command, Stdio};

use threadpool::ThreadPool;

mod draw;
mod approx;
mod util;

use crate::util::*;

const LOCAL_PARAMS_NAME: &str = "params";

#[cfg(not(target_os = "windows"))]
const PRECOMPILED_PATH: &str = "/home/maksim/tools/precompiled/O2";
#[cfg(not(target_os = "windows"))]
const SETTINGS_FILE: &str = "/home/maksim/tools/settings/settings.json";

#[cfg(target_os = "windows")]
const PRECOMPILED_PATH: &str = "C:/MyPath/precompiled/O2";
#[cfg(target_os = "windows")]
const SETTINGS_FILE: &str = "C:/Users/magor/AppData/Local/cp_rust/settings.json";

const OPEN_FILE_WITH: &str = "subl.exe";
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

fn help() {
    let s = indoc! {"
        Usage: cpr [option] [flags]

        Use \"cpr [option] --help\" to know more about specific option

        Options:
            approx              Solve approximation problems when you need to submit only answer
            draw                Draws something
            help                Display this message
            init                Inits directory with main file and parses samples
            interact            Connects main.exe and interact.exe to test interactive problems
            mk                  Make file, write template to it and open it
            mktest              Make test case to test your solution
            multirun            Run tests created by \"cpr splittest\" using multiple threads
            parse               Parse samples from url (now only codeforces, atcoder,
                                codechef (sometimes works), cses, codingame)
            stress              Run your solution on multiple generated tests to check it
            istress             Similar to stress, but combines all source files into one
            submit              Submits solution to OJ (now only codeforces)
            splittest           Split multitest into multiple files
            test                Run your solutions on given tests in files like \"in123\"
            time                Measures execution time of a program
    "};
    print!("{}", s);
}

fn stress_test(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr stress [filename] [flags]

            Runs \"gen.exe\" to generate input, then \"easy.exe\" to generate answer to that
            input then \"[filename].exe\" (\"main.exe\" if not specified) to get output and
            then compares output to the answer.
            \"gen.exe\" should accept random seed in the first argument.
            All programs have to read and write using stdin, stdout.
            It uses files \"in\", \"out\", \"ans\" for corresponding info.

            Flags:
                --help              Display this message
                -q, --quiet         Don't display anything, except number of current test
                -s [seed]           Random seed for the first case. After each case it will
                                    be increased by 1
                --check             Run with \"check.exe\" instead of \"easy.exe\" to check
                                    output, if different answers are possible. In that case,
                                    programs are executed in the order \"gen.exe\", 
                                    \"[filename].exe\", \"check.exe\". \"check.exe\" have to
                                    read input, then output of the program and return 0 if
                                    check is successful and not 0 otherwise. Merged input
                                    and output will be written to \"inout\", where you can
                                    see it.
                --easy [cmd]        Specify command line for easy solution
                --gen [cmd]         Specify command line for generator
                --checkf [cmd]      Specify command line for checker
                --eps, -e [val]     Specify epsilon for comparison
                -t, --timeout [t]   Specify timeout in seconds (may be float)
        "};
        print!("{}", s);
        return;
    }

    let mut filename = String::from(DEFAULT_FILE_NAME);
    let mut seed: i32 = 0;
    let mut i = 0;
    let mut quiet = false;
    let mut check = false;
    let mut easy_str = String::from("easy");
    let mut gen_str = String::from("gen");
    let mut check_str = String::from("check");
    let mut timeout = DEFAULT_TIMEOUT;

    let mut epsilon: Option<f64> = None;

    while i < args.len() {
        if args[i] == "-s" {
            if i + 1 == args.len() {
                eprintln!("You need to specify seed after \"-s\"");
                std::process::exit(1);
            }
            seed = match args[i + 1].parse() {
                Ok(x) => x,
                Err(_) => {
                    eprintln!("Can't parse integer seed after \"-s\"");
                    std::process::exit(1)
                }
            };
            i += 1;
        } else if args[i] == "-q" || args[i] == "--quiet" {
            quiet = true;
        } else if args[i] == "--check" {
            check = true;
        } else if args[i] == "--easy" {
            if i + 1 == args.len() {
                eprintln!("You need to specify easy filename after \"--easy\"");
                std::process::exit(1);
            }
            easy_str = args[i + 1].clone();
            i += 1;
        } else if args[i] == "--gen" {
            if i + 1 == args.len() {
                eprintln!("You need to specify gen filename after \"--gen\"");
                std::process::exit(1);
            }
            gen_str = args[i + 1].clone();
            i += 1;
        } else if args[i] == "--checkf" {
            if i + 1 == args.len() {
                eprintln!("You need to specify check filename after \"--checkf\"");
                std::process::exit(1);
            }
            check_str = args[i + 1].clone();
            i += 1;
        } else if args[i] == "-e" || args[i] == "--eps" {
            if i + 1 == args.len() {
                eprintln!("You need to specify check epsilon after \"{}\"", args[i]);
                std::process::exit(1);
            }
            epsilon = Some(args[i + 1].parse().unwrap());
            i += 1;
        } else if args[i] == "-t" || args[i] == "--timeout" {
            timeout = args[i + 1].parse().unwrap();
            i += 1;
        } else if args[i].starts_with("-") {
            eprintln!("Unknown flag \"{}\"", args[i]);
            std::process::exit(1);
        } else {
            filename = args[i].clone();
        }
        i += 1;
    }

    let mut case = 1;

    loop {
        print!("Case #{}:  ", case);
        io::stdout().flush().unwrap();
        let result = run_and_wait(&[&fix_unix_filename(&gen_str), &seed.to_string()], "", "in", Some(timeout));
        if !result.success() {
            println!("X  [seed = {}]", seed);
            break;
        }
        print!(".");
        io::stdout().flush().unwrap();

        if !check {
            let result = run_and_wait(&[&easy_str], "in", "ans", Some(timeout));
            if !result.success() {
                println!("X  [seed = {}]", seed);
                break;
            }
            print!(".");
            io::stdout().flush().unwrap();
        }

        let result = run_and_wait(&[&filename], "in", "out", Some(timeout));
        if !result.success() {
            println!("X  [seed = {}]", seed);
            break;
        }
        print!(".");
        io::stdout().flush().unwrap();

        if check {
            let inout = [fs::read_to_string("in").unwrap(), fs::read_to_string("out").unwrap()].concat();
            fs::File::create("inout").unwrap().write(inout.as_bytes()).unwrap();

            let result = run_and_wait(&[&check_str], "inout", "ans", Some(timeout));
            if !result.success() {
                println!("X  [seed = {}]", seed);

                if !quiet {
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

        if !check && !compare_output("out", "ans", epsilon) {
            println!("   failed   [seed = {}]", seed);
            if !quiet {
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
    if !quiet {
        print!("{}", fs::read_to_string("err").unwrap());
    }
}

fn stress_test_inline(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr istress [filename] [flags]

            Combines \"main.cpp\", \"easy.cpp\" and \"gen.cpp\" into one file to run
            stress tests. All programs have to read and write using stdin, stdout
            using cin and cout. Saves test into \"in\", \"out\", \"ans\".

            Flags:
                --help              Display this message
                -q, --quiet         Don't display anything, except number of current test
                -s [seed]           Random seed for the first case. After each case it will
                                    be increased by 1
                --check             Run with \"check.cpp\" instead of \"easy.cpp\" to check
                                    output, if different answers are possible. In that case,
                                    programs are executed in the order \"gen\", 
                                    \"[filename].exe\", \"check\". \"check.cpp\" have to
                                    read input, then output of the program and return 0 if
                                    check is successful and not 0 otherwise. Merged input
                                    and output will be written to \"inout\", where you can
                                    see it.
                --easy [file]       Specify filename for easy solution
                --gen [file]        Specify filename for generator
                --checkf [file]     Specify filename for checker
                --eps, -e [val]     Specify epsilon for comparison
        "};
        print!("{}", s);
        return;
    }

    let mut filename = String::from(DEFAULT_FILE_NAME);
    let mut seed: i32 = 0;
    let mut i = 0;
    let mut quiet = false;
    let mut check = false;
    let mut easy_name = String::from("easy");
    let mut gen_name = String::from("gen");
    let mut check_name = String::from("check");

    let mut epsilon: Option<f64> = None;

    while i < args.len() {
        if args[i] == "-s" {
            if i + 1 == args.len() {
                eprintln!("You need to specify seed after \"-s\"");
                std::process::exit(1);
            }
            seed = match args[i + 1].parse() {
                Ok(x) => x,
                Err(_) => {
                    eprintln!("Can't parse integer seed after \"-s\"");
                    std::process::exit(1)
                }
            };
            i += 1;
        } else if args[i] == "-q" || args[i] == "--quiet" {
            quiet = true;
        } else if args[i] == "--check" {
            check = true;
        } else if args[i] == "--easy" {
            if i + 1 == args.len() {
                eprintln!("You need to specify easy filename after \"--easy\"");
                std::process::exit(1);
            }
            easy_name = args[i + 1].clone();
            i += 1;
        } else if args[i] == "--gen" {
            if i + 1 == args.len() {
                eprintln!("You need to specify gen filename after \"--gen\"");
                std::process::exit(1);
            }
            gen_name = args[i + 1].clone();
            i += 1;
        } else if args[i] == "--checkf" {
            if i + 1 == args.len() {
                eprintln!("You need to specify check filename after \"--checkf\"");
                std::process::exit(1);
            }
            check_name = args[i + 1].clone();
            i += 1;
        } else if args[i] == "-e" || args[i] == "--eps" {
            if i + 1 == args.len() {
                eprintln!("You need to specify check epsilon after \"{}\"", args[i]);
                std::process::exit(1);
            }
            epsilon = Some(args[i + 1].parse().unwrap());
            i += 1;
        } else if args[i].starts_with("-") {
            eprintln!("Unknown flag \"{}\"", args[i]);
            std::process::exit(1);
        } else {
            filename = args[i].clone();
        }
        i += 1;
    }

    let mut template_file = get_templates_path();
    if check {
        template_file.push("stress_test_check_template.cpp");
    } else {
        template_file.push("stress_test_template.cpp");
    }
    let template_file = template_file.to_str().unwrap();
    let template = fs::read_to_string(template_file).unwrap().trim().to_string();
    let template = template.split('\n').map(|x| x.trim_end()).collect::<Vec<_>>();

    let mut result: Vec<String> = Vec::new();
    let mut headers: Vec<String> = vec!["#include \"bits/stdc++.h\"".to_string(), "using namespace std;".to_string()];
    for line in template.iter() {
        if line.starts_with("//->settings") {
            if let Some(eps) = epsilon {
                result.push(["const double eps = ".to_string(), eps.to_string(), ";".to_string()].concat());
                result.push("const bool use_eps = true;".to_string());
            } else {
                result.push("const double eps = 0;".to_string());
                result.push("const bool use_eps = false;".to_string());
            }
            if quiet {
                result.push("const bool quiet = true;".to_string());
            } else {
                result.push("const bool quiet = false;".to_string());
            }
            result.push(["const int start_seed = ".to_string(), seed.to_string(), ";".to_string()].concat());
        } else if line.starts_with("//->") {
            let name = &line[4..];
            let file = match name {
                "main" => filename.clone(),
                "easy" => easy_name.clone(),
                "gen" => gen_name.clone(),
                "check" => check_name.clone(),
                _ => {
                    eprintln!("wrong template file");
                    std::process::exit(1);
                }
            };

            let lines = fs::read_to_string(&[file, ".cpp".to_string()].concat()).unwrap().trim().to_string();
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

    let _result = Popen::create(&[fix_unix_filename("cpr_tmp_file")], PopenConfig {
        ..Default::default()
    }).unwrap().wait();
}

fn run_tests(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr test [filename] [flags]

            Flags:
                --help              Display this message
                -q, --quiet         Don't display failed test, just the verdict
                -i [numbers]        Specify what tests to run. Numbers can be a string,
                                    such as \"1-5,8,9-20,7\" (no spaces, no quotes)
                --check             Run checker on output insted of comparing with ans
                --checkf            Specify command line for checker
                -e, --eps [value]   Specify epsilon for comparison
                -t, --timeout [t]   Specify timeout in seconds (may be float)
                --near              Print output and answer side by side
        "};
        print!("{}", s);
        return;
    }

    let mut filename = String::from(DEFAULT_FILE_NAME);

    let mut tests = get_available_tests();
    tests.sort();

    let mut i = 0;
    let mut quiet = false;
    let mut check = false;
    let mut check_str = String::from("check");

    let mut has_epsilon = false;
    let mut epsilon: f64 = 0.0;

    let mut timeout = DEFAULT_TIMEOUT;

    let mut near = false;

    while i < args.len() {
        if args[i] == "-i" {
            if i + 1 == args.len() {
                eprintln!("You need to specify tests after \"-i\"");
                std::process::exit(1);
            }

            let mut mask: HashSet<i32> = HashSet::new();

            for token in args[i + 1].split(",") {
                let token: Vec<_> = token.split("-").collect();
                if token.len() == 1 {
                    mask.insert(match token[0].parse() {
                        Ok(x) => x,
                        Err(_) => {
                            eprintln!("Wrong mask format after \"-i\"");
                            std::process::exit(1);
                        }
                    });
                } else if token.len() == 2 {
                    let l: i32 = match token[0].parse() {
                        Ok(x) => x,
                        Err(_) => {
                            eprintln!("Wrong mask format after \"-i\"");
                            std::process::exit(1);
                        }
                    };
                    let r: i32 = match token[1].parse() {
                        Ok(x) => x,
                        Err(_) => {
                            eprintln!("Wrong mask format after \"-i\"");
                            std::process::exit(1);
                        }
                    };
                    for i in l..r + 1 {
                        mask.insert(i);
                    }
                }
            }
            tests.retain(|x| mask.contains(x));
            i += 1;
        } else if args[i] == "-q" || args[i] == "--quiet" {
            quiet = true;
        } else if args[i] == "--check" {
            check = true;
        } else if args[i] == "--checkf" {
            if i + 1 == args.len() {
                eprintln!("You need to specify check filename after \"--check\"");
                std::process::exit(1);
            }
            check_str = args[i + 1].clone();
            i += 1;
        } else if args[i] == "-e" || args[i] == "--eps" {
            if i + 1 == args.len() {
                eprintln!("You need to specify epsilon filename after \"{}\"", args[i]);
                std::process::exit(1);
            }
            has_epsilon = true;
            epsilon = args[i + 1].parse().unwrap();
            i += 1;
        } else if args[i] == "-t" || args[i] == "--timeout" {
            timeout = args[i + 1].parse().unwrap();
            i += 1;
        } else if args[i] == "-t" || args[i] == "--near" {
            near = true;
        } else if args[i].starts_with("-") {
            eprintln!("Unknown flag \"{}\"", args[i]);
            std::process::exit(1);
        } else {
            filename = args[i].clone();
        }
        i += 1;
    }

    for test in tests.iter() {
        let now = Instant::now();
        print!("Case #{:<6}", format!("{}:", test));
        io::stdout().flush().unwrap();

        let result = run_and_wait(&[&filename], &["in", &test.to_string()].concat(), &["out", &test.to_string()].concat(), Some(timeout));
        let duration = now.elapsed().as_millis();
        print!("{:>5} ms   ", duration);

        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        if let ExitStatus::Other(0) = result {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
            writeln!(&mut stdout, "failed with TLE").unwrap();
            stdout.set_color(&ColorSpec::new()).unwrap();

            if !quiet {
                println!("========== in  ==========");
                println!("{}", read_lines_trim(&["in", &test.to_string()].concat()).join("\n"));
            }
        } else if !result.success() {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
            writeln!(&mut stdout, "failed with status {:?}", result).unwrap();
            stdout.set_color(&ColorSpec::new()).unwrap();

            if !quiet {
                println!("========== in  ==========");
                println!("{}", read_lines_trim(&["in", &test.to_string()].concat()).join("\n"));
                println!("========== out ==========");
                println!("{}", read_lines_trim(&["out", &test.to_string()].concat()).join("\n"));
                println!("========== err ==========");
                println!("{}", read_lines_trim("err").join("\n"));
            }
        } else if check {
            let mut in_string = fs::read_to_string(&["in", &test.to_string()].concat()).unwrap();
            if in_string.len() != 0 && in_string.as_bytes()[in_string.len() - 1] != b'\n' {
                in_string += "\n";
            }
            let out_string = fs::read_to_string(&["out", &test.to_string()].concat()).unwrap();
            let inout = [in_string, out_string].concat();
            fs::File::create(&["inout", &test.to_string()].concat()).unwrap().write(inout.as_bytes()).unwrap();

            let result = run_and_wait(&[&check_str], &["inout", &test.to_string()].concat(), &["ans", &test.to_string()].concat(), Some(timeout));
            if !result.success() {
                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
                writeln!(&mut stdout, "failed").unwrap();
                stdout.set_color(&ColorSpec::new()).unwrap();

                if !quiet {
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
        } else if !compare_output(&["out", &test.to_string()].concat(), &["ans", &test.to_string()].concat(), if has_epsilon { Some(epsilon) } else { None }) {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
            writeln!(&mut stdout, "failed").unwrap();
            stdout.set_color(&ColorSpec::new()).unwrap();

            if !quiet {
                println!("========== in  ==========");
                println!("{}", read_lines_trim(&["in", &test.to_string()].concat()).join("\n"));

                if near {
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
                } else if has_epsilon {
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

fn interact(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr interact [filename] [flags]

            Flags:
                --help              Display this message
                -q, --quiet         Don't display anything, except number of current test
                --interactf         Specify interactor filename (\"interact\" by default)
                --debug             Run interactor and main once, printing each line
                  --tab-size [val]  Number of spaces before printing \"judge:\". 20 by default
        "};
        print!("{}", s);
        return;
    }

    let mut seed = 0;
    let mut quiet = false;
    let mut filename = String::from(DEFAULT_FILE_NAME);
    let mut interact = String::from("interact");
    let mut debug = false;
    let mut tab_size = 20;

    let mut i = 0;

    while i < args.len() {
        if args[i] == "-s" {
            if i + 1 == args.len() {
                eprintln!("You need to specify seed after \"-s\"");
                std::process::exit(1);
            }
            seed = match args[i + 1].parse() {
                Ok(x) => x,
                Err(_) => {
                    eprintln!("Can't parse integer seed after \"-s\"");
                    std::process::exit(1)
                }
            };
            i += 1;
        } else if args[i] == "-q" || args[i] == "--quiet" {
            quiet = true;
        } else if args[i] == "--interactf" {
            if i + 1 == args.len() {
                eprintln!("You need to specify check filename after \"--interact\"");
                std::process::exit(1);
            }
            interact = args[i + 1].clone();
            i += 1;
        } else if args[i] == "--debug" {
            debug = true;
        } else if args[i] == "--tab-size" {
            if i + 1 == args.len() {
                eprintln!("You need to specify tab size after \"--tab-size\"");
                std::process::exit(1);
            }
            tab_size = match args[i + 1].parse() {
                Ok(x) => x,
                Err(_) => {
                    eprintln!("Can't parse integer seed after \"--tab-size\"");
                    std::process::exit(1)
                }
            };
            i += 1;
        } else if args[i].starts_with("-") {
            eprintln!("Unknown flag \"{}\"", args[i]);
            std::process::exit(1);
        } else {
            filename = args[i].clone();
        }
        i += 1;
    }

    let mut filename_vec: Vec<String> = Vec::new();
    filename_vec.extend(filename.split_whitespace().map(|x| String::from(x)).collect::<Vec<_>>());

    if cfg!(unix) {
        filename_vec[0] = ["./", &filename_vec[0]].concat().to_string();
    }

    let mut interact_vec: Vec<String> = Vec::new();
    interact_vec.extend(interact.split_whitespace().map(|x| String::from(x)).collect::<Vec<_>>());

    if cfg!(unix) {
        interact_vec[0] = ["./", &filename_vec[0]].concat().to_string();
    }

    if debug {
        let mut main = Command::new(filename_vec[0].clone());
        for i in 1..filename_vec.len() {
            main.arg(filename_vec[i].clone());
        }
        let mut main = main.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("!oops");
        let out_main = child_stream_to_vec(main.stdout.take().expect("!stdout"));
        let err_main = child_stream_to_vec(main.stderr.take().expect("!stderr"));
        let mut stdin = match main.stdin.take() {
            Some(stdin) => stdin,
            None => panic!("!stdin"),
        };

        let mut interact = Command::new(interact_vec[0].clone());
        for i in 1..interact_vec.len() {
            interact.arg(interact_vec[i].clone());
        }
        let mut interact = interact.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("!oops");
        let out_judge = child_stream_to_vec(interact.stdout.take().expect("!stdout"));
        let err_judge = child_stream_to_vec(interact.stderr.take().expect("!stderr"));
        let mut stdin_judge = match interact.stdin.take() {
            Some(stdin) => stdin,
            None => panic!("!stdin"),
        };

        let mut main_finished = false;
        let mut judge_finished = false;

        let mut stdout = StandardStream::stdout(ColorChoice::Always);

        while !main_finished || !judge_finished {
            if !err_main.lock().unwrap().is_empty() {
                if err_main.lock().unwrap().last().unwrap() == &b'\n' || main_finished {
                    let result = &String::from_utf8(err_main.lock().unwrap().to_vec()).unwrap();
                    err_main.lock().unwrap().clear();
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).unwrap();
                    for line in result.split('\n') {
                        if !line.is_empty() {
                            writeln!(&mut stdout, "main: {}", &line).unwrap();
                        }
                    }
                    stdout.set_color(&ColorSpec::new()).unwrap();
                }
            }
            if !out_main.lock().unwrap().is_empty() {
                if !main_finished && out_main.lock().unwrap().last().unwrap() == &(0 as u8) {
                    main_finished = true;
                    out_main.lock().unwrap().pop();
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).unwrap();
                    writeln!(&mut stdout, "main finished").unwrap();
                    stdout.set_color(&ColorSpec::new()).unwrap();
                }
                if !out_main.lock().unwrap().is_empty() {
                    if out_main.lock().unwrap().last().unwrap() == &b'\n' || main_finished {
                        let result = &String::from_utf8(out_main.lock().unwrap().to_vec()).unwrap();
                        out_main.lock().unwrap().clear();
                        for line in result.split('\n') {
                            if !line.is_empty() {
                                writeln!(&mut stdout, "main: {}", &line).unwrap();
                            }
                        }
                        match stdin_judge.write_all(result.as_bytes()) {
                            Err(x) => {
                                eprintln!("can't send data to judge [{}]", x);
                                judge_finished = true;
                            },
                            _ => {}
                        };
                    }
                }
            }
            if !err_judge.lock().unwrap().is_empty() {
                if err_judge.lock().unwrap().last().unwrap() == &b'\n' || judge_finished {
                    let result = &String::from_utf8(err_judge.lock().unwrap().to_vec()).unwrap();
                    err_judge.lock().unwrap().clear();
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).unwrap();
                    for line in result.split('\n') {
                        if !line.is_empty() {
                            writeln!(&mut stdout, "{:w$}judge: {}", "", &line, w = tab_size).unwrap();
                        }
                    }
                    stdout.set_color(&ColorSpec::new()).unwrap();
                }
            }
            if !out_judge.lock().unwrap().is_empty() {
                if !judge_finished && out_judge.lock().unwrap().last().unwrap() == &(0 as u8) {
                    judge_finished = true;
                    out_judge.lock().unwrap().pop();
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))).unwrap();
                    writeln!(&mut stdout, "{:w$}judge finished", "", w = tab_size).unwrap();
                    stdout.set_color(&ColorSpec::new()).unwrap();
                }
                if !out_judge.lock().unwrap().is_empty() {
                    if out_judge.lock().unwrap().last().unwrap() == &b'\n' || judge_finished {
                        let result = &String::from_utf8(out_judge.lock().unwrap().to_vec()).unwrap();
                        out_judge.lock().unwrap().clear();
                        for line in result.split('\n') {
                            if !line.is_empty() {
                                writeln!(&mut stdout, "{:w$}judge: {}", "", &line, w = tab_size).unwrap();
                            }
                        }
                        stdin.write_all(result.as_bytes()).expect("can't send data to main");
                    }
                }
            }
        }

        return;
    }

    let mut case = 1;

    loop {
        let mut p_main = match Popen::create(&filename_vec[..], PopenConfig {
            stdin: Redirection::Pipe,
            stdout: Redirection::Pipe,
            ..Default::default()
        }) {
            Ok(x) => x,
            Err(_) => {
                eprintln!("Error when starting process {:?}", filename);
                std::process::exit(1)
            }
        };

        let mut p_interact = match Popen::create(&[&interact_vec[..], &[seed.to_string()]].concat(), PopenConfig {
            stdin: Redirection::File(p_main.stdout.as_mut().unwrap().try_clone().unwrap()),
            stdout: Redirection::File(p_main.stdin.as_mut().unwrap().try_clone().unwrap()),
            stderr: Redirection::File(fs::File::create("err").unwrap()),
            ..Default::default()
        }) {
            Ok(x) => x,
            Err(_) => {
                eprintln!("Error when starting process {:?}", interact);
                std::process::exit(1)
            }
        };

        print!("Case #{}: [seed = {}] ", case, seed);
        io::stdout().flush().unwrap();

        p_interact.wait_timeout(std::time::Duration::from_secs(5)).unwrap();
        p_main.wait_timeout(std::time::Duration::from_secs(5)).unwrap();

        if let None = p_main.poll() {
            p_main.kill().unwrap();
        }

        if let None = p_interact.poll() {
            p_interact.kill().unwrap();
            println!("timeout");
            if !quiet {
                println!("{}", read_lines_trim("err").join("\n"));
            }
            break;
        }
        if !p_interact.poll().unwrap().success() {
            println!("failed");
            if !quiet {
                println!("{}", read_lines_trim("err").join("\n"));
            }
            break;
        }

        seed += 1;
        case += 1;

        print!("\r                                    \r");
    }
}

fn parse(args: &Vec<String>, params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr parse [flags]

            Parses samples using competitive companion (port 10046)

            Flags:
                --help              Display this message
                -f                  Ignore all settings and listen on port
                --contest           Parse contest
                    -n              Specify the number of problems
                     -na, -nA, -n1  Specify name of first problem
                --echo              Print full responses
        "};
        print!("{}", s);
        return;
    }

    let mut url: Option<String>;
    if params.contains_key("url") {
        url = Some(params.get("url").unwrap().clone());
    } else {
        url = None
    }

    let mut parse_contest = false;
    let mut problem_names: Vec<String> = Vec::new();
    let mut force = false;
    let mut echo = false;

    let mut i = 0;
    while i < args.len() {
        if args[i] == "-f" {
            url = None;
            force = true;
            i += 1;
        } else if args[i] == "--contest" {
            parse_contest = true;
            i += 1;
        } else if args[i].starts_with("-n") {
            if i + 1 == args.len() {
                eprintln!("You need to specify number of problems after \"-n\"");
                std::process::exit(1);
            }
            let mut first_problem = args[i][2..].to_string();
            if first_problem.is_empty() {
                first_problem = "A".to_string();
            }
            let problem_count = match args[i + 1].parse() {
                Ok(x) => x,
                Err(_) => {
                    eprintln!("Can't parse integer seed after \"-n\"");
                    std::process::exit(1)
                }
            };
            for j in 0..problem_count {
                if first_problem == "A" {
                    problem_names.push(String::from_utf8(vec![b'A' + j]).unwrap());
                } else if first_problem == "a" {
                    problem_names.push(String::from_utf8(vec![b'a' + j]).unwrap());
                } else if first_problem == "1" {
                    problem_names.push(String::from_utf8((j + 1).to_string().as_bytes().to_vec()).unwrap());
                }
            }
            i += 2;
        } else if args[i] == "--echo" {
            echo = true;
        } else if args[i].starts_with("-") {
            eprintln!("Unknown flag \"{}\"", args[i]);
            std::process::exit(1);
        } else {
            problem_names.push(args[i].clone());
            i += 1;
        }
    }

    if let Some(ref url) = url {
        if url.contains("codeforces.com") {
            let response = reqwest::blocking::get(url).unwrap().text().unwrap();
            let soup = Soup::new(&response);

            let inputs: Vec<_> = soup.tag("div").class("input").find_all().map(|x| x.tag("pre").find().unwrap().display()).collect();
            let inputs: Vec<_> = inputs.iter().map(|x| x.replace("<br>", "").replace("</br>", "\n").replace("<pre>", "").replace("</pre>", "")).collect();

            let answers: Vec<_> = soup.tag("div").class("output").find_all().map(|x| x.tag("pre").find().unwrap().display()).collect();
            let answers: Vec<_> = answers.iter().map(|x| x.replace("<br>", "").replace("</br>", "\n").replace("<pre>", "").replace("</pre>", "")).collect();

            for i in 0..inputs.len() {
                let test = first_available_test();
                fs::File::create(&["in", &test.to_string()].concat()).unwrap().write(inputs[i].as_bytes()).unwrap();
                if i < answers.len() {
                    fs::File::create(&["ans", &test.to_string()].concat()).unwrap().write(answers[i].as_bytes()).unwrap();
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

            fs::File::create(&["in", &index.to_string()].concat()).unwrap().write(input.as_bytes()).unwrap();
            if !answer.is_empty() {
                fs::File::create(&["ans", &index.to_string()].concat()).unwrap().write(answer.as_bytes()).unwrap();
            }
        }

        println!("Parsed {} tests", tests.len());
    };

    if parse_contest {
        println!("Creating problems: {:?}", problem_names);
        url = None;
    }

    if !parse_contest && !force {
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
        if parse_contest {
            if problem_iter == problem_names.len() {
                break
            } else {
                problem_name = problem_names[problem_iter].clone();
                problem_iter += 1;
            }
        }

        let mut stream = listener_iter.next().unwrap().unwrap();

        let mut buffer = [0; 4096];

        stream.read(&mut buffer).unwrap();

        let response = String::from_utf8_lossy(&buffer[..]);
        if echo {
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

        if parse_contest {
            if parsed_problems.contains(&response_url) {
                println!("duplicate");
                problem_iter -= 1;
            } else {
                contest_data.insert(problem_name, data);
                parsed_problems.insert(response_url);
            }
            let data = Value::Object(contest_data.clone());
            fs::File::create(".preparsed_samples").unwrap().write(serde_json::to_string(&data).unwrap().as_bytes()).unwrap();
        } else {
            create_tests_from_json(&data);
            return;
        }
    }

    if parse_contest {
    }
}

fn make_file(args: &Vec<String>, params: &mut HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr mk [filename] [flags]

            Creates file [filename] with template. Default extension is \".cpp\" if not
            specified.

            Flags:
                --help              Display this message
                -t, -gen, -gcj      Use template \"tstart\", \"gstart\" or \"gcj\"
                                    respectively. \"start\" is chosen by default.
        "};
        print!("{}", s);
        return;
    }
    let mut filename = String::from(DEFAULT_FILE_NAME);
    let mut extension = String::from(DEFAULT_FILE_EXTENSION);

    let mut i = 0;

    enum TemplateType {
        Start,
        Tstart,
        Gcj,
        Gstart,
    }

    let mut template_type = TemplateType::Start;

    while i < args.len() {
        if args[i] == "-t" {
            template_type = TemplateType::Tstart;
        } else if args[i] == "-gcj" {
            template_type = TemplateType::Gcj;
        } else if args[i] == "-gen" {
            template_type = TemplateType::Gstart;
        } else if args[i].starts_with("-") {
            eprintln!("Unknow flag \"{}\"", args[i]);
            std::process::exit(1);
        } else {
            let v: Vec<_> = args[i].split(".").collect();
            filename = v[0].to_string();
            if v.len() > 1 {
                extension = v[1].to_string();
            }
        }
        i += 1;
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
        TemplateType::Gstart => "gstart"
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
                let filter = line[ind+2..next_ind].split(':').collect::<Vec<_>>();
                let filter_type = filter[0];
                let values = filter[1].split('|').collect::<Vec<_>>();

                let good_line = match filter_type {
                    "os" => values.contains(&env::consts::OS),
                    "type" => values.contains(&template_type),
                    _ => false
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
        template = template.replace("\\$", "$").replace("${1:date}", &now.format("%d.%m.%Y %H:%M:%S").to_string()).to_string();

        let mut cursor_expr = "$0";
        if template.contains("${0:}") {
            cursor_expr = "${0:}";
        }

        if template.contains(cursor_expr) {
            position.0 = template[..template.find(cursor_expr).unwrap()].matches("\n").count();
            position.1 = template.split("\n").collect::<Vec<_>>()[position.0].find(cursor_expr).unwrap();

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
        if OPEN_FILE_WITH == "subl" || OPEN_FILE_WITH == "subl.exe" {
            std::process::Command::new(OPEN_FILE_WITH).arg(format!("{}:{}:{}", full_name, position.0, position.1)).output().unwrap();
        } else {
            std::process::Command::new(OPEN_FILE_WITH).arg(format!("{}", full_name)).output().unwrap();
        }
    }
}

fn init_task(args: &Vec<String>, params: &mut HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr init [flags]

            Executes \"cpr mk [flags]\" and \"cpr parse\". See \"cpr mk --help\" for more info

            Flags:
                --help              Display this message
        "};
        print!("{}", s);
        return;
    }

    make_file(args, params);
    parse(&Vec::new(), params);
}

fn submit(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cp submit [filename]

            Submits code from \"filename\" (\"main.cpp\" by default)

            Flags:
                --help              Display this message
        "};
        print!("{}", s);
        return;
    }
    let mut filename = "main.cpp";
    if !args.is_empty() {
        filename = &args[0][..];
    }

    let code = &match fs::read_to_string(filename) {
        Ok(x) => x,
        Err(_) => {
            eprintln!("File \"{}\" not found", filename);
            std::process::exit(1)
        }
    };

    let extension = filename.split(".").collect::<Vec<_>>()[1];

    if let ProblemSource::Codeforces(contest, problem) = get_problem_source() {
        let (login, password) = get_login_password("codeforces");

        let language_code = match extension {
            "cpp" => 61,
            "rs" => 49,
            "py" => 41,
            _ => {
                eprintln!("I don't know this extension");
                std::process::exit(1);
            }
        };

        let client = reqwest::blocking::Client::builder().cookie_store(true)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/86.0.4240.75 Safari/537.36").build().unwrap();
        let response = client.get("https://codeforces.com/enter").send().unwrap().text().unwrap();

        let ftaa = &thread_rng().sample_iter(&Alphanumeric).take(18).collect::<String>().to_lowercase();
        let bfaa = "58b9b2061cf94495c1ff57f24750dcf5";

        let form = [
            ("csrf_token", &extract_codeforces_csrf(&response)[..]),
            ("action", "enter"),
            ("ftaa", ftaa),
            ("bfaa", bfaa),
            ("handleOrEmail", &login),
            ("password", &password),
            ("_tta", "111"),
        ];

        let post = client.post("https://codeforces.com/enter").form(&form);

        let response = post.send().unwrap().text().unwrap();
        if response.matches("error for__password").count() != 0 {
            eprintln!("Login failed\n");
            std::process::exit(1);
        }

        let response = client.get(&format!("https://codeforces.com/contest/{}/submit", contest)).send().unwrap().text().unwrap();
        let csrf = extract_codeforces_csrf(&response);

        let form = [
            ("csrf_token", &csrf[..]),
            ("ftaa", ftaa),
            ("bfaa", bfaa),
            ("action", "submitSolutionFormSubmitted"),
            ("submittedProblemIndex", &problem),
            ("source", code),
            ("_tta", "880"),
            ("tabSize", "4"),
            ("sourceFile", ""),
            ("programTypeId", &language_code.to_string()),
        ];

        client.post(&format!("https://codeforces.com/contest/{}/submit?csrf_token={}", contest, csrf)).form(&form).send().unwrap();
    } else {
        eprintln!("Can submit only on codeforces");
        std::process::exit(1);
    }
}

fn make_test(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr mktest [index] [flags]

            Creates test from input. Split input and answer with ` (on the new line),
            answer can be empty. Input and answer will be written to \"in[index]\" and
            \"ans[index]\". If index is not specified, it will be chosen as the least
            number such that \"in[index]\" does not exist.

            Flags:
                --help              Display this message
                -0                  Copy \"in\", \"ans\" to the new test instead of reading
                                    from stdin
        "};
        print!("{}", s);
        return;
    }

    let mut index = first_available_test();
    let mut from_zero = false;
    let mut i = 0;

    while i < args.len() {
        if args[i] == "-0" {
            from_zero = true;
        } else if args[i].starts_with("-") {
            eprintln!("Unknow flag \"{}\"", args[i]);
            std::process::exit(1);
        } else {
            index = match args[0].parse() {
                Ok(x) => x,
                Err(_) => {
                    eprintln!("Can't parse integer from \"{}\"", args[0]);
                    std::process::exit(1);
                }
            };
        }
        i += 1;
    }

    if from_zero {
        fs::File::create(&["in", &index.to_string()].concat()).unwrap().write(fs::read_to_string("in").unwrap().as_bytes()).unwrap();
        if let Ok(answer) = fs::read_to_string("ans") {
            fs::File::create(&["ans", &index.to_string()].concat()).unwrap().write(answer.as_bytes()).unwrap();
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

    fs::File::create(&["in", &index.to_string()].concat()).unwrap().write(input.join("").as_bytes()).unwrap();
    if !answer.is_empty() {
        fs::File::create(&["ans", &index.to_string()].concat()).unwrap().write(answer.join("").as_bytes()).unwrap();
    }
}

fn measure_time(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr time [command_line]

            Executes [command_line] and measures execution time

            Flags:
                --help              Display this message
        "};
        print!("{}", s);
        return;
    }

    if args.is_empty() {
        eprintln!("Specify args");
        std::process::exit(1);
    }

    let mut filename_vec: Vec<String> = Vec::new();
    for item in args.iter() {
        filename_vec.extend(item.split_whitespace().map(|x| String::from(x)).collect::<Vec<_>>());
    }

    fix_unix_filename_vec(&mut filename_vec);

    let now = Instant::now();
    let mut p = match Popen::create(&filename_vec[..], PopenConfig {
        ..Default::default()
    }) {
        Ok(x) => x,
        Err(_) => {
            eprintln!("Error when starting process {:?}", filename_vec);
            std::process::exit(1)
        }
    };

    p.wait().unwrap();
    let duration = now.elapsed().as_secs_f32();

    let result = p.poll().unwrap();
    if !result.success() {
        let mut stderr = StandardStream::stderr(ColorChoice::Always);
        stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red))).unwrap();
        writeln!(&mut stderr, "failed with status {:?}", result).unwrap();
        stderr.set_color(&ColorSpec::new()).unwrap();
    }

    eprintln!("time: {:.3}", duration);
}

fn split_test(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr splittest [filename] [input]

            Splits multitest from [input] into single tests and puts them in folder \"tests\".
            Needs C++ solution [filename] which has \"/* input-end */\" after reading input
            for each test.

            Flags:
                --help              Display this message
        "};
        print!("{}", s);
        return;
    }

    let mut filename = String::from(DEFAULT_FILE_NAME);
    let mut input = String::new();

    let mut i = 0;
    let mut j = 0;

    while i < args.len() {
        if args[i].starts_with("-") {
            eprintln!("Unknown flag \"{}\"", args[i]);
            std::process::exit(1);
        } else {
            if j == 0 {
                filename = args[i].clone();
            } else if j == 1 {
                input = args[i].clone();
            } else {
                eprintln!("Too many args");
                std::process::exit(1);
            }
            j += 1;
        }
        i += 1;
    }

    if j != 2 {
        eprintln!("Need two arguments");
        std::process::exit(1);
    }

    filename = [filename, String::from("."), DEFAULT_FILE_EXTENSION.to_string()].concat();

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
    let mut split_positions = split_positions.iter().map(|x| x.parse::<usize>().unwrap()).collect::<Vec<usize>>();
    split_positions.insert(0, 0);

    let input = fs::read_to_string(input).unwrap().trim().to_string();
    split_positions.insert(1, input.find('\n').unwrap());

    fs::create_dir_all("tests").unwrap();
    fs::remove_dir_all("tests").unwrap();
    fs::create_dir_all("tests").unwrap();

    for i in 1..split_positions.len()-1 {
        let test = input[split_positions[i] + 1 .. split_positions[i + 1]].to_string();

        let mut file = fs::File::create(format!("tests/{:0>3}", i)).unwrap();
        file.write(&["1\n".to_string(), test, "\n".to_string()].concat().as_bytes()).unwrap();
    }

    print!("\r                                    ");
    println!("\rCreated {} tests", split_positions.len() - 2);
}

fn multirun(args: &Vec<String>, _params: &HashMap<String, String>) {
    if !args.is_empty() && args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr multirun [filename]

            Flags:
                --help              Display this message
                -t [num]            Number of threads
                -o [file]           Output filename
        "};
        print!("{}", s);
        return;
    }

    let mut filename = String::from(DEFAULT_FILE_NAME);
    let mut threads = 8;
    let mut output: Option<String> = None;

    let mut i = 0;

    while i < args.len() {
        if args[i] == "-t" {
            if i + 1 == args.len() {
                eprintln!("You need to specify number of threads after \"-t\"");
                std::process::exit(1);
            }
            threads = args[i + 1].parse().unwrap();
            i += 1;
        } else if args[i] == "-o" {
            if i + 1 == args.len() {
                eprintln!("You need to specify output filename after \"-o\"");
                std::process::exit(1);
            }
            output = Some(args[i + 1].clone());
            i += 1;
        } else if args[i].starts_with("-") {
            eprintln!("Unknown flag \"{}\"", args[i]);
            std::process::exit(1);
        } else {
            filename = args[i].clone();
        }
        i += 1;
    }

    filename = [filename, String::from("."), DEFAULT_FILE_EXTENSION.to_string()].concat();

    let mut new_main: Vec<String> = Vec::new();
    let file = fs::read_to_string(filename).unwrap().trim().to_string();
    let file = file.split('\n').map(|x| x.trim_end()).collect::<Vec<_>>();

    for &line in file.iter() {
        if line.starts_with("int main(") {
            new_main.push("int main(int argc, char *argv[]) {".to_string());
        } else if line.contains("cout << \"Case #\"") {
            new_main.push(format!("{}cout << \"Case #\" << stoi(argv[1]) << \": \";",
                String::from_utf8(vec![b' '; line.find('c').unwrap()]).unwrap()));
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
        .map(|x| (x.path().file_name().unwrap().to_str().unwrap().to_string(), x.metadata().unwrap().len()))
        .collect::<Vec<_>>();
    tests.sort_by(|a, b| b.1.cmp(&a.1));
    let tests = tests.into_iter()
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
    print!("{}", String::from_utf8(result_string_mutex.lock().unwrap().to_vec()).unwrap());
    io::stdout().flush().unwrap();

    let failed_tests = Arc::new(Mutex::new(0));

    let pool = ThreadPool::new(threads);

    for input in tests.iter() {
        let input = input.clone();
        let local_result_string = result_string_mutex.clone();
        let local_failed_tests = failed_tests.clone();

        pool.execute(move || {
            let test_num = input.parse::<usize>().unwrap();

            local_result_string.lock().unwrap()[test_num] = b'.';
            print!("\r{}", String::from_utf8(local_result_string.lock().unwrap().to_vec()).unwrap());
            io::stdout().flush().unwrap();

            let result = run_and_wait(&[&fix_unix_filename("cpr_tmp_file"), &input], &format!("tests/{}", input), &format!("tests/{}_out", input), None);

            if result.success() {
                local_result_string.lock().unwrap()[test_num] = b'+';
            } else {
                local_result_string.lock().unwrap()[test_num] = b'X';
                *local_failed_tests.lock().unwrap() += 1;
            }
            print!("\r{}", String::from_utf8(local_result_string.lock().unwrap().to_vec()).unwrap());
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

    if let Some(output) = output {
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

// ************************************* main *************************************


fn main() {
    let args: Vec<String> = env::args().collect::<Vec<String>>()[1..].to_vec();

    let mut params = get_params();

    if args.len() == 0 || args[0] == "help" {
        help();
    } else if args[0] == "stress" {
        stress_test(&args[1..].to_vec(), &params);
    } else if args[0] == "istress" {
        stress_test_inline(&args[1..].to_vec(), &params);
    } else if args[0] == "test" {
        run_tests(&args[1..].to_vec(), &params);
    } else if args[0] == "interact" {
        interact(&args[1..].to_vec(), &params);
    } else if args[0] == "parse" {
        parse(&args[1..].to_vec(), &params);
    } else if args[0] == "mk" {
        make_file(&args[1..].to_vec(), &mut params);
    } else if args[0] == "init" {
        init_task(&args[1..].to_vec(), &mut params);
    } else if args[0] == "submit" {
        submit(&args[1..].to_vec(), &params);
    } else if args[0] == "mktest" {
        make_test(&args[1..].to_vec(), &params);
    } else if args[0] == "draw" {
        draw::draw(&args[1..].to_vec(), &params);
    } else if args[0] == "time" {
        measure_time(&args[1..].to_vec(), &params);
    } else if args[0] == "splittest" {
        split_test(&args[1..].to_vec(), &params);
    } else if args[0] == "multirun" {
        multirun(&args[1..].to_vec(), &params);
    } else if args[0] == "approx" {
        approx::approx(&args[1..].to_vec(), &params);
    } else {
        eprintln!("Unknown option \"{}\"", args[0]);
        std::process::exit(1);
    }
}


// *********************************** internal ***********************************

fn run_and_wait(filename: &[&str], fin: &str, fout: &str, timeout: Option<f64>) -> ExitStatus {
    let stdin = match fin {
        "" => Redirection::Pipe,
        name => Redirection::File(fs::File::open(name).unwrap())
    };
    let stdout = match fout {
        "" => Redirection::Pipe,
        name => Redirection::File(fs::File::create(name).unwrap())
    };

    let mut filename_vec: Vec<String> = Vec::new();
    for &item in filename.iter() {
        filename_vec.extend(item.split(" ").map(|x| String::from(x)).collect::<Vec<_>>());
    }

    fix_unix_filename_vec(&mut filename_vec);

    let mut p = match Popen::create(&filename_vec[..], PopenConfig {
        stdin: stdin,
        stdout: stdout,
        stderr: Redirection::File(fs::File::create("err").unwrap()),
        ..Default::default()
    }) {
        Ok(x) => x,
        Err(_) => {
            eprintln!("Error when starting process {:?}", filename_vec);
            std::process::exit(1)
        }
    };

    if let Some(timeout) = timeout {
        p.wait_timeout(std::time::Duration::from_millis((timeout * 1000.0).round() as u64)).unwrap();
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
    let mut res = fs::read_to_string(filename).unwrap().trim_end().split("\n").map(String::from).collect::<Vec<_>>();
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
    let mut v: Vec<_> = fs::read_dir(".").unwrap().map(|x| x.unwrap().path().file_name().unwrap().to_str().unwrap().to_string()).collect();
    v.retain(|x| x.starts_with("in") && match x[2..].parse::<i32>() { Ok(_) => true, Err(_) => false });
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
        let folder = std::env::current_dir().unwrap().as_path().file_name().unwrap().to_str().unwrap().to_string();
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

fn add_param(key: &str, value :&str, params: &mut HashMap<String, String>) {
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
            Err(_) => -1
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
            return Some(format!("https://codeforces.com/contest/{}/problem/{}", contest, problem));
        } else {
            return Some(format!("https://codeforces.com/gym/{}/problem/{}", contest, problem));
        }
    } else if let ProblemSource::CodeChef(contest, problem) = problem_source {
        return Some(format!("https://www.codechef.com/{}/problems/{}", contest, problem));
    } else if let ProblemSource::AtCoder(contest, problem) = problem_source {
        return Some(format!("https://atcoder.jp/contests/{0}/tasks/{0}_{1}", contest, problem.to_lowercase()));
    } else if let ProblemSource::CodinGamePuzzle(problem) = problem_source {
        return Some(format!("https://www.codingame.com/ide/puzzle/{}", problem));
    } else if let ProblemSource::Cses(problem) = problem_source {
        return Some(format!("https://cses.fi/problemset/task/{}", problem));
    }
    None
}

fn extract_codeforces_csrf(html: &str) -> String {
    let search_for = "data-csrf='";
    let idx = html.find(search_for).unwrap() + search_for.len();
    html[idx..idx + 32].to_string()
}

fn get_login_password(source: &str) -> (String, String) {
    let settings: Value = match serde_json::from_str(& match fs::read_to_string(SETTINGS_FILE) {
        Ok(x) => x,
        Err(_) => {
            eprintln!("Can't open \"settings.json\"");
            std::process::exit(1);
        }
    }) {
        Ok(x) => x,
        Err(_) => {
            eprintln!("Can't parse json from \"settings.json\"");
            std::process::exit(1);
        }
    };
    let login = match settings["auth"][source]["login"].as_str() {
        Some(x) => x,
        None => {
            eprintln!("Can't find {} login in \"settings.json\"", source);
            std::process::exit(1);
        }
    };
    let password = match settings["auth"][source]["password"].as_str() {
        Some(x) => x,
        None => {
            eprintln!("Can't find {} password in \"settings.json\"", source);
            std::process::exit(1);
        }
    };
    (login.to_string(), password.to_string())
}

fn child_stream_to_vec<R>(mut stream: R) -> Arc<Mutex<Vec<u8>>> where R: Read + Send + 'static {
    let out = Arc::new(Mutex::new(Vec::new()));
    let vec = out.clone();
    thread::Builder::new()
        .name("child_stream_to_vec".into())
        .spawn(move || loop {
            let mut buf = [0];
            match stream.read(&mut buf) {
                Err(err) => {
                    println!("[{}] Error reading from stream: {}", line!(), err);
                    break;
                }
                Ok(got) => {
                    if got == 0 {
                        vec.lock().expect("!lock").push(0);
                        break;
                    } else if got == 1 {
                        vec.lock().expect("!lock").push(buf[0]);
                    } else {
                        println!("[{}] Unexpected number of bytes: {}", line!(), got);
                        break;
                    }
                }
            }
        })
        .expect("!thread");
    out
}

fn compile_cpr_tmp_file() -> Result<(), ()> {
    print!("Compiling...");
    io::stdout().flush().unwrap();

    let mut p = Popen::create(&format!("g++ --std=c++20 -O2 cpr_tmp_file.cpp -o cpr_tmp_file -DHOUSE -Winvalid-pch {} -I{}",
                                       if cfg!(unix) { "" } else { "-Wl,-stack,1073741824" },
                                       PRECOMPILED_PATH)
        .split_whitespace().collect::<Vec<_>>(),
        PopenConfig {
            ..Default::default()
    }).unwrap();
    p.wait().unwrap();
    if let None = p.poll() {
        p.terminate().unwrap();
        return Err(());
    }
    let result = p.poll().unwrap();

    if !result.success() {
        return Err(())
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
