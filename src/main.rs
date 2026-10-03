#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
mod config;

#[cfg(target_os = "windows")]
mod cursor;

#[cfg(target_os = "windows")]
mod fix;

#[cfg(target_os = "windows")]
mod gui;

#[cfg(target_os = "windows")]
mod process;

#[cfg(target_os = "linux")]
mod process;

fn main() {
    #[cfg(target_os = "windows")]
    {
        gui::run_gui();
    }

    #[cfg(target_os = "linux")]
    {
        println!("GUI is currently only supported on Windows.");
    }
}
