use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use sqlx::FromRow;

use crate::{config::Config, empty_or_value};

#[async_trait]
pub trait Initable {
    fn initialized(&self) -> bool;
    async fn init(&mut self) -> Result<()>;
}

#[async_trait]
pub trait ProjectRepository: Initable {
    async fn create_proj_table(&mut self) -> Result<()>;
    async fn insert_project(&mut self, entity: Project) -> Result<()>;
    async fn remove_project(&mut self, key: String) -> Result<u32>;
    async fn get_project(&mut self, key: String) -> Result<Project>;
    async fn list_project(&mut self) -> Result<Vec<Project>>;
    async fn list_project_with_filter<T: Fn(&Project) -> bool + Send + Sync>(
        &mut self,
        pred: T,
    ) -> Result<Vec<Project>>;
}

#[async_trait]
pub trait NoteRepository: Initable {
    async fn create_note_table(&mut self) -> Result<()>;
    async fn insert_note(&mut self, entity: Note) -> Result<()>;
    async fn remove_note(&mut self, key: String) -> Result<u32>;
    async fn get_note(&mut self, key: String) -> Result<Note>;
    async fn list_note(&mut self) -> Result<Vec<Note>>;
    async fn list_note_with_filter<T: Fn(&Note) -> bool + Send + Sync>(
        &mut self,
        pred: T,
    ) -> Result<Vec<Note>>;
    async fn update_note(&mut self, key: String, text: String, project_id: String) -> Result<u64>;
}

#[derive(Debug, Clone, FromRow)]
pub struct Project {
    id: String,
    name: String,
    ts: chrono::NaiveDateTime,
}

impl Project {
    pub fn guid(&self) -> &String {
        &self.id
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn ts(&self) -> chrono::NaiveDateTime {
        self.ts
    }
    pub fn new(guid: String, name: String, ts: chrono::NaiveDateTime) -> Self {
        Self { id: guid, name, ts }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Note {
    id: String,
    project_id: String,
    name: String,
    content: String,
    ts: chrono::NaiveDateTime,
}

impl Note {
    pub(crate) fn get_print(&self, config: &Config, no_guid: bool) -> String {
        let mut builder = String::new();
        if no_guid {
            builder.push_str(format!("{}\n", self.guid()).as_str());
        }
        builder.push_str(
            format!(
                "{}{}{}\n",
                empty_or_value(self.name().to_string(), self.name().to_string()),
                if self.name().is_empty() { "" } else { "|" },
                self.ts()
                    .format(if !config.include_time() {
                        "%Y-%m-%d"
                    } else {
                        "%Y-%m-%d %H:%M:%S"
                    })
                    .to_string()
            )
            .as_str(),
        );
        builder.push_str(
            format!(
                "{}\n",
                empty_or_value(self.content().to_string(), "<EMPTY>".to_string())
            )
            .as_str(),
        );
        builder.push_str("=====================================");
        builder
    }
}

impl Note {
    pub fn guid(&self) -> &String {
        &self.id
    }
    pub fn project_id(&self) -> &String {
        &self.project_id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn content(&self) -> &str {
        &self.content
    }
    pub fn ts(&self) -> chrono::NaiveDateTime {
        self.ts
    }
    pub fn new(
        guid: String,
        project_id: String,
        name: String,
        content: String,
        ts: chrono::NaiveDateTime,
    ) -> Self {
        Self {
            id: guid,
            project_id,
            name,
            content,
            ts,
        }
    }
}
