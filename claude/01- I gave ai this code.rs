use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::io::Write;

enum Command {
    List,
    Add,
    Rm,
    Done,
}
#[derive(Clone, Serialize, Deserialize)]
struct Todo {
    id: u64,
    header: String,
    is_done: bool,
    // date added,
}

#[derive(Clone, Serialize, Deserialize)]
enum Kind {
    Added,
    Removed,
    Done,
}

#[derive(Clone, Serialize, Deserialize)]
struct Event {
    id: u64,        // this event's own id, unique forever
    kind: Kind,     // what happened
    todo_id: u64,   // which todo it happened to
    header: String, // the title (only meaningful for Added)
}

fn read_line() -> String {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().to_string()
}

fn log_event(event: &Event) {
    let line = serde_json::to_string(event).expect("Failed to turn event into text");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("history.jsonl")
        .expect("Failed to open history.jsonl");
    writeln!(file, "{}", line).expect("Failed to write event");
}

fn load_log() -> Vec<Event> {
    match fs::read_to_string("history.jsonl") {
        Ok(text) => text
            .lines()
            .map(|line| serde_json::from_str(line).expect("bad line in history.jsonl"))
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn main_menu() -> char {
    read_line().chars().next().unwrap_or(' ')
}

fn save(todos: &Vec<Todo>) {
    let text = serde_json::to_string_pretty(todos).expect("Failed to turn todos into text");
    fs::write("todos.json", text).expect("Failed to write todos.json");
}

fn load() -> Vec<Todo> {
    match fs::read_to_string("todos.json") {
        Ok(text) => serde_json::from_str(&text).expect("todos.json is corrupted"),
        Err(_) => Vec::new(),
    }
}

impl Todo {
    fn new(id: u64, header: String) -> Todo {
        Todo {
            id,
            header,
            is_done: false,
        }
    }
}

fn main() {
    println!("Program starts:");
    println!("q, quits,");
    println!("1, list todos,");
    println!("2, add todo,");
    println!("3, rm todo,");
    println!("4, mark todo done");

    // let mut todos: Vec<Todo> = Vec::new();
    let mut todos = load();
    let mut log = load_log();

    loop {
        let command = match main_menu() {
            'q' => break,
            '1' => Command::List,
            '2' => Command::Add,
            '3' => Command::Rm,
            '4' => Command::Done,
            _ => {
                println!("Unknown choice, try again.");
                continue;
            }
        };
        match command {
            Command::Add => {
                println!("Title?");
                let header = read_line();

                let id = log.len() as u64 + 1;
                let event = Event {
                    id,
                    kind: Kind::Added,
                    todo_id: id,
                    header: header.clone(),
                };
                log_event(&event);
                log.push(event);

                todos.push(Todo::new(id, header));
                save(&todos);
                println!("Added.");
            }
            Command::List => {
                for todo in &todos {
                    println!(
                        "{}. [{}] {}",
                        todo.id,
                        if todo.is_done { "x" } else { " " },
                        todo.header
                    );
                }
            }
            Command::Rm => {
                println!("Which id?");
                let todo_id: u64 = match read_line().parse() {
                    Ok(n) => n,
                    Err(_) => {
                        println!("That is not a number.");
                        continue;
                    }
                };

                match todos.iter().position(|t| t.id == todo_id) {
                    Some(index) => {
                        let event = Event {
                            id: log.len() as u64 + 1,
                            kind: Kind::Removed,
                            todo_id,
                            header: String::new(),
                        };
                        log_event(&event);
                        log.push(event);

                        todos.remove(index);
                        save(&todos);
                        println!("Removed.");
                    }
                    None => println!("No todo with that id."),
                }
            }
            Command::Done => println!("(done: not written yet)"),
        }
    }
}
