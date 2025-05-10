use std::collections::HashMap;

use clap::{Parser, Subcommand};
use draw_graph::GraphArgs;
use draw_matrix::MatrixArgs;
use draw_points::PointsArgs;
use draw_tree::TreeArgs;

mod draw_graph;
mod draw_matrix;
mod draw_points;
mod draw_tree;

#[derive(Subcommand)]
enum Options {
    /// Points on a plane
    Points(PointsArgs),
    /// Tree
    Tree(TreeArgs),
    /// Graph
    Graph(GraphArgs),
    /// Matrix
    Matrix(MatrixArgs),
}

#[derive(Parser)]
pub struct DrawArgs {
    #[command(subcommand)]
    option: Options,
}

pub fn draw(args: DrawArgs, _params: &HashMap<String, String>) {
    match args.option {
        Options::Points(args) => draw_points::draw(args, _params),
        Options::Tree(args) => draw_tree::draw(args, _params),
        Options::Graph(args) => draw_graph::draw(args, _params),
        Options::Matrix(args) => draw_matrix::draw(args, _params),
    }
}
