pub mod command_manager;
pub mod navigation;
pub mod todo_config;

use std::{
    env,
    io::{stdout, Stdout, Write},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute, queue,
    style::{Color, Print, SetForegroundColor},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use time::{format_description, OffsetDateTime};
use time_humanize::HumanTime;
use todo_config::Todo;

use crate::{
    navigation::{Cursor, HierarchyItemEnum, HierarchyItemEnumMut, PositionHierarchy},
    todo_config::{num_to_str, Group, TodoConfig},
};

fn format_hierarchy(context: &TodoConfig, stdout: &mut Stdout) {
    let mut out = stdout;
    for group in context.groups.iter() {
        out = group.traverse(
            out,
            |g, d, v| {
                queue!(
                    v,
                    Print("  ".repeat(d)),
                    Print("["),
                    Print(if g.open {
                        '*'
                    } else {
                        num_to_str(g.todo_count())
                    }),
                    Print("] "),
                    Print(&g.name),
                    Print("\r\n")
                )
                .ok();
                (g.open, v)
            },
            |t, d, v| {
                let format_time = format_description::parse("[year]-[month]-[day] [hour]:[minute]")
                    .expect("Format to parse.");

                if t.done_time.is_some() {
                    queue!(v, SetForegroundColor(Color::DarkGrey)).ok();
                } else if let Some(due) = t.due {
                    if let Ok(now) = OffsetDateTime::now_local() {
                        if now > due {
                            queue!(v, SetForegroundColor(Color::Red)).ok();
                        } else if (due - now).whole_hours() < 24 {
                            queue!(v, SetForegroundColor(Color::Yellow)).ok();
                        }
                    }
                }

                queue!(
                    v,
                    Print("  ".repeat(d)),
                    Print("["),
                    Print(if t.done_time.is_some() { "*" } else { " " }),
                    Print("] "),
                    Print(&t.name),
                )
                .ok();

                if let Some(due) = t.due {
                    if let Ok(now) = OffsetDateTime::now_local() {
                        queue!(
                            v,
                            Print(format!(
                                " ({})",
                                HumanTime::from_seconds((due - now).whole_seconds())
                            ))
                        )
                        .ok();
                    } else {
                        queue!(
                            v,
                            Print(format!(" ({})", due.format(&format_time).unwrap()))
                        )
                        .ok();
                    }
                }
                queue!(v, Print("\r\n")).ok();

                queue!(v, SetForegroundColor(Color::Reset)).ok();

                v
            },
            |_, _, v| v,
            1,
        );
    }
}

fn draw_vis(stdout: &mut Stdout, config: &TodoConfig, cursor: &Cursor) -> Result<()> {
    match cursor {
        Cursor::Hierarchy(h) => {
            queue!(
                stdout,
                Clear(crossterm::terminal::ClearType::All),
                MoveTo(0, 0),
                Print(format!("{:?}\n\r", h.indexes))
            )?;

            format_hierarchy(config, stdout);

            let cursor_y: u16 = h.vert_pos(config)?.try_into()?;

            queue!(stdout, MoveTo(0, cursor_y + 1), Print("> "))?;

            stdout.flush()?;
        }
    }

    Ok(())
}

fn prompt(stdout: &mut Stdout, prompt: &str, def: &str) -> Result<String> {
    // disable_raw_mode()?;
    execute!(stdout, Show)?;

    execute!(
        stdout,
        MoveTo(0, 0),
        Clear(ClearType::CurrentLine),
        Print(prompt)
    )?;

    let mut out = def.to_string();
    let mut cursor_pos = out.len();

    let mut done = false;

    while !done {
        execute!(
            stdout,
            MoveTo(
                prompt
                    .len()
                    .try_into()
                    .expect("Prompt should be less than 64Ki bytes."),
                0
            ),
            Clear(ClearType::UntilNewLine),
            Print(&out),
            MoveTo(
                (prompt.len() + cursor_pos)
                    .try_into()
                    .expect("Prompt should be less than 64Ki bytes."),
                0
            ),
        )?;

        let evt = read()?;
        if let Event::Key(ke) = evt {
            if let KeyCode::Char(c) = ke.code {
                out.insert(cursor_pos, c);
                cursor_pos += 1;
            } else if let KeyCode::Backspace = ke.code {
                out.remove(cursor_pos - 1);
                cursor_pos -= 1;
            } else if let KeyCode::Left = ke.code {
                cursor_pos = cursor_pos.saturating_sub(1);
            } else if let KeyCode::Right = ke.code {
                if cursor_pos < out.len() {
                    cursor_pos += 1;
                }
            } else if let KeyCode::Home = ke.code {
                cursor_pos = 0;
            } else if let KeyCode::End = ke.code {
                cursor_pos = out.len();
            } else if let KeyCode::Enter = ke.code {
                done = true;
            }
        }
    }

    execute!(stdout, Hide)?;
    // enable_raw_mode()?;

    Ok(out)
}

fn prompt_date(stdout: &mut Stdout) -> Option<OffsetDateTime> {
    if prompt(stdout, "Add a due date? (y/n) ", "").ok()? == "y" {
        let mut current = OffsetDateTime::now_local().ok()?;

        let year_input = prompt(stdout, "Year?  ", &format!("{}", current.year())).ok()?;
        if !year_input.is_empty() {
            current = current.replace_year(year_input.parse().ok()?).ok()?;
        }
        let month_input = prompt(stdout, "Month? ", &format!("{}", current.month())).ok()?;
        if !month_input.is_empty() {
            current = current.replace_month(month_input.parse().ok()?).ok()?;
        }
        let day_input = prompt(stdout, "Day? ", &format!("{}", current.day())).ok()?;
        if !day_input.is_empty() {
            current = current.replace_day(day_input.parse().ok()?).ok()?;
        }
        let hour_input = prompt(stdout, "Hour? ", &format!("{}", current.hour())).ok()?;
        if !hour_input.is_empty() {
            current = current.replace_hour(hour_input.parse().ok()?).ok()?;
        }
        let minute_input = prompt(stdout, "Minute? ", &format!("{}", current.minute())).ok()?;
        if !minute_input.is_empty() {
            current = current.replace_minute(minute_input.parse().ok()?).ok()?;
        }

        Some(current)
    } else {
        None
    }
}

fn prompt_date_in_place(
    stdout: &mut Stdout,
    mut current: OffsetDateTime,
) -> Option<OffsetDateTime> {
    let choice = prompt(stdout, "Change due date? (k/y/n) ", "").ok()?;
    match choice.as_str() {
        "y" => {
            let year_input = prompt(stdout, "Year? ", &format!("{}", current.year())).ok()?;
            if !year_input.is_empty() {
                current = current.replace_year(year_input.parse().ok()?).ok()?;
            }
            let month_input = prompt(stdout, "Month? ", &format!("{}", current.month())).ok()?;
            if !month_input.is_empty() {
                current = current.replace_month(month_input.parse().ok()?).ok()?;
            }
            let day_input = prompt(stdout, "Day? ", &format!("{}", current.day())).ok()?;
            if !day_input.is_empty() {
                current = current.replace_day(day_input.parse().ok()?).ok()?;
            }
            let hour_input = prompt(stdout, "Hour? ", &format!("{}", current.hour())).ok()?;
            if !hour_input.is_empty() {
                current = current.replace_hour(hour_input.parse().ok()?).ok()?;
            }
            let minute_input = prompt(stdout, "Minute? ", &format!("{}", current.minute())).ok()?;
            if !minute_input.is_empty() {
                current = current.replace_minute(minute_input.parse().ok()?).ok()?;
            }

            Some(current)
        }
        "n" => None,
        _ => {
            // Keep
            Some(current)
        }
    }
}

fn create_top_group(config: &mut TodoConfig, stdout: &mut Stdout) -> Result<()> {
    let name = prompt(stdout, "Enter Name for Top Group: ", "")?;

    config.groups.push(Group {
        hidden: false,
        name,
        open: false,
        todos: vec![],
        completed: vec![],
        todo_archive: vec![],
        subgroups: vec![],
        subgroup_archive: vec![],
    });

    Ok(())
}

fn activate_item(cursor: &mut Cursor, config: &mut TodoConfig) -> Result<()> {
    if match cursor {
        Cursor::Hierarchy(ref mut h) => {
            matches!(h.find_item(config)?.item, HierarchyItemEnum::Group(_))
        }
    } {
        match cursor {
            Cursor::Hierarchy(ref mut h) => {
                if let HierarchyItemEnumMut::Group(g) = h.find_item_mut(config)?.item {
                    g.open = !g.open;
                }
            }
        };
    } else if match cursor {
        Cursor::Hierarchy(ref mut h) => {
            matches!(h.find_item(config)?.item, HierarchyItemEnum::Todo(_))
        }
    } {
        match cursor {
            Cursor::Hierarchy(ref mut h) => {
                let g = h.find_group_mut(config)?;
                if h.last()? < g.subgroups.len() + g.todos.len() {
                    let mut t = g.todos.remove(h.last()? - g.subgroups.len());
                    t.done_time = OffsetDateTime::now_local().ok();
                    g.completed.push(t);
                } else if h.last()? < g.subgroups.len() + g.todos.len() + g.completed.len() {
                    let mut t = g
                        .completed
                        .remove(h.last()? - g.subgroups.len() - g.todos.len());
                    t.done_time = None;
                    g.todos.push(t);
                }
            }
        };
    }

    Ok(())
}

fn main() -> Result<()> {
    let custom_path = &env::args().nth(1);

    let config_path = &if let Some(path) = custom_path {
        PathBuf::from(path)
    } else {
        dirs::config_dir().unwrap().join("dmc/todo/config.ron")
    };

    let mut config = match TodoConfig::read_config(config_path) {
        Ok(config) => {
            println!("Config read successfully");
            config
        }
        Err(err) => match err {
            todo_config::ConfigError::NoConfigFile => {
                println!("Config not found, creating default config");
                let config = TodoConfig::default();
                config.write_config(config_path)?;
                config
            }
            todo_config::ConfigError::Io(_) => {
                return Err(anyhow!("Error loading config file."));
            }
            todo_config::ConfigError::Parse(_) => {
                return Err(anyhow!("Error parsing config file."));
            }
            _ => {
                return Err(anyhow!(
                    "Generic error loading config file this should not be possible."
                ));
            }
        },
    };

    let mut cursor = Cursor::Hierarchy(PositionHierarchy::new());
    // let mut cursor_flat = PositionFlat::new();

    enable_raw_mode()?;

    let mut stdout = stdout();

    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide)?;

    draw_vis(&mut stdout, &config, &cursor)?;

    loop {
        // Wait up to 1s for another event
        if poll(std::time::Duration::from_millis(1_000))? {
            // Fixing blanks
            if config.groups.is_empty() {
                create_top_group(&mut config, &mut stdout)?;
            }

            for group in config.groups.iter_mut() {
                group.traverse_mut::<&time::Duration>(
                    &config.archive_time,
                    |g, _d, v| {
                        for i in (0..g.completed.len()).rev() {
                            if let Some(done_time) = g.completed[i].done_time {
                                if let Ok(now) = OffsetDateTime::now_local() {
                                    if now - done_time > *v {
                                        g.todo_archive.push(g.completed.remove(i));
                                    }
                                }
                            }
                        }

                        (true, v)
                    },
                    |_t, _d, v| v,
                    |_g, _d, v| v,
                    0,
                );
            }

            let event = read()?;

            match event {
                Event::Key(ke) => {
                    if ke.code == config.keybindings.quit
                        && ke.modifiers.contains(KeyModifiers::ALT)
                    {
                        break;
                    } else if ke.code == config.keybindings.quit {
                        config.write_config(config_path)?;
                        break;
                    } else if ke.code == config.keybindings.save {
                        config.write_config(config_path)?;
                    } else if ke.code == config.keybindings.cursor_up {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => h.cursor_up(&config).ok(),
                        };
                    } else if ke.code == config.keybindings.cursor_down {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => h.cursor_down(&config).ok(),
                        };
                    } else if ke.code == config.keybindings.group_up {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => h.group_up(&config).ok(),
                        };
                    } else if ke.code == config.keybindings.group_down {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => h.group_down(&config).ok(),
                        };
                    } else if ke.code == config.keybindings.hierarchy_up {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => h.hierarchy_up(&config).ok(),
                        };
                    } else if ke.code == config.keybindings.hierarchy_down {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => h.hierarchy_down(&mut config).ok(),
                        };
                    } else if ke.code == config.keybindings.toggle_group
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Group(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                if let HierarchyItemEnumMut::Group(g) =
                                    h.find_item_mut(&mut config)?.item
                                {
                                    g.open = !g.open;
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.toggle_todo
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Todo(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                let g = h.find_group_mut(&mut config)?;
                                if h.last()? < g.subgroups.len() + g.todos.len() {
                                    let mut t = g.todos.remove(h.last()? - g.subgroups.len());
                                    t.done_time = OffsetDateTime::now_local().ok();
                                    g.completed.push(t);
                                } else if h.last()?
                                    < g.subgroups.len() + g.todos.len() + g.completed.len()
                                {
                                    let mut t = g
                                        .completed
                                        .remove(h.last()? - g.subgroups.len() - g.todos.len());
                                    t.done_time = None;
                                    g.todos.push(t);
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.archive_todo
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Todo(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                let g = h.find_group_mut(&mut config)?;
                                let t = if h.last()? < g.subgroups.len() + g.todos.len() {
                                    g.todos.remove(h.last()? - g.subgroups.len())
                                } else {
                                    g.completed
                                        .remove(h.last()? - g.subgroups.len() - g.todos.len())
                                };
                                g.todo_archive.push(t);

                                if h.last()? >= g.len() {
                                    if h.last()? > 0 {
                                        *h.last_mut()? -= 1;
                                    } else {
                                        h.hierarchy_up(&config)?;
                                    }
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.hide_group
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Group(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                if h.indexes.len() == 1 {
                                    let t = config.groups.remove(h.last()?);
                                    config.archive_groups.push(t);

                                    if h.last()? >= config.groups.len() && h.last()? > 0 {
                                        *h.last_mut()? -= 1;
                                    }
                                } else {
                                    let g = h.find_group_mut(&mut config)?;
                                    let t = g.subgroups.remove(h.last()?);
                                    g.subgroup_archive.push(t);
                                    if h.last()? >= g.len() {
                                        if h.last()? > 0 {
                                            *h.last_mut()? -= 1;
                                        } else {
                                            h.hierarchy_up(&config)?;
                                        }
                                    }
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.add_todo
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Group(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                if let HierarchyItemEnumMut::Group(g) =
                                    h.find_item_mut(&mut config)?.item
                                {
                                    let todo_name = prompt(&mut stdout, "Todo: ", "")?;
                                    g.todos.push(Todo {
                                        name: todo_name,
                                        done_time: None,
                                        due: prompt_date(&mut stdout),
                                        created: OffsetDateTime::now_local()?,
                                    });
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.edit_todo
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Todo(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                if let HierarchyItemEnumMut::Todo(t) =
                                    h.find_item_mut(&mut config)?.item
                                {
                                    let todo_name = prompt(&mut stdout, "Todo: ", &t.name)?;
                                    if !todo_name.is_empty() {
                                        t.name = todo_name;
                                    }

                                    if let Some(due) = t.due {
                                        t.due = Some(
                                            prompt_date_in_place(&mut stdout, due).unwrap_or(due),
                                        );
                                    } else {
                                        t.due = prompt_date(&mut stdout);
                                    }
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.add_group
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Group(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                if let HierarchyItemEnumMut::Group(g) =
                                    h.find_item_mut(&mut config)?.item
                                {
                                    let group_name = prompt(&mut stdout, "Group: ", "")?;
                                    g.subgroups.push(Group {
                                        name: group_name,
                                        hidden: false,
                                        open: true,
                                        todos: vec![],
                                        completed: vec![],
                                        todo_archive: vec![],
                                        subgroups: vec![],
                                        subgroup_archive: vec![],
                                    });
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.edit_group
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Group(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                if let HierarchyItemEnumMut::Group(g) =
                                    h.find_item_mut(&mut config)?.item
                                {
                                    let group_name =
                                        prompt(&mut stdout, "Group: ", &format!("{} ", &g.name))?;
                                    if !group_name.is_empty() {
                                        g.name = group_name;
                                    }
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.add_top_group {
                        let group_name = prompt(&mut stdout, "Group: ", "")?;
                        config.groups.push(Group {
                            name: group_name,
                            hidden: false,
                            open: true,
                            todos: vec![],
                            completed: vec![],
                            todo_archive: vec![],
                            subgroups: vec![],
                            subgroup_archive: vec![],
                        });
                    } else if ke.code == config.keybindings.move_group_down
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Group(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                let group = h.find_group_mut(&mut config)?;
                                if h.last()? + 1 < group.subgroups.len() {
                                    group.subgroups.swap(h.last()?, h.last()? + 1);
                                    *h.last_mut()? += 1;
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.move_group_up
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Group(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                let group = h.find_group_mut(&mut config)?;
                                if h.last()? > 0 {
                                    group.subgroups.swap(h.last()?, h.last()? - 1);
                                    *h.last_mut()? -= 1;
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.move_todo_down
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Todo(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                let group = h.find_group_mut(&mut config)?;
                                if h.last()? + 1 < group.subgroups.len() + group.todos.len()
                                    && h.last()? >= group.subgroups.len()
                                {
                                    group.todos.swap(
                                        h.last()? - group.subgroups.len(),
                                        h.last()? + 1 - group.subgroups.len(),
                                    );
                                    *h.last_mut()? += 1;
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.move_todo_up
                        && match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                matches!(h.find_item(&config)?.item, HierarchyItemEnum::Todo(_))
                            }
                        }
                    {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                let group = h.find_group_mut(&mut config)?;
                                if h.last()? > group.subgroups.len()
                                    && h.last()? < group.subgroups.len() + group.todos.len()
                                {
                                    group.todos.swap(
                                        h.last()? - group.subgroups.len(),
                                        h.last()? - 1 - group.subgroups.len(),
                                    );
                                    *h.last_mut()? -= 1;
                                }
                            }
                        }
                    } else if ke.code == config.keybindings.clean
                        && ke.modifiers.contains(KeyModifiers::ALT)
                    {
                        //cleanup
                        config.archive_groups = vec![];
                        for group in config.groups.iter_mut() {
                            group.traverse_mut(
                                (),
                                |g, _d, v| {
                                    g.todo_archive = vec![];
                                    g.subgroup_archive = vec![];

                                    (true, v)
                                },
                                |_t, _d, v| v,
                                |_g, _d, v| v,
                                0,
                            );
                        }
                    }
                }
                Event::Mouse(me) => {
                    if let MouseEventKind::Down(MouseButton::Left) = me.kind {
                        match cursor {
                            Cursor::Hierarchy(ref mut h) => {
                                h.indexes = vec![0];
                                for _ in 1..me.row {
                                    h.cursor_down(&config)?;
                                }
                                activate_item(&mut cursor, &mut config)?;
                            }
                        }
                    }
                }
                _ => {}
            }

            if config.groups.is_empty() {
                create_top_group(&mut config, &mut stdout)?;
            }

            draw_vis(&mut stdout, &config, &cursor).ok();
        }
    }

    execute!(
        stdout,
        Show,
        DisableMouseCapture,
        Clear(ClearType::All),
        LeaveAlternateScreen
    )?;

    disable_raw_mode()?;

    Ok(())
}
