use std::collections::{HashMap};

use indoc::indoc;

mod draw_points;
mod draw_tree;

pub fn draw(args: &Vec<String>, _params: &HashMap<String, String>) {
    if args.is_empty() || args[0] == "--help" {
        let s = indoc! {"
            Usage: cpr draw [type] [flags]

            Draws. Use \"cpr draw [type] --help\" for more info.

            Types:
                pts, points         Draw points on a plane
                tree                Draw tree

            Flags:
                --help              Display this message
        "};
        print!("{}", s);
        return;
    }
    if args[0] == "pts" || args[0] == "points" {
        draw_points::draw(&args[1..].to_vec(), _params);
    } else if args[0] == "tree" {
        draw_tree::draw(&args[1..].to_vec(), _params);
    } else {
        eprintln!("Unknown option \"{}\"", args[0]);
        std::process::exit(1);
    }
}
