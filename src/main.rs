use subprocess::{ExitStatus, Popen, PopenConfig, Redirection};

use std::collections::{HashSet, HashMap};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Instant};

use soup::prelude::*;

use chrono::{Local};

use indoc::indoc;

use serde_json::Value;

use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod draw;

const LOCAL_PARAMS_NAME: &str = "params";

const SETTINGS_FILE: &str = "C:/Users/magor/AppData/Local/cp_rust/settings.json";
const SETTINGS_FILE_BASH: &str = "/mnt/c/Users/magor/AppData/Local/cp_rust/settings.json";
const TEMPLATE_PATH: &str = "C:/Users/magor/AppData/Roaming/Sublime Text 3/Packages/User/Snippets/";

const OPEN_FILE_WITH: &str = "subl";

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
            draw                Draws something
            help                Display this message
            init                Inits directory with main file and parses samples
            interact            Connects main.exe and interact.exe to test interactive problems
            mk                  Make file, write template to it and open it
            mktest              Make test case to test your solution
            parse               Parse samples from url (now only codeforces, atcoder,
                                codechef (sometimes works), cses, codingame)
            stress              Run your solution on multiple generated tests to check it
            submit              Submits solution to OJ (now only codeforces)
            test                Run your solutions on given tests in files like \"in123\"
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

    let mut filename = get_main_filename();
    let mut seed: i32 = 0;
    let mut i = 0;
    let mut quiet = false;
    let mut check = false;
    let mut easy_str = String::from("easy");
    let mut gen_str = String::from("gen");
    let mut check_str = String::from("check");
    let mut timeout = 5_f64;

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
        let result = run_and_wait(&[&gen_str, &seed.to_string()], "", "in", timeout);
        if !result.success() {
            println!("X  [seed = {}]", seed);
            break;
        }
        print!(".");
        io::stdout().flush().unwrap();

        if !check {
            let result = run_and_wait(&[&easy_str], "in", "ans", timeout);
            if !result.success() {
                println!("X  [seed = {}]", seed);
                break;
            }
            print!(".");
            io::stdout().flush().unwrap();
        }

        let result = run_and_wait(&[&filename], "in", "out", timeout);
        if !result.success() {
            println!("X  [seed = {}]", seed);
            break;
        }
        print!(".");
        io::stdout().flush().unwrap();

        if check {
            let inout = [fs::read_to_string("in").unwrap(), fs::read_to_string("out").unwrap()].concat();
            fs::File::create("inout").unwrap().write(inout.as_bytes()).unwrap();

            let result = run_and_wait(&[&check_str], "inout", "ans", timeout);
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
        "};
        print!("{}", s);
        return;
    }

    let mut filename = get_main_filename();

    let mut tests = get_available_tests();
    tests.sort();

    let mut i = 0;
    let mut quiet = false;
    let mut check = false;
    let mut check_str = String::from("check");

    let mut has_epsilon = false;
    let mut epsilon: f64 = 0.0;

    let mut timeout = 5_f64;

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

        let result = run_and_wait(&[&filename], &["in", &test.to_string()].concat(), &["out", &test.to_string()].concat(), timeout);
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

            let result = run_and_wait(&[&check_str], &["inout", &test.to_string()].concat(), &["ans", &test.to_string()].concat(), timeout);
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
        "};
        print!("{}", s);
        return;
    }

    let mut seed = 0;
    let mut quiet = false;
    let mut filename = String::from("main");
    let mut interact = String::from("interact");

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
            Usage: cpr parse [url] [flags]

            Parses samples from url. If not specified, then uses url from file \"params\".
            If there is no url, then takes url from file \"params\" in the parent folder
            and adds \"/[current_folder]\" to it.

            Flags:
                --help              Display this message
        "};
        print!("{}", s);
        return;
    }

    let url: String;
    if !args.is_empty() {
        url = args[0].clone();
    } else if params.contains_key("url") {
        url = params.get("url").unwrap().clone();
    } else {
        eprintln!("Can't find url anywhere");
        std::process::exit(1);
    }

    println!("Parsing from url \"{}\"", url);

    if url.contains("codeforces.com") {
        let response = reqwest::blocking::get(&url).unwrap().text().unwrap();
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
    } else if url.contains("codechef.com") {
        let mut v: Vec<_> = url.split("/").collect();
        while v.last().unwrap().is_empty() {
            v.pop();
        }
        let contest = v[v.len() - 3];
        let problem = v[v.len() - 1];

        let url = format!("https://www.codechef.com/api/contests/{}", contest);

        let response = reqwest::blocking::get(&url).unwrap().text().unwrap();
        let data: Value = serde_json::from_str(&response).unwrap();
        let mut samples = data["problems_data"][problem]["body"].to_string();

        if samples == "null" {
            let url = format!("https://www.codechef.com/api/contests/{}/problems/{}", contest, problem);
            let response = reqwest::blocking::get(&url).unwrap().text().unwrap();
            let data: Value = serde_json::from_str(&response).unwrap();
            samples = data["body"].to_string();
            if samples == "null" {
                eprintln!("Can't read samples");
                std::process::exit(1);
            }
        }

        let mut inputs: Vec<String> = Vec::new();
        let mut answers: Vec<String> = Vec::new();

        for item in samples.split("###") {
            if item.to_lowercase().contains("sample input") {
                let item = (&item["Sample Input:".len()..])
                    .replace("\\n", "\n")
                    .replace("\\r", "\r")
                    .replace("\\t", "")
                    .replace("```", "").trim().to_string();
                inputs.push(item);
            }
            if item.to_lowercase().contains("sample output") {
                let item = (&item["Sample Output:".len()..])
                    .replace("\\n", "\n")
                    .replace("\\r", "\r")
                    .replace("\\t", "")
                    .replace("```", "").trim().to_string();
                answers.push(item);
            }
        }

        for i in 0..inputs.len() {
            let ind = first_available_test();
            fs::File::create(&["in", &ind.to_string()].concat()).unwrap().write(inputs[i].as_bytes()).unwrap();
            if i < answers.len() {
                fs::File::create(&["ans", &ind.to_string()].concat()).unwrap().write(answers[i].as_bytes()).unwrap();
            }
        }

        println!("Parsed {} tests from codechef", inputs.len());
    } else if url.contains("atcoder.jp") {
        let mut problem_response = reqwest::blocking::get(&url).unwrap();
        if let reqwest::StatusCode::OK = problem_response.status() {
        } else {
            eprintln!("logging in...");
            let client = reqwest::blocking::Client::builder().cookie_store(true)
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/86.0.4240.75 Safari/537.36").build().unwrap();
            let response = client.get("https://atcoder.jp/login").send().unwrap().text().unwrap();

            let (login, password) = get_login_password("atcoder");

            let form = [
                ("csrf_token", &extract_atcoder_csrf(&response)[..]),
                ("username", &login),
                ("password", &password),
            ];

            let post = client.post("https://atcoder.jp/login").form(&form);
            let response = post.send().unwrap().text().unwrap();
            if response.contains("Username or Password is incorrect") {
                eprintln!("Login failed\n");
                std::process::exit(1);
            }

            problem_response = client.get(&url).send().unwrap();
        }

        let soup = Soup::new(&problem_response.text().unwrap());
        let mut pres: Vec<_> = soup.tag("pre").find_all().collect();
        pres = pres[2 - pres.len()/2 % 2..pres.len() / 2].to_vec();

        for i in 0..pres.len() / 2 {
            let input = pres[i * 2].text();
            let answer = pres[i * 2 + 1].text();
            let test = first_available_test();
            fs::File::create(&["in", &test.to_string()].concat()).unwrap().write(input.as_bytes()).unwrap();
            fs::File::create(&["ans", &test.to_string()].concat()).unwrap().write(answer.as_bytes()).unwrap();
        }

        println!("Parsed {} tests from atcoder", pres.len() / 2);
    } else if url.contains("codingame.com/ide/puzzle") {
        let problem = url.split("/").collect::<Vec<_>>().last().unwrap().to_string();
        let client = reqwest::blocking::Client::builder().cookie_store(true)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/86.0.4240.75 Safari/537.36").build().unwrap();

        let response = client.post("https://www.codingame.com/services/Puzzle/generateSessionFromPuzzlePrettyId")
            .body(format!("[null, \"{}\", false]", problem)).send().unwrap().text().unwrap();

        let handle: Value = serde_json::from_str(&response).unwrap();
        let handle = handle["handle"].to_string();
        let handle = handle[1..handle.len() - 1].to_string();  // remove quotes

        let response = client.post("https://www.codingame.com/services/TestSession/startTestSession")
            .body(format!("[\"{}\"]", handle)).send().unwrap().text().unwrap();

        let data: Value = serde_json::from_str(&response).unwrap();
        let data: Vec<Value> = serde_json::from_str(&data["currentQuestion"]["question"]["testCases"].to_string()).unwrap();

        print!("Downloaded 0 tests out of {}", data.len());
        io::stdout().flush().unwrap();

        for (i, item) in data.iter().enumerate() {
            let input_id = item["inputBinaryId"].to_string();
            let answer_id = item["outputBinaryId"].to_string();
            let input = client.get(&format!("https://static.codingame.com/servlet/fileservlet?id={}", input_id)).send().unwrap().text().unwrap();
            let answer = client.get(&format!("https://static.codingame.com/servlet/fileservlet?id={}", answer_id)).send().unwrap().text().unwrap();
            let ind = first_available_test();
            fs::File::create(&["in", &ind.to_string()].concat()).unwrap().write(input.as_bytes()).unwrap();
            fs::File::create(&["ans", &ind.to_string()].concat()).unwrap().write(answer.as_bytes()).unwrap();
            print!("\rDownloaded {} tests out of {}", i + 1, data.len());
            io::stdout().flush().unwrap();
        }

        println!("\nParsed {} tests from codingame", data.len());
    } else if url.contains("cses.fi/problemset") {
        let response = reqwest::blocking::get(&url).unwrap().text().unwrap();

        let response = response.to_string().replace("<br />\r\n", "\n");
        let response = response[response.find("<b id=\"example").unwrap()..].to_string();
        let code_open = "<code>";
        let code_close = "</code>";

        let mut code_opens: Vec<usize> = Vec::new();
        let mut code_closes: Vec<usize> = Vec::new();
        for i in 0..response.len() {
            if i + code_open.len() <= response.len() && &response[i..i+code_open.len()] == code_open {
                code_opens.push(i + code_open.len());
            } else if i + code_close.len() <= response.len() && &response[i..i+code_close.len()] == code_close {
                code_closes.push(i);
            }
        }

        let mut inputs: Vec<String> = Vec::new();
        let mut answers: Vec<String> = Vec::new();

        for i in 0..code_opens.len() {
            if i % 2 == 0 {
                inputs.push(response[code_opens[i]..code_closes[i]].to_string());
            } else {
                answers.push(response[code_opens[i]..code_closes[i]].to_string());
            }
        }

        for i in 0..inputs.len() {
            let test = first_available_test();
            fs::File::create(&["in", &test.to_string()].concat()).unwrap().write(inputs[i].as_bytes()).unwrap();
            if i < answers.len() {
                fs::File::create(&["ans", &test.to_string()].concat()).unwrap().write(answers[i].as_bytes()).unwrap();
            }
        }

        println!("Parsed {} tests from cses", inputs.len());
    } else {
        eprintln!("I don't know how to parse from this url :(");
        std::process::exit(1);
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
    let mut filename = String::from("main");
    let mut extension = String::from("cpp");

    let mut i = 0;

    enum TemplateType {
        Start,
        Tstart,
        Gcj,
        Gstart,
    };

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

    let mut folder = String::from(TEMPLATE_PATH);
    if extension == "cpp" {
        folder.push_str("C++/");
    } else if extension == "rs" {
        folder.push_str("Rust/");
    } else if extension == "c" {
        folder.push_str("C/");
    } else if extension == "kt" {
        folder.push_str("Kotlin/");
    } else if extension == "java" {
        folder.push_str("Java/");
    } else if extension == "go" {
        folder.push_str("Go/");
    } else {
        folder.clear();
    }

    let mut position = (0, 0);

    if !folder.is_empty() {
        let mut available_templates: Vec<_> = fs::read_dir(&folder).unwrap().map(|x| x.unwrap().path().file_name().unwrap().to_str().unwrap().to_string()).collect();
        available_templates.retain(|x| x.ends_with(".sublime-snippet"));
        available_templates = available_templates.iter().map(|x| x.replace(".sublime-snippet", "")).collect();

        let mut template = "start";
        match template_type {
            TemplateType::Tstart => if available_templates.contains(&"tstart".to_string()) { template = "tstart"; },
            TemplateType::Gstart => if available_templates.contains(&"gstart".to_string()) { template = "gstart"; },
            TemplateType::Gcj    => if available_templates.contains(&"gcj"   .to_string()) { template = "gcj";    },
            _ => (),
        }

        let mut template = fs::read_to_string(&[&folder, template, ".sublime-snippet"].concat()).unwrap().trim().to_string();
        let ir = template.find("]]></content>").unwrap();
        let il = template.find("<![CDATA[").unwrap() + "<![CDATA[".len();

        let now = Local::now();
        template = template[il..ir].replace("\\$", "$").replace("${1:date}", &now.format("%d.%m.%Y %H:%M:%S").to_string()).to_string();

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

    std::process::Command::new(OPEN_FILE_WITH).arg(format!("{}:{}:{}", full_name, position.0, position.1)).output().unwrap();
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
    if !args.is_empty() && args[0] == "__help" {
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
            break;
        }
        if let Ok(0) = read_result {
            break;
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

    if cfg!(unix) {
        filename_vec[0] = ["./", &filename_vec[0]].concat().to_string();
    }

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

    eprintln!("time: {:.3}", duration);
}

// ************************************* main *************************************


fn main() {
    let args: Vec<String> = env::args().collect::<Vec<String>>()[1..].to_vec();

    let mut params = get_params();

    if args.len() == 0 || args[0] == "help" {
        help();
    } else if args[0] == "stress" {
        stress_test(&args[1..].to_vec(), &params);
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
    } else if args[0] == "todo" {
        println!("cpr test --check");
        println!("cpr param");
        println!("cpr interact");
        println!("cpr time");
    } else {
        eprintln!("Unknown option \"{}\"", args[0]);
        std::process::exit(1);
    }
}


// *********************************** internal ***********************************

fn run_and_wait(filename: &[&str], fin: &str, fout: &str, timeout: f64) -> ExitStatus {
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

    if cfg!(unix) {
        filename_vec[0] = ["./", &filename_vec[0]].concat().to_string();
    }

    let mut p = match Popen::create(&filename_vec[..], PopenConfig {
        stdin: stdin,
        stdout: stdout,
        stderr: Redirection::File(fs::File::create("err").unwrap()),
        ..Default::default()
    }) {
        Ok(x) => x,
        Err(_) => {
            eprintln!("Error when starting process {:?}", filename);
            std::process::exit(1)
        }
    };

    p.wait_timeout(std::time::Duration::from_millis((timeout * 1000.0).round() as u64)).unwrap();

    if let None = p.poll() {
        p.terminate().unwrap();
        return ExitStatus::Other(0);
    }
    p.poll().unwrap()
}

fn read_lines_trim(filename: &str) -> Vec<String> {
    let mut res = fs::read_to_string(filename).unwrap().trim().split("\n").map(String::from).collect::<Vec<_>>();
    for i in 0..res.len() {
        res[i] = res[i].trim().to_string();
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

fn get_main_filename() -> String {
    String::from("main")
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

fn extract_atcoder_csrf(html: &str) -> String {
    let search_for = "var csrfToken = \"";
    let idx = html.find(search_for).unwrap() + search_for.len();
    html[idx..idx + 44].to_string()
}

fn get_login_password(source: &str) -> (String, String) {
    let settings_file: &str;
    if cfg!(unix) {
        settings_file = SETTINGS_FILE_BASH;
    } else {
        settings_file = SETTINGS_FILE;
    }
    let settings: Value = match serde_json::from_str(& match fs::read_to_string(settings_file) {
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
