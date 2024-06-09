use std::fs;
use std::io::stdout;
use std::path::PathBuf;
use std::process::Command;

use anyhow::anyhow;
use anyhow::Result;
use crossterm::{event, ExecutableCommand};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use itertools::Itertools;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::layout::Constraint::Fill;
use ratatui::Terminal;
use uuid::Uuid;

use crate::backend::{Note, Project};

pub(crate) fn check_guid_prefix_match(notes: &Vec<Note>) -> anyhow::Result<()> {
    match notes.len() {
        x if x == 0 => Err(anyhow!("This gid does not exist in this database")),
        x if x == 1 => Ok(()),
        _ => Err(anyhow!(
            "This gid prefix holds multiple results in this database"
        )),
    }?;
    Ok(())
}

pub(crate) fn match_guid_prefix(name: &String) -> impl Fn(&Note) -> bool + '_ {
    move |it: &Note| it.guid().starts_with(name)
}

pub(crate) fn match_name(name: &String) -> impl Fn(&Project) -> bool + '_ {
    move |it: &Project| it.name() == name
}

pub(crate) fn match_project_id(proj_id: &String) -> impl Fn(&Note) -> bool + '_ {
    move |it: &Note| it.project_id() == proj_id
}

pub(crate) fn empty_or_value(text: String, value: String) -> String {
    if text.is_empty() {
        value
    } else {
        text
    }
}

pub(crate) struct InternalEditor {}

impl Editor for InternalEditor {
    fn edit(
        self: Box<Self>,
        name: Option<String>,
        date: chrono::NaiveDateTime,
        text: Option<String>,
    ) -> Result<String> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        terminal.clear()?;
        let mut textarea =
            tui_textarea::TextArea::from(text.or_else(|| Some("".to_string())).unwrap().lines());
        'main_loop: loop {
            terminal.draw(|frame| {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![Constraint::Length(1), Constraint::Min(1)])
                    .split(frame.size());
                let title = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![Constraint::Percentage(25), Fill(1)])
                    .split(layout[0]);
                if let Some(n) = &name {
                    frame.render_widget(format!("{}|", n), title[0]);
                }
                frame.render_widget(format!("{}", date.format("%Y-%m-%d %H:%M:%S").to_string()), title[1]);
                frame.render_widget(textarea.widget(), layout[1]);
            })?;

            if event::poll(std::time::Duration::from_millis(16))? {
                if let event::Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Esc
                        || (key.kind == KeyEventKind::Press
                        && key.modifiers == KeyModifiers::CONTROL
                        && key.code == KeyCode::Char('q'))
                    {
                        break 'main_loop;
                    }
                    (&mut textarea).input(key);
                }
            }
        }

        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(textarea.lines().join("\n"))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExternalEditor {
    command: String,
    exe_path: PathBuf,
}

impl ExternalEditor {
    pub fn new(command: String, exe_path: PathBuf) -> Self {
        Self { command, exe_path }
    }
    pub fn command(&self) -> &str {
        &self.command
    }
    pub fn exe_path(&self) -> &PathBuf {
        &self.exe_path
    }
}

impl Editor for ExternalEditor {
    fn edit(
        self: Box<Self>,
        name: Option<String>,
        date: chrono::NaiveDateTime,
        text: Option<String>,
    ) -> Result<String> {
        let local = Uuid::new_v4().to_string();
        let path = self.exe_path().join(format!("{}.txt", local));
        let unwrapped_name = name.or_else(|| Some("".to_string())).unwrap();
        let mut builder = format!(
            "{}{}{}",
            if unwrapped_name.is_empty() {
                "".to_string()
            } else {
                format!("{}|", unwrapped_name)
            },
            date.format("%Y-%m-%d %H:%M:%S").to_string(),
            if let Some(te) = text {
                format!("\n{}", te)
            } else {
                "".to_string()
            }
        );
        builder.push_str("\nDO NOT EDIT ABOVE THE LINE, CAUSE IT WILL NOT BE RECORDED(===)");
        builder.push_str("\n====================\n");

        fs::write(
            &path,
            builder,
        )?;
        let mut output = Command::new(self.command())
            .args([&path])
            .current_dir(self.exe_path)
            .spawn()?;

        let _ = output.wait()?;
        let output_str = fs::read_to_string(&path)?.split("\n").collect_vec()[3..].join("\n");
        fs::remove_file(&path)?;
        Ok(output_str)
    }
}

pub(crate) trait Editor {
    fn edit(
        self: Box<Self>,
        name: Option<String>,
        date: chrono::NaiveDateTime,
        text: Option<String>,
    ) -> Result<String>;
}
