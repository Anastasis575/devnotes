use std::env;
use std::str::FromStr;
use String::String;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use itertools::Itertools;
use sqlx::{ConnectOptions, SqliteConnection};
use sqlx::sqlite::SqliteConnectOptions;

use crate::lib::{Initable, Note, NoteRepository, Project, ProjectRepository};

pub struct SqliteRepository {
    conn: SqliteConnection,
    initialized: bool,
}

impl SqliteRepository {
    fn conn(&self) -> &SqliteConnection {
        &self.conn
    }
    pub(crate) async fn default() -> Result<SqliteRepository> {
        Self::connect(env::current_exe()?.join("..").join("note.db").to_str().ok_or(anyhow!("Could not evaluate path"))?.to_string())
    }
    async fn connect(conn_str: String) -> Result<SqliteRepository> {
        let conn = SqliteConnectOptions::from_str(&conn_str.to_owned())?.connect().await?;
        let mut repo = SqliteRepository { conn, initialized: false };
        repo.init()?;
        Ok(repo)
    }
}

impl Initable for SqliteRepository {
    fn initialized(&self) -> bool {
        self.initialized
    }

    async fn init(&mut self) -> Result<()> {
        if !self.initialized() {
            self.create_proj_table().await?;
            self.create_note_table().await?;
        }
        self.initialized = true;
        Ok(())
    }
}

#[async_trait]
impl NoteRepository for SqliteRepository {
    async fn create_note_table(&self) -> Result<()> {
        sqlx::query("create table if not exists note(id nvarchar(256) primary key,project_id nvarchar(256) references project(id), name nvarchar(150),content text,ts datetime);").execute(self.conn()).await?;
        Ok(())
    }

    async fn insert_note(&self, entity: Note) -> Result<()> {
        sqlx::query("insert into note(id,project_id,name,content,ts) values(?,?,?) on duplicate key update text=variable(text);").bind(entity.guid().to_string()).bind(entity.project_id().to_string()).bind(entity.name().to_string()).bind(entity.content().to_string()).bind(entity.ts().to_string()).bind(entity.ts().to_string()).execute(self.conn()).await?;
        Ok(())
    }

    async fn remove_note(&self, key: String) -> Result<u32> {
        let count: u32 = sqlx::query("delete from note where id like '?%';").bind(key.to_string()).execute(self.conn()).await?;
        Ok(count)
    }

    async fn get_note(&self, key: String) -> Result<Note> {
        let item: Note = sqlx::query_as("select * from note where id like '?%';").bind(key.to_string()).fetch_one(self.conn()).await?;
        Ok(item)
    }

    async fn list_note<K: Fn(&Note) -> bool>(&self, pred: Option<K>) -> Result<Vec<Note>> {
        let items: Vec<Note> = sqlx::query_as("select * from note;").fetch_all(self.conn()).await?;
        Ok(apply_filter(pred, items))
    }

    async fn update_note(&self, key: String, text: String) -> Result<()> {
        sqlx::query("update note set content=? where id like '?%';").bind(text).bind(key.to_string()).execute(self.conn()).await?;
        Ok(())
    }
}

#[async_trait]
impl ProjectRepository for SqliteRepository {
    async fn create_proj_table(&self) -> Result<()> {
        sqlx::query("create table if not exists project(id nvarchar(256) primary key,name nvarchar(150) unique,ts datetime);").execute(self.conn()).await?;
        Ok(())
    }

    async fn insert_project(&self, entity: Project) -> Result<()> {
        sqlx::query("insert into project(id,name,ts) values(?,?,?);").bind(entity.guid().to_string()).bind(entity.name()).bind(entity.ts().to_string()).execute(self.conn()).await?;
        Ok(())
    }

    async fn remove_project(&self, key: String) -> Result<u32> {
        let count: u32 = sqlx::query("delete from project where id like '?%';").bind(key.to_string()).execute(self.conn()).await?;
        Ok(count)
    }

    async fn get_project(&self, key: String) -> Result<Project> {
        let item: Project = sqlx::query_as("select * from project where id like '?%';").bind(key.to_string()).fetch_one(self.conn()).await?;
        Ok(item)
    }

    async fn list_project<K: Fn(&Project) -> bool>(&self, pred: Option<K>) -> Result<Vec<Project>> {
        let items: Vec<Project> = sqlx::query_as("select * from project;").fetch_all(self.conn()).await?;
        Ok(apply_filter(pred, items))
    }
}

fn apply_filter<T, K: Fn(&T) -> bool>(pred: Option<K>, items: Vec<T>) -> Vec<T> {
    match pred {
        None => items,
        Some(p) => {
            items.iter().filter_map(|it| {
                if p(it) { Some(it.to_owned()) } else { None }
            }).collect_vec()
        }
    }
}