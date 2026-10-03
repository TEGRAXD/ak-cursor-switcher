#![windows_subsystem = "windows"]

mod config;
mod cursor;
mod fix;
mod gui;
mod process;

fn main() {
    gui::run_gui();
}
