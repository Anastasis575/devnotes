use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{anyhow, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use itertools::Itertools;
use uuid::Uuid;

use commands::*;

use crate::backend::{Note, NoteRepository, Project, ProjectRepository};
use crate::config::Config;
use crate::sqlite::SqliteRepository;

pub mod backend;
mod commands;
mod config;
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
        project: String,
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
        id: String,
    },
    /// List notes for current project
    #[command(name = "ls")]
    List {
        #[arg(name = "g", short, long, action = clap::ArgAction::SetTrue)]
        no_guid: bool,
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
        guid: String,
    },
    /// Moves note with guid prefix to current project
    Move {
        /// id of the note
        guid: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Varargs::parse();
    let binding = env::current_exe()?;
    let exe_path = binding.parent().ok_or(anyhow!("bitch"))?;
    let config =
        config::get_or_create_config(&exe_path).expect("Config did not exist...Default Created");
    let selected = (&exe_path).join("selected.txt").to_owned();
    let selected_proj = if !Path::new(&selected).exists() {
        fs::write(&selected, "")?;
        "".to_string()
    } else {
        fs::read_to_string(&selected)?
    };
    let mut repo = SqliteRepository::default().await?;
    if let CommandMode::Use { .. } = args.mode {
    } else if let CommandMode::Projects = args.mode {
    } else {
        if selected_proj.is_empty() {
            return Err(anyhow!(
                "No project selected please run with the \"use <proj_name>\" command first"
            ));
        }
    }
    match args.mode {
        CommandMode::Use { project } => {
            println!(
                "Project: {}",
                if !selected_proj.is_empty() {
                    selected_proj.as_str()
                } else {
                    "Not Selected"
                }
            );

            if let Ok(projs) = repo.list_project_with_filter(match_name(&project)).await {
                if projs.is_empty() {
                    repo.insert_project(Project::new(
                        Uuid::new_v4().to_string(),
                        project.to_string(),
                        Utc::now().naive_utc(),
                    ))
                    .await?
                }
            }
            fs::write(&selected, &project)?;
            println!("Using Project: {project}")
        }
        CommandMode::Add { name, date } => {
            println!(
                "Project: {}",
                if !selected_proj.is_empty() {
                    selected_proj.as_str()
                } else {
                    "Not Selected"
                }
            );
            let default_name = config.default_name().to_owned();
            let final_name = name.or_else(|| config::string_optional(default_name));

            if selected_proj.is_empty() {
                return Err(anyhow!(
                    "No project selected please run with the \"use <proj_name>\" command first"
                ));
            }

            let selected_projjj = repo
                .list_project_with_filter(match_name(&selected_proj.to_string()))
                .await?
                .first()
                .unwrap()
                .to_owned();

            let date = match date {
                None => Utc::now().naive_utc(),
                Some(d) => chrono::naive::NaiveDateTime::parse_from_str(&d, "%Y-%m-%d %H:%M:%S")?,
            };
            let editor = create_editor(&config, &exe_path);
            let text = editor.edit(final_name.clone(), date, None)?;
            if !text.is_empty() || !&config.no_empty_adds_or_updates() {
                repo.insert_note(Note::new(
                    Uuid::new_v4().to_string(),
                    selected_projjj.guid().to_owned(),
                    final_name.clone().or_else(|| Some("".to_string())).unwrap(),
                    text,
                    date,
                ))
                .await?;
            }
        }
        CommandMode::Delete { id } => {
            let notes = repo.list_note_with_filter(match_guid_prefix(&id)).await?;
            check_guid_prefix_match(&notes)?;
            let id = notes.first().unwrap().guid().to_string();
            repo.remove_note(id).await?;
        }
        CommandMode::List { no_guid } => {
            println!(
                "Project: {}",
                if !selected_proj.is_empty() {
                    selected_proj.as_str()
                } else {
                    "Not Selected"
                }
            );

            let selected_projj = repo
                .list_project_with_filter(&match_name(&selected_proj.to_string()))
                .await?
                .first()
                .unwrap()
                .to_owned();
            println!("Notes for {}", &selected_proj);
            let notes = repo
                .list_note_with_filter(match_project_id(selected_projj.guid()))
                .await?;
            if !config.group_by_date() {
                for note in notes {
                    println!("{}", note.get_print(&config, no_guid))
                }
            } else {
                let dates = notes
                    .iter()
                    .map(|it| {
                        (
                            format!(
                                "{}{}{}",
                                empty_or_value(it.name().to_string(), it.name().to_string()),
                                if it.name().is_empty() { "" } else { "|" },
                                it.ts()
                                    .format(if !config.include_time() {
                                        "%Y-%m-%d"
                                    } else {
                                        "%Y-%m-%d %H:%M:%S"
                                    })
                                    .to_string(),
                            ),
                            format!(
                                "{}{}{}",
                                if no_guid { it.guid() } else { "" },
                                if no_guid { "\n" } else { "" },
                                it.content()
                            ),
                        )
                    })
                    .into_group_map();
                for date in dates {
                    println!("{}", date.0);
                    println!("{}", date.1.join("\n================\n"));
                }
            }
        }
        CommandMode::Projects => {
            let list = repo.list_project().await?;
            for x in list {
                println!("{}", x.name())
            }
        }
        CommandMode::Edit { guid } => {
            println!(
                "Project: {}",
                if !selected_proj.is_empty() {
                    selected_proj.as_str()
                } else {
                    "Not Selected"
                }
            );

            if selected_proj.is_empty() {
                return Err(anyhow!(
                    "No project selected please run with the \"use <proj_name>\" command first"
                ));
            }
            let notes = repo.list_note_with_filter(match_guid_prefix(&guid)).await?;
            check_guid_prefix_match(&notes)?;
            let note = notes.first().unwrap().to_owned();
            let editor = create_editor(&config, &exe_path);
            let text = editor.edit(
                Some(note.name().to_string()),
                note.ts(),
                Some(note.content().to_string()),
            )?;

            if !text.is_empty() || !&config.no_empty_adds_or_updates() {
                let count = repo
                    .update_note(note.guid().to_string(), text, note.project_id().to_owned())
                    .await?;
                if count == 0 {
                    return Err(anyhow!("Update failed"));
                }
            }
        }
        CommandMode::View { guid, no_guid } => {
            println!(
                "Project: {}",
                if !selected_proj.is_empty() {
                    selected_proj.as_str()
                } else {
                    "Not Selected"
                }
            );
            let notes = repo.list_note_with_filter(match_guid_prefix(&guid)).await?;
            check_guid_prefix_match(&notes)?;
            let note = notes.first().unwrap().to_owned();
            println!("{}", note.get_print(&config, no_guid));
        }
        CommandMode::Move { guid } => {
            println!(
                "Project: {}",
                if !selected_proj.is_empty() {
                    selected_proj.as_str()
                } else {
                    "Not Selected"
                }
            );

            if selected_proj.is_empty() {
                return Err(anyhow!(
                    "No project selected please run with the \"use <proj_name>\" command first"
                ));
            }
            let selected_projj = repo
                .list_project_with_filter(&match_name(&selected_proj.to_string()))
                .await?
                .first()
                .unwrap()
                .to_owned();
            let notes = repo.list_note_with_filter(match_guid_prefix(&guid)).await?;
            check_guid_prefix_match(&notes)?;
            let note = notes.first().unwrap();
            let count = repo
                .update_note(
                    note.guid().to_string(),
                    note.content().to_owned(),
                    selected_projj.guid().to_owned(),
                )
                .await?;
            if count == 0 {
                return Err(anyhow!("Update failed"));
            }
        }
    }
    Ok(())
}

fn create_editor<K: AsRef<OsStr> + ?Sized>(config: &Config, exe_path: &K) -> Box<dyn Editor> {
    match config::string_optional(config.edit_app().to_owned()) {
        None => Box::new(InternalEditor {}),
        Some(editor_command) if editor_command == "interna" => Box::new(InternalEditor {}),
        Some(editor_command) => Box::new(ExternalEditor::new(
            editor_command.to_string(),
            PathBuf::from(exe_path),
        )),
    }
}
