use std::path::PathBuf;

use crossterm::event::KeyCode;
use thiserror::Error;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Todo {
    pub name: String,                      // Name of the todo
    pub done_time: Option<OffsetDateTime>, // None if not done
    pub due: Option<OffsetDateTime>,       // None if no due date specified
    pub created: OffsetDateTime,           // When the todo was created
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Group {
    pub hidden: bool, // Whether the group is hidden or not
    pub name: String, // Name of the group
    pub open: bool,   // Whether the group is open or not
    #[serde(default = "default_todos")]
    pub todos: Vec<Todo>, // Todos in the group
    #[serde(default = "default_todos")]
    pub completed: Vec<Todo>, // Todos which are marked done (shown after todos)
    #[serde(default = "default_todos")]
    pub todo_archive: Vec<Todo>, // Todos which are marked done for 24h
    #[serde(default = "default_groups")]
    pub subgroups: Vec<Group>, // Subgroups
    #[serde(default = "default_groups")]
    pub subgroup_archive: Vec<Group>, // Archive of subgroups
}

impl Group {
    pub fn traverse<T>(
        &self,
        value: T,
        pre_handle: fn(&Group, usize, T) -> (bool, T),
        todo_handle: fn(&Todo, usize, T) -> T,
        after_handle: fn(&Group, usize, T) -> T,
        depth: usize,
    ) -> T {
        let (use_inner, mut value) = pre_handle(self, depth, value);
        if use_inner {
            for subgroup in self.subgroups.iter() {
                value = subgroup.traverse(value, pre_handle, todo_handle, after_handle, depth + 1);
            }
            for todo in self.todos.iter() {
                value = todo_handle(todo, depth + 1, value);
            }
            for todo in self.completed.iter() {
                value = todo_handle(todo, depth + 1, value);
            }
        }
        after_handle(self, depth, value)
    }

    pub fn traverse_mut<T>(
        &mut self,
        value: T,
        pre_handle: fn(&mut Group, usize, T) -> (bool, T),
        todo_handle: fn(&mut Todo, usize, T) -> T,
        after_handle: fn(&mut Group, usize, T) -> T,
        depth: usize,
    ) -> T {
        let (use_inner, mut value) = pre_handle(self, depth, value);
        if use_inner {
            for subgroup in self.subgroups.iter_mut() {
                value =
                    subgroup.traverse_mut(value, pre_handle, todo_handle, after_handle, depth + 1);
            }
            for todo in self.todos.iter_mut() {
                value = todo_handle(todo, depth + 1, value);
            }
            for todo in self.completed.iter_mut() {
                value = todo_handle(todo, depth + 1, value);
            }
        }
        after_handle(self, depth, value)
    }

    pub fn todo_count(&self) -> usize {
        self.traverse(
            0,
            |_, _, v| (true, v),
            |t, _, v| if t.done_time.is_none() { v + 1 } else { v },
            |_, _, v| v,
            0,
        )
    }

    pub fn is_empty(&self) -> bool {
        self.subgroups.is_empty() && self.todos.is_empty() && self.completed.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.subgroups.len() + self.todos.len() + self.completed.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TodoConfig {
    #[serde(default = "default_groups")]
    pub groups: Vec<Group>,
    #[serde(default = "default_groups")]
    pub archive_groups: Vec<Group>,
    pub archive_time: Duration, // How long a todo should be kept before being archived
    pub keybindings: Keybindings,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Keybindings {
    #[serde(default = "default_add_todo")]
    pub add_todo: KeyCode, //
    #[serde(default = "default_add_group")]
    pub add_group: KeyCode, //
    #[serde(default = "default_add_top_group")]
    pub add_top_group: KeyCode, //
    #[serde(default = "default_toggle_group")]
    pub toggle_group: KeyCode, //
    #[serde(default = "default_toggle_todo")]
    pub toggle_todo: KeyCode, //
    #[serde(default = "default_archive_todo")]
    pub archive_todo: KeyCode, //
    #[serde(default = "default_hide_group")]
    pub hide_group: KeyCode, //
    #[serde(default = "default_edit_todo")]
    pub edit_todo: KeyCode, //
    #[serde(default = "default_edit_group")]
    pub edit_group: KeyCode, //
    #[serde(default = "default_move_todo_up")]
    pub move_todo_up: KeyCode,
    #[serde(default = "default_move_todo_down")]
    pub move_todo_down: KeyCode,
    #[serde(default = "default_move_group_up")]
    pub move_group_up: KeyCode,
    #[serde(default = "default_move_group_down")]
    pub move_group_down: KeyCode,
    #[serde(default = "default_cursor_up")]
    pub cursor_up: KeyCode, //
    #[serde(default = "default_cursor_down")]
    pub cursor_down: KeyCode, //
    #[serde(default = "default_group_up")]
    pub group_up: KeyCode, //
    #[serde(default = "default_group_down")]
    pub group_down: KeyCode, //
    #[serde(default = "default_hierarchy_up")]
    pub hierarchy_up: KeyCode, //
    #[serde(default = "default_hierarchy_down")]
    pub hierarchy_down: KeyCode, //
    #[serde(default = "default_quit")]
    pub quit: KeyCode, //
    #[serde(default = "default_save")]
    pub save: KeyCode, //
    #[serde(default = "default_clean")]
    pub clean: KeyCode,
    #[serde(default = "default_help")]
    pub help: KeyCode,
}

fn default_add_todo() -> KeyCode {
    KeyCode::Char('a')
}
fn default_add_group() -> KeyCode {
    KeyCode::Char('g')
}
fn default_add_top_group() -> KeyCode {
    KeyCode::Char('n')
}
fn default_toggle_group() -> KeyCode {
    KeyCode::Char(' ')
}
fn default_toggle_todo() -> KeyCode {
    KeyCode::Char(' ')
}
fn default_archive_todo() -> KeyCode {
    KeyCode::Char('d')
}
fn default_hide_group() -> KeyCode {
    KeyCode::Char('x')
}
fn default_edit_todo() -> KeyCode {
    KeyCode::Char('e')
}
fn default_edit_group() -> KeyCode {
    KeyCode::Char('e')
}
fn default_move_todo_up() -> KeyCode {
    KeyCode::Char('i')
}
fn default_move_todo_down() -> KeyCode {
    KeyCode::Char('k')
}
fn default_move_group_up() -> KeyCode {
    KeyCode::Char('i')
}
fn default_move_group_down() -> KeyCode {
    KeyCode::Char('k')
}
fn default_cursor_up() -> KeyCode {
    KeyCode::Up
}
fn default_cursor_down() -> KeyCode {
    KeyCode::Down
}
fn default_group_up() -> KeyCode {
    KeyCode::PageUp
}
fn default_group_down() -> KeyCode {
    KeyCode::PageDown
}
fn default_hierarchy_up() -> KeyCode {
    KeyCode::Char('[')
}
fn default_hierarchy_down() -> KeyCode {
    KeyCode::Char(']')
}
fn default_quit() -> KeyCode {
    KeyCode::Char('q')
}
fn default_save() -> KeyCode {
    KeyCode::Char('s')
}
fn default_clean() -> KeyCode {
    KeyCode::Char('o')
}
fn default_help() -> KeyCode {
    KeyCode::Char('h')
}

fn default_groups() -> Vec<Group> {
    vec![]
}
fn default_todos() -> Vec<Todo> {
    vec![]
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            add_todo: default_add_todo(), //
            add_group: default_add_group(),
            add_top_group: default_add_top_group(),
            toggle_group: default_toggle_group(), //
            toggle_todo: default_toggle_todo(),   //
            archive_todo: default_archive_todo(), //
            hide_group: default_hide_group(),     //
            edit_todo: default_edit_todo(),       //
            edit_group: default_edit_group(),
            move_todo_up: default_move_todo_up(),
            move_todo_down: default_move_todo_down(),
            move_group_up: default_move_group_up(),
            move_group_down: default_move_group_down(),
            cursor_up: default_cursor_up(),           //
            cursor_down: default_cursor_down(),       //
            group_up: default_group_up(),             //
            group_down: default_group_down(),         //
            hierarchy_down: default_hierarchy_down(), //
            hierarchy_up: default_hierarchy_up(),     //
            quit: default_quit(),                     //
            save: default_save(),                     //
            help: default_help(),
            clean: default_clean(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error")]
    Io(#[from] std::io::Error), // Should fail to avoid data loss
    #[error("RON parsing error")]
    Parse(#[from] ron::error::SpannedError), // Should fail the program to avoid data loss
    #[error("RON error")]
    Stringify(#[from] ron::error::Error), // Should warn the user about possible data loss
    #[error("No config file found")]
    NoConfigFile, // Should generate a new config file
}

impl TodoConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_config(config_path: &PathBuf) -> Result<Self, ConfigError> {
        println!("Config path: {:?}", config_path);
        if config_path.exists() {
            Ok(ron::from_str(&std::fs::read_to_string(config_path)?)?)
        } else {
            Err(ConfigError::NoConfigFile)
        }
    }

    pub fn write_config(&self, config_path: &PathBuf) -> Result<(), ConfigError> {
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        std::fs::write(
            config_path,
            ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?,
        )?;
        Ok(())
    }
}

impl Default for TodoConfig {
    fn default() -> Self {
        Self {
            groups: vec![Group {
                hidden: false,
                name: "Welcome".to_string(),
                open: true,
                todos: vec![
                    Todo {
                        name: "Welcome to todo!".to_string(),
                        done_time: None,
                        due: None,
                        created: OffsetDateTime::now_local()
                            .unwrap_or_else(|_| OffsetDateTime::now_utc()),
                    },
                    Todo {
                        name: "Press 'h' for help".to_string(),
                        done_time: None,
                        due: None,
                        created: OffsetDateTime::now_local()
                            .unwrap_or_else(|_| OffsetDateTime::now_utc()),
                    },
                ],
                completed: vec![],
                todo_archive: vec![],
                subgroups: vec![
                    Group {
                        hidden: false,
                        name: "Subgroup".to_string(),
                        open: true,
                        todos: vec![Todo {
                            name: "This is a subgroup".to_string(),
                            done_time: None,
                            due: None,
                            created: OffsetDateTime::now_local()
                                .unwrap_or_else(|_| OffsetDateTime::now_utc()),
                        }],
                        completed: vec![],
                        todo_archive: vec![],
                        subgroups: vec![],
                        subgroup_archive: vec![],
                    },
                    Group {
                        hidden: false,
                        name: "Another subgroup".to_string(),
                        open: true,
                        todos: vec![Todo {
                            name: "This is another subgroup".to_string(),
                            done_time: None,
                            due: None,
                            created: OffsetDateTime::now_local()
                                .unwrap_or_else(|_| OffsetDateTime::now_utc()),
                        }],
                        completed: vec![],
                        todo_archive: vec![],
                        subgroups: vec![],
                        subgroup_archive: vec![],
                    },
                ],
                subgroup_archive: vec![],
            }],
            archive_groups: vec![],
            archive_time: Duration::days(1),
            keybindings: Default::default(),
        }
    }
}

pub fn num_to_str(num: usize) -> char {
    if num < 10 {
        char::from_u32(
            (('0' as usize) + num)
                .try_into()
                .expect("Number is less than 10 so it must fit into i32"),
        )
        .expect("Number is less than 10 so it must fit into char")
    } else {
        '+'
    }
}

pub fn format_hierarchy(context: &TodoConfig) -> String {
    let mut out = String::new();
    for group in context.groups.iter() {
        out = group.traverse(
            out,
            |g, d, mut v| {
                v.push_str(&format!(
                    "{}[{}] {}\r\n",
                    "  ".repeat(d),
                    if g.open {
                        '*'
                    } else {
                        num_to_str(g.todo_count())
                    },
                    g.name
                ));
                (g.open, v)
            },
            |t, d, mut v| {
                v.push_str(&format!(
                    "{}[{}] {}\r\n",
                    "  ".repeat(d),
                    if t.done_time.is_some() { "*" } else { " " },
                    t.name
                ));
                v
            },
            |_, _, v| v,
            1,
        );
    }

    out
}
