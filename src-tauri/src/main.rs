#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    match linen_tray::relaunch_from_runtime() {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            eprintln!("Could not launch the installed runtime: {error}");
            std::process::exit(1);
        }
    }
    homing::run();
}
