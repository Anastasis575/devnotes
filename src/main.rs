use std::{env, fs};
use std::alloc::Layout;
use std::io::stdout;
use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use crossterm::{event, ExecutableCommand};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint::Fill;
use ratatui::prelude::*;
use ratatui::style::Stylize;
use ratatui::Terminal;
use uuid::Uuid;

use devnotes::{Note, Project};

use crate::lib::{NoteRepository, ProjectRepository};
use crate::sqlite::SqliteRepository;

pub mod lib;
pub mod sqlite;

/// Simple program to add dev notes
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Varargs {
    ///The command to be run
    #[command(subcommand)]
    mode: CommandMode,
}

#[derive(Subcommand, Debug, Clone)]
enum CommandMode {
    /// Use a project to add the notes to
    Use {
        /// The project name to be appended
        project: String
    },
    ///Add Dev note to selected project
    Add {
        ///Name of the name
        name: Option<String>,

        /// Optional: Date of note
        date: Option<String>,
    },
    ///Delete note from project
    Delete {
        /// Guid prefix of the note to delete
        id: String
    },
    /// List notes for current project
    List,
    /// List selectable Projects
    Projects,
    /// Edit note
    Edit {
        /// id of the note
        guid: String
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Varargs::parse();
    let exe_path = env::current_exe()?.join("..");
    let selected = (&exe_path).join("selected.txt").to_owned();
    let selected_proj = if !Path::new(&selected).exists() {
        fs::write(&selected, "")?;
        ""
    } else {
        &fs::read_to_string(selected)?
    };
    let repo = SqliteRepository::default().await?;
    println!("Project: {}", if !selected_proj.is_empty() { selected_proj } else { "Not Selected" });
    if let CommandMode::Use = args.mode {} else if let CommandMode::Projects = args.mode {} else {
        if selected_proj.is_empty() {
            return Err(anyhow!("No project selected please run with the \"use <proj_name>\" command first"));
        }
    }
    match args.mode {
        CommandMode::Use { project } => {
            if let Err(_) = repo.list_project(match_name(&project)).await {
                repo.insert_project(Project::new(Uuid::new_v4(), project.to_string(), Utc::now().naive_utc())).await?
            }
            fs::write(&selected, project)?;
        }
        CommandMode::Add { name, date } => {
            if selected_proj.is_empty() {
                return Err(anyhow!("No project selected please run with the \"use <proj_name>\" command first"));
            }
            let selected_proj_uuid = repo.list_project(match_name(&selected_proj.to_string())).await?.first().unwrap().guid();

            let date = match date {
                None => Utc::now().naive_utc(),
                Some(d) => chrono::naive::NaiveDateTime::parse_from_str(&d, "%Y-%m-%d %H:%M:%S")?
            };
            let text = edit(&name, date, None)?;
            repo.insert_note(Note::new(Uuid::new_v4().to_string(), selected_proj_uuid, name.ok_or_else(|| Some("")).unwrap(), text, date))
        }
        CommandMode::Delete { id } => {
            repo.remove_note(id)
        }
        CommandMode::List => {
            let selected_proj_uuid = repo.list_project(match_name(&selected_proj.to_string())).await?.first().unwrap().guid();
            println!("Notes for {}", &selected_proj);
            let notes = repo.list_note(match_project_id(selected_proj_uuid)).await?;
            for note in notes {
                println!("{}", note.guid());
                println!("{}{}{}", empty_or_value(note.name(), note.name()), empty_or_value(note.name(), "|".to_string()), note.date().to_string())
            }
        }
        CommandMode::Projects => {
            let list = repo.list_project(None).await?;
            for x in list {
                println!("{}", x.name())
            }
        }
        CommandMode::Edit { guid } => {
            if selected_proj.is_empty() {
                return Err(anyhow!("No project selected please run with the \"use <proj_name>\" command first"));
            }
            let note = repo.get_note(guid).await?;
            let text = edit(note.name(), note.ts(), note.content())?;

            repo.update_note(note.guid(), text).await?
        }
    }
    Ok(())
}

fn match_name(name: &String) -> impl Fn(&Project) -> bool {
    |it| it.name() == name
}

fn match_project_id(proj_id: &String) -> impl Fn(&Note) -> bool {
    |it| it.project_id() == proj_id
}

fn empty_or_value(text: String, value: String) -> String {
    if text.is_empty() { "".to_string() } else { value }
}

fn edit(name: &Option<String>, date: chrono::NaiveDateTime, text: Option<String>) -> Result<String> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    let textarea = tui_textarea::TextArea::from(text.or_else(|| Some("".to_string())).unwrap().lines());
    'main_loop: loop {
        terminal.draw(|frame| {
            let layout = Layout::default().direction(Direction::Vertical).constraints(
                vec![Constraint::Length(1), Constraint::Min(1)]
            ).split(frame.size());
            let title = Layout::default().direction(Direction::Horizontal).constraints(
                vec![Constraint::Percentage(25), Fill(1)]
            ).split(layout[0]);
            if let Some(n) = &name {
                frame.render_widget(format!("{}|", n), title[0]);
            }
            frame.render_widget(format!("{}", date.to_string()), title[1]);
            frame.render_widget(textarea.widget(), layout[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                if key.code == KeyCode::Esc || (key.kind == KeyEventKind::Press && key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('q')) {
                    break 'main_loop;
                }
                (&textarea).input(key)
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(textarea.lines().join("\n"))
}