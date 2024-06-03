use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use sqlx::FromRow;

#[async_trait]
pub trait Initable {
    fn initialized(&self) -> bool;
    async fn init(&mut self) -> Result<()>;
}

#[async_trait]
pub trait ProjectRepository: Initable {
    async fn create_proj_table(&self) -> Result<()>;
    async fn insert_project(&self, entity: Project) -> Result<()>;
    async fn remove_project(&self, key: String) -> Result<u32>;
    async fn get_project(&self, key: String) -> Result<Project>;
    async fn list_project<K: Fn(&Project) -> bool>(&self, pred: Option<K>) -> Result<Vec<Project>>;
}

#[async_trait]
pub trait NoteRepository: Initable {
    async fn create_note_table(&self) -> Result<()>;
    async fn insert_note(&self, entity: Note) -> Result<()>;
    async fn remove_note(&self, key: String) -> Result<u32>;
    async fn get_note(&self, key: String) -> Result<Note>;
    async fn list_note<K: Fn(&Note) -> bool>(&self, pred: Option<K>) -> Result<Vec<Note>>;
    async fn update_note(&self, key: String, text: String) -> Result<()>;
}


#[derive(Debug, Clone, FromRow)]
pub struct Project {
    guid: String,
    name: String,
    ts: chrono::NaiveDateTime,
}

impl Project {
    pub fn guid(&self) -> &String {
        &self.guid
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn ts(&self) -> chrono::NaiveDateTime {
        self.ts
    }
    pub fn new(guid: String, name: String, ts: chrono::NaiveDateTime) -> Self {
        Self { guid, name, ts }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Note {
    guid: String,
    project_id: String,
    name: String,
    content: String,
    ts: chrono::NaiveDateTime,
}

impl Note {
    pub fn guid(&self) -> &String {
        &self.guid
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
    pub fn new(guid: String, project_id: String, name: String, content: String, ts: chrono::NaiveDateTime) -> Self {
        Self { guid, project_id, name, content, ts }
    }
}