// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use arknights_frame_assistant_macos_lib::startup;

fn main() {
    startup::run();
}
