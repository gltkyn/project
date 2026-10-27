//! project
//!
//!
//! TODO: Core application.
//! 2026-10-24 18:51
//! [ ] Use ratatui
//! [ ] Use serde for persistence.
//! ## Modules
//! 2026-10-24 18:52
//! [ ] Make todo list
//! [ ] 

// TODO: The todo of my todo.
//
// use std::fs;
//
// trait Entry: Serialize + DeserializeOwned + Clone {
//     const FILE: &'static str; // "todos.json", "pages.json"
//     fn id(&self) -> u64;
//     fn title(&self) -> &str;
//     fn body(&self) -> &str {
//         "" // default: nothing extra to search
//     }
// }
//
// #[derive(Clone, Serialize, Deserialize)]
// struct Todo {
//     id: u64,
//     header: String,
//     is_done: bool,
// }
//
// impl Entry for Todo {
//     const FILE: &'static str = "todos.json";
//     fn id(&self) -> u64 {
//         self.id
//     }
//     fn title(&self) -> &str {
//         &self.header
//     }
// }

use ratatui;

struct App {
    modules: 
}

fn main() {
    let mut app = App::new();
    let mut terminal = ratatui::init(); // also installs a panic hook that restores the terminal
    let result = app.run(&mut terminal);
    ratatui::restore();
    if let Err(e) = result {
        eprintln!("error: {e}");
    }
}
