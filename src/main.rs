use chrono::Local;
use ncurses::*;
use std::env;
use std::fs::File;
use std::io::Write;
use std::io::{self, BufRead};
use std::process;

const REGULAR_PAIR: i16 = 0;
const HIGHLIGHT_PAIR: i16 = 1;

type Id = usize;

#[derive(Default)]
struct Ui {
    list_current: Option<Id>,
    row: usize,
    col: usize,
}

impl Ui {
    fn begin(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }
    fn begin_list(&mut self, id: Id) {
        assert!(self.list_current.is_none(), "NESTED LISTS -> NOT ALLOWED");
        self.list_current = Some(id);
    }

    fn list_element(&mut self, label: &str, id: Id) -> bool {
        let id_current = self
            .list_current
            .expect("LIST ELEMENTS -> NOT ALLOWED TO CREATE ELEMENT OUTSIDE OF LIST");

        self.label(label, {
            if id_current == id {
                HIGHLIGHT_PAIR
            } else {
                REGULAR_PAIR
            }
        });

        false
    }

    fn end_list(&mut self) {
        self.list_current = None;
    }

    fn label(&mut self, text: &str, pair: i16) {
        mv(self.row as i32, self.col as i32);
        attron(COLOR_PAIR(pair));
        addstr(text);
        attroff(COLOR_PAIR(pair));
        self.row += 1;
    }

    fn end(&mut self) {}
}

#[derive(Debug)]
enum Status {
    Todo,
    Done,
}

impl Status {
    fn toggle(&self) -> Self {
        match self {
            Status::Todo => Status::Done,
            Status::Done => Status::Todo,
        }
    }
}

fn parse_todo(line: &str) -> Option<(Status, &str)> {
    let todo_prefix = "TODO: ";
    let done_prefix = "DONE: ";

    if line.starts_with(todo_prefix) {
        return Some((Status::Todo, &line[todo_prefix.len()..]));
    }

    if line.starts_with(done_prefix) {
        return Some((Status::Done, &line[done_prefix.len()..]));
    }

    None
}

fn list_up(list_current: &mut usize) {
    if *list_current > 0 {
        *list_current -= 1;
    }
}

fn list_down(list: &Vec<String>, list_current: &mut usize) {
    if *list_current + 1 < list.len() {
        *list_current += 1;
    }
}

fn list_transfer(
    list_dst: &mut Vec<String>,
    list_src: &mut Vec<String>,
    list_src_curr: &mut usize,
) {
    if *list_src_curr < list_src.len() {
        list_dst.push(list_src.remove(*list_src_curr));
        if *list_src_curr >= list_src.len() && list_src.len() > 0 {
            *list_src_curr = list_src.len() - 1;
        }
    }
}

fn save_state(todos: &Vec<String>, dones: &Vec<String>, file_path: &str) {
    let mut file = File::create(file_path).unwrap();
    for todo in todos.iter() {
        writeln!(file, "TODO: {}", todo).unwrap();
    }
    for done in dones.iter() {
        writeln!(file, "DONE: {}", done).unwrap();
    }
}

fn load_state(todos: &mut Vec<String>, dones: &mut Vec<String>, file_path: &str) {
    let file = File::open(file_path).unwrap();
    for (index, line) in io::BufReader::new(file).lines().enumerate() {
        match parse_todo(&line.unwrap()) {
            Some((Status::Todo, title)) => todos.push(title.to_string()),
            Some((Status::Done, title)) => dones.push(title.to_string()),
            None => {
                eprintln!(
                    "{}:{}: ERROR: item line format incorrectly",
                    file_path,
                    index + 1
                );
                process::exit(1);
            }
        }
    }
}

// TODO: undo system
// TODO: new elements to list(todo) maybe done
// TODO: keep track of dates
// DONE: persist app state (save)
// TODO: edit todos
// TODO: add priority to todos and tags?
// TODO: delete items
// TODO: only show daily todos
// TODO: save state

fn main() {
    let mut args = env::args();
    args.next().unwrap();

    let file_path = {
        match args.next() {
            Some(file_path) => file_path,
            None => {
                eprintln!("Usage: todo-rs <file-path>");
                eprintln!("ERROR: no filepath provided");
                process::exit(1);
            }
        }
    };

    let mut quit = false;
    let mut todos = Vec::<String>::new();
    let mut todo_current: usize = 0;
    let mut dones = Vec::<String>::new();
    let mut done_current: usize = 0;

    load_state(&mut todos, &mut dones, &file_path);

    initscr();
    let current_day = Local::now();
    let formatted_date = current_day.format("%d/%m/%Y");

    // disable echo and cursor
    noecho();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    start_color();
    init_pair(REGULAR_PAIR, COLOR_WHITE, COLOR_BLACK);
    init_pair(HIGHLIGHT_PAIR, COLOR_BLACK, COLOR_WHITE);

    let mut tab = Status::Todo;

    let mut ui = Ui::default();

    while !quit {
        erase();
        ui.begin(0, 0);
        {
            match tab {
                Status::Todo => {
                    ui.label(
                        format!("[TODO] DONE  {}:", formatted_date.to_string()).as_str(),
                        REGULAR_PAIR,
                    );
                    ui.label("------------------------", REGULAR_PAIR);
                    ui.begin_list(todo_current);
                    for (index, todo) in todos.iter().enumerate() {
                        ui.list_element(&format!("[ ] {}", todo), index);
                    }

                    ui.end_list();

                    if todos.len() < 1 {
                        ui.label("Everything done, enjoy the day", REGULAR_PAIR)
                    }
                }
                Status::Done => {
                    ui.label(
                        format!(" TODO [DONE] {}:", formatted_date.to_string()).as_str(),
                        REGULAR_PAIR,
                    );
                    ui.label("------------------------", REGULAR_PAIR);
                    ui.begin_list(done_current);
                    for (index, done) in dones.iter().enumerate() {
                        ui.list_element(&format!("[x] {}", done), index);
                    }
                    ui.end_list();
                }
            }
        }
        ui.end();

        refresh();

        let key = getch();

        // movement keys
        match key as u8 as char {
            'q' => quit = true,
            'k' => match tab {
                Status::Todo => list_up(&mut todo_current),
                Status::Done => list_up(&mut done_current),
            },
            'j' => match tab {
                Status::Todo => list_down(&todos, &mut todo_current),
                Status::Done => list_down(&dones, &mut done_current),
            },
            '\n' => match tab {
                Status::Todo => list_transfer(&mut dones, &mut todos, &mut todo_current),
                Status::Done => list_transfer(&mut todos, &mut dones, &mut done_current),
            },

            's' => todos.push(dones[done_current].clone()),
            'e' => {
                let mut file = File::create("TODO").unwrap();
                for todo in todos.iter() {
                    writeln!(file, "TODO: {}", todo);
                }
                for done in dones.iter() {
                    writeln!(file, "DONE: {}", done);
                }
            }
            '\t' => {
                tab = tab.toggle();
            }
            _ => {}
        }
    }
    getch();

    save_state(&todos, &dones, &file_path);
    endwin();
}
