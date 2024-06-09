use std::{env, fs};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use uuid::Uuid;

use commands::*;

use crate::backend::{Note, NoteRepository, Project, ProjectRepository};
use crate::config::Config;
use crate::sqlite::SqliteRepository;

pub mod backend;
pub mod sqlite;
mod config;
mod commands;

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
    #[command(name = "rm")]
    Delete {
        /// Guid prefix of the note to delete
        id: String
    },
    /// List notes for current project
    #[command(name = "ls")]
    List {
        #[arg(name = "g", short, long, action = clap::ArgAction::SetTrue)]
        no_guid: bool
    },
    /// List selectable Projects
    Projects,
    /// View note
    View {
        /// id of the note
        guid: String,
        #[arg(name = "g", short, long, action = clap::ArgAction::SetTrue)]
        no_guid: bool,
    },
    /// Edit note
    Edit {
        /// id of the note
        guid: String
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Varargs::parse();
    let binding = env::current_exe()?;
    let exe_path = binding.parent().ok_or(anyhow!("bitch"))?;
    let config = config::get_or_create_config(&exe_path).expect("Config did not exist...Default Created");
    let selected = (&exe_path).join("selected.txt").to_owned();
    let selected_proj = if !Path::new(&selected).exists() {
        fs::write(&selected, "")?;
        "".to_string()
    } else {
        fs::read_to_string(&selected)?
    };
    let mut repo = SqliteRepository::default().await?;
    if let CommandMode::Use { .. } = args.mode {} else if let CommandMode::Projects = args.mode {} else {
        if selected_proj.is_empty() {
            return Err(anyhow!("No project selected please run with the \"use <proj_name>\" command first"));
        }
    }
    match args.mode {
        CommandMode::Use { project } => {
            println!("Project: {}", if !selected_proj.is_empty() { selected_proj.as_str() } else { "Not Selected" });

            if let Ok(projs) = repo.list_project_with_filter(match_name(&project)).await {
                if projs.is_empty() {
                    repo.insert_project(Project::new(Uuid::new_v4().to_string(), project.to_string(), Utc::now().naive_utc())).await?
                }
            }
            fs::write(&selected, &project)?;
            println!("Using Project: {project}")
        }
        CommandMode::Add { name, date } => {
            println!("Project: {}", if !selected_proj.is_empty() { selected_proj.as_str() } else { "Not Selected" });
            let default_name = config.default_name().to_owned();
            let final_name = name.or_else(|| default_name);

            if selected_proj.is_empty() {
                return Err(anyhow!("No project selected please run with the \"use <proj_name>\" command first"));
            }

            let selected_projjj = repo.list_project_with_filter(match_name(&selected_proj.to_string())).await?.first().unwrap().to_owned();

            let date = match date {
                None => Utc::now().naive_utc(),
                Some(d) => chrono::naive::NaiveDateTime::parse_from_str(&d, "%Y-%m-%d %H:%M:%S")?
            };
            let editor = create_editor(&config, &exe_path);
            let text = editor.edit(final_name.clone(), date, None)?;
            repo.insert_note(Note::new(Uuid::new_v4().to_string(), selected_projjj.guid().to_owned(), final_name.clone().or_else(|| Some("".to_string())).unwrap(), text, date)).await?;
        }
        CommandMode::Delete { id } => {
            let notes = repo.list_note_with_filter(match_guid_prefix(&id)).await?;
            check_guid_prefix_match(&notes)?;
            let id = notes.first().unwrap().guid().to_string();
            repo.remove_note(id).await?;
        }
        CommandMode::List { no_guid } => {
            println!("Project: {}", if !selected_proj.is_empty() { selected_proj.as_str() } else { "Not Selected" });

            let selected_projj = repo.list_project_with_filter(&match_name(&selected_proj.to_string())).await?.first().unwrap().to_owned();
            println!("Notes for {}", &selected_proj);
            let notes = repo.list_note_with_filter(match_project_id(selected_projj.guid())).await?;
            for note in notes {
                println!("{}", note.get_print(config.include_time().or_else(|| Some(false)).unwrap(), no_guid))
            }
        }
        CommandMode::Projects => {
            let list = repo.list_project().await?;
            for x in list {
                println!("{}", x.name())
            }
        }
        CommandMode::Edit { guid } => {
            println!("Project: {}", if !selected_proj.is_empty() { selected_proj.as_str() } else { "Not Selected" });

            if selected_proj.is_empty() {
                return Err(anyhow!("No project selected please run with the \"use <proj_name>\" command first"));
            }
            let notes = repo.list_note_with_filter(match_guid_prefix(&guid)).await?;
            check_guid_prefix_match(&notes)?;
            let note = notes.first().unwrap().to_owned();
            let editor = create_editor(&config, &exe_path);
            let text = editor.edit(Some(note.name().to_string()), note.ts(), Some(note.content().to_string()))?;

            repo.update_note(note.guid().to_string(), text).await?
        }
        CommandMode::View { guid, no_guid } => {
            println!("Project: {}", if !selected_proj.is_empty() { selected_proj.as_str() } else { "Not Selected" });
            let notes = repo.list_note_with_filter(match_guid_prefix(&guid)).await?;
            check_guid_prefix_match(&notes)?;
            let note = notes.first().unwrap().to_owned();
            println!("{}", note.get_print(config.include_time().or_else(|| Some(false)).unwrap(), no_guid));
        }
    }
    Ok(())
}

fn create_editor<K: AsRef<OsStr> + ?Sized>(config: &Config, exe_path: &K) -> Box<dyn Editor> {
    match config.edit_app() {
        None => Box::new(InternalEditor {}),
        Some(editor_command) => Box::new(ExternalEditor::new(editor_command.to_string(), PathBuf::from(exe_path)))
    }
}
