// Su Windows, in release, niente finestra del terminale accanto all'app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    datini_lib::run()
}
