use std::fs;
use std::io;
use std::io::Write;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

enum Command {
    List,
    Add,
    Rm,
    Done,
    PageList,
    PageAdd,
    PageView,
    PageEdit,
    Find,
}

#[derive(Clone, Serialize, Deserialize)]
struct Todo {
    id: u64,
    header: String,
    is_done: bool,
    // date added,
}

#[derive(Clone, Serialize, Deserialize)]
struct Page {
    id: u64,
    title: String,
    text: String,
}

#[derive(Clone, Serialize, Deserialize)]
enum Kind {
    Added,
    Removed,
    Done,
}

#[derive(Clone, Serialize, Deserialize)]
struct Event {
    id: u64,       // this event's own id, unique forever
    kind: Kind,    // what happened
    todo_id: u64,  // which todo it happened to
    header: String // the title (only meaningful for Added)
}

struct Store<T: Entry> {
    items: Vec<T>,
}

impl<T: Entry> Store<T> {
    fn load() -> Self {
        let items = match fs::read_to_string(T::FILE) {
            Ok(text) => serde_json::from_str(&text).expect("corrupted file"),
            Err(_) => Vec::new(),
        };
        Store { items }
    }
    fn save(&self) {
        let text = serde_json::to_string_pretty(&self.items).expect("serialize failed");
        fs::write(T::FILE, text).expect("write failed");
    }
    fn next_id(&self) -> u64 {
        self.items.iter().map(|i| i.id()).max().unwrap_or(0) + 1
    }
    fn get(&self, id: u64) -> Option<&T> {
        self.items.iter().find(|i| i.id() == id)
    }
    fn get_mut(&mut self, id: u64) -> Option<&mut T> {
        self.items.iter_mut().find(|i| i.id() == id)
    }
    fn remove(&mut self, id: u64) -> bool {
        let before = self.items.len();
        self.items.retain(|i| i.id() != id);
        self.items.len() != before
    }
}

struct SearchHit { 
    score: i32, 
    label: String 
}

trait Module {
    fn name(&self) -> &str;                          // "todo", "dict"
    fn commands(&self) -> Vec<(&str, &str)>;         // ("add", "add a todo")
    fn run(&mut self, command: &str);
    fn search(&self, query: &str) -> Vec<SearchHit>; // fuzzy finder asks each part
}

fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    let query: Vec<char> = query.to_lowercase().chars().collect();
    let target: Vec<char> = target.to_lowercase().chars().collect();

    let mut score = 0;
    let mut qi = 0;
    let mut last_match: Option<usize> = None;

    for (ti, &c) in target.iter().enumerate() {
        if qi < query.len() && c == query[qi] {
            score += 1;
            // bonus: matched right after the previous matched letter
            if let Some(last) = last_match {
                if last + 1 == ti {
                    score += 5;
                }
            }
            // bonus: matched at the start of a word
            if ti == 0 || target[ti - 1] == ' ' {
                score += 3;
            }
            last_match = Some(ti);
            qi += 1;
        }
    }

    if qi == query.len() { Some(score) } else { None }
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

fn save_pages(pages: &Vec<Page>) {
    let text = serde_json::to_string_pretty(pages).expect("Failed to turn pages into text");
    fs::write("pages.json", text).expect("Failed to write pages.json");
}

fn load_pages() -> Vec<Page> {
    match fs::read_to_string("pages.json") {
        Ok(text) => serde_json::from_str(&text).expect("pages.json is corrupted"),
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

trait Entry: Serialize + DeserializeOwned + Clone {
    const FILE: &'static str;   // "todos.json", "pages.json"
    fn id(&self) -> u64;
    fn title(&self) -> &str;
    fn body(&self) -> &str { "" }   // default: nothing extra to search
}

impl Todo {
    fn new(id: u64, header: String) -> Todo {
        Todo { id, header, is_done: false }
    }
}

fn main() {
    println!("Program starts:");
    println!("q, quits,");
    println!("1, list todos,");
    println!("2, add todo,");
    println!("3, rm todo,");
    println!("4, mark todo done");
    println!("5, list dictionary pages,");
    println!("6, add dictionary page,");
    println!("7, view dictionary page,");
    println!("8, edit dictionary page");
    println!("9, search everything");


    // let mut todos: Vec<Todo> = Vec::new();
    let mut pages = load_pages();
    let mut todos = load();
    let mut log = load_log();

    loop {
        let command = match main_menu() {
            'q' => break,
            '1' => Command::List,
            '2' => Command::Add,
            '3' => Command::Rm,
            '4' => Command::Done,
            '5' => Command::PageList,
            '6' => Command::PageAdd,
            '7' => Command::PageView,
            '8' => Command::PageEdit,
            '9' => Command::Find,
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
                let event = Event { id, kind: Kind::Added, todo_id: id, header: header.clone() };
                log_event(&event);
                log.push(event);
            
                todos.push(Todo::new(id, header));
                save(&todos);
                println!("Added.");
            }
            Command::List => {
                for todo in &todos {
                    println!("{}. [{}] {}", todo.id, if todo.is_done { "x" } else { " " }, todo.header);
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
            Command::PageList => {
                for page in &pages {
                    println!("{}. {}", page.id, page.title);
                }
            }
            Command::PageAdd => {
                println!("Title?");
                let title = read_line();
                println!("Text?");
                let text = read_line();
            
                let id = pages.iter().map(|p| p.id).max().unwrap_or(0) + 1;
                pages.push(Page { id, title, text });
                save_pages(&pages);
                println!("Page added.");
            }
            Command::PageView => {
                println!("Which id?");
                let id: u64 = match read_line().parse() {
                    Ok(n) => n,
                    Err(_) => {
                        println!("That is not a number.");
                        continue;
                    }
                };
                match pages.iter().find(|p| p.id == id) {
                    Some(page) => println!("# {}\n{}", page.title, page.text),
                    None => println!("No page with that id."),
                }
            }
            Command::PageEdit => {
                println!("Which id?");
                let id: u64 = match read_line().parse() {
                    Ok(n) => n,
                    Err(_) => {
                        println!("That is not a number.");
                        continue;
                    }
                };
                match pages.iter_mut().find(|p| p.id == id) {
                    Some(page) => {
                        println!("Current text: {}", page.text);
                        println!("New text? (empty keeps the old one)");
                        let new_text = read_line();
                        if !new_text.is_empty() {
                            page.text = new_text;
                            save_pages(&pages);
                            println!("Page updated.");
                        }
                    }
                    None => println!("No page with that id."),
                }
            }Command::PageList => {
                for page in &pages {
                    println!("{}. {}", page.id, page.title);
                }
            }
            Command::PageAdd => {
                println!("Title?");
                let title = read_line();
                println!("Text?");
                let text = read_line();
            
                let id = pages.iter().map(|p| p.id).max().unwrap_or(0) + 1;
                pages.push(Page { id, title, text });
                save_pages(&pages);
                println!("Page added.");
            }
            Command::PageView => {
                println!("Which id?");
                let id: u64 = match read_line().parse() {
                    Ok(n) => n,
                    Err(_) => {
                        println!("That is not a number.");
                        continue;
                    }
                };
                match pages.iter().find(|p| p.id == id) {
                    Some(page) => println!("# {}\n{}", page.title, page.text),
                    None => println!("No page with that id."),
                }
            }
            Command::PageEdit => {
                println!("Which id?");
                let id: u64 = match read_line().parse() {
                    Ok(n) => n,
                    Err(_) => {
                        println!("That is not a number.");
                        continue;
                    }
                };
                match pages.iter_mut().find(|p| p.id == id) {
                    Some(page) => {
                        println!("Current text: {}", page.text);
                        println!("New text? (empty keeps the old one)");
                        let new_text = read_line();
                        if !new_text.is_empty() {
                            page.text = new_text;
                            save_pages(&pages);
                            println!("Page updated.");
                        }
                    }
                    None => println!("No page with that id."),
                }
            }
            Command::Find => {
                println!("Search?");
                let query = read_line();
                if query.is_empty() {
                    println!("Empty search.");
                    continue;
                }
            
                let mut results: Vec<(i32, String)> = Vec::new();
            
                for todo in &todos {
                    if let Some(score) = fuzzy_score(&query, &todo.header) {
                        results.push((score, format!("todo {}. {}", todo.id, todo.header)));
                    }
                }
            
                for page in &pages {
                    // a title match counts double compared to a match in the body text
                    let title_score = fuzzy_score(&query, &page.title).map(|s| s * 2);
                    let text_score = fuzzy_score(&query, &page.text);
                    if let Some(score) = title_score.max(text_score) {
                        results.push((score, format!("page {}. {}", page.id, page.title)));
                    }
                }
            
                results.sort_by(|a, b| b.0.cmp(&a.0));
            
                if results.is_empty() {
                    println!("Nothing found.");
                }
                for (score, line) in results {
                    println!("[{}] {}", score, line);
                }
            }
        }
    }
}

