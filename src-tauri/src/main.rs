// Prevents an additional console window from appearing on Windows in
// release builds. Does not affect other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    laserprep_lib::run();
}
