#![windows_subsystem = "windows"]

pub mod graph;
pub mod graph_app;
pub mod graph_errors;
pub mod graph_flows;
pub mod graph_parser;
pub mod graph_renderer;

fn main() {
    std::env::set_var("GTK_USE_PORTAL", "1");

    crate::graph_app::graph_window::init_app();
}
