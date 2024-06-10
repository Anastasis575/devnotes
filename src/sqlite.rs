#![allow(dead_code)]

use std::env;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use itertools::Itertools;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, SqliteConnection};

use crate::backend::{Initable, Note, NoteRepository, Project, ProjectRepository};

pub struct SqliteRepository {
    conn: SqliteConnection,
    initialized: bool,
}

impl SqliteRepository {
    fn conn(&self) -> &SqliteConnection {
        &self.conn
    }
    fn conn_mut(&mut self) -> &mut SqliteConnection {
        &mut self.conn
    }
    pub(crate) async fn default() -> Result<SqliteRepository> {
        Self::connect(
            env::current_exe()?
                .join("..")
                .join("note.db")
                .to_str()
                .ok_or(anyhow!("Could not evaluate path"))?
                .to_string(),
        )
        .await
    }
    async fn connect(conn_str: String) -> Result<SqliteRepository> {
        let conn = SqliteConnectOptions::from_str(&conn_str.to_owned())?
            .create_if_missing(true)
            .connect()
            .await?;
        let mut repo = SqliteRepository {
            conn,
            initialized: false,
        };
        repo.init().await?;
        Ok(repo)
    }
}

#[async_trait]
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
    async fn create_note_table(&mut self) -> Result<()> {
        sqlx::query("create table if not exists note(id nvarchar(256) primary key,project_id nvarchar(256) references project(id), name nvarchar(150),content text,ts datetime);").execute(self.conn_mut()).await?;
        Ok(())
    }

    async fn insert_note(&mut self, entity: Note) -> Result<()> {
        let output = sqlx::query("update note set content=?, ts=? where id=?;")
            .bind(entity.content().to_string())
            .bind(entity.ts().to_string())
            .bind(entity.guid().to_string())
            .execute(self.conn_mut())
            .await?;
        if output.rows_affected() == 0 {
            sqlx::query("insert into note(id,project_id,name,content,ts) values (?,?,?,?,?)")
                .bind(entity.guid().to_string())
                .bind(entity.project_id().to_string())
                .bind(entity.name().to_string())
                .bind(entity.content().to_string())
                .bind(entity.ts().to_string())
                .execute(self.conn_mut())
                .await?;
        }
        Ok(())
    }
    async fn remove_note(&mut self, key: String) -> Result<u32> {
        let count: u32 = sqlx::query("delete from note where id=?;")
            .bind(key.to_string())
            .execute(self.conn_mut())
            .await?
            .rows_affected() as u32;
        Ok(count)
    }

    async fn get_note(&mut self, key: String) -> Result<Note> {
        let item: Note = sqlx::query_as("select * from note where id=?;")
            .bind(key.to_string())
            .fetch_one(self.conn_mut())
            .await?;
        Ok(item)
    }

    async fn list_note(&mut self) -> Result<Vec<Note>> {
        let items: Vec<Note> = sqlx::query_as("select * from note;")
            .fetch_all(self.conn_mut())
            .await?;
        Ok(items)
    }

    async fn list_note_with_filter<T: Fn(&Note) -> bool + Send + Sync>(
        &mut self,
        pred: T,
    ) -> Result<Vec<Note>> {
        let items: Vec<Note> = self.list_note().await?;
        Ok(apply_filter(pred, items))
    }

    async fn update_note(&mut self, key: String, text: String, project_id: String) -> Result<u64> {
        let count = sqlx::query("update note set project_id=?,content=? where id=?;")
            .bind(project_id)
            .bind(text)
            .bind(key.to_string())
            .execute(self.conn_mut())
            .await?
            .rows_affected();
        Ok(count)
    }
}

#[async_trait]
impl ProjectRepository for SqliteRepository {
    async fn create_proj_table(&mut self) -> Result<()> {
        sqlx::query("create table if not exists project(id nvarchar(256) primary key,name nvarchar(150) unique,ts datetime);").execute(self.conn_mut()).await?;
        Ok(())
    }

    async fn insert_project(&mut self, entity: Project) -> Result<()> {
        sqlx::query("insert into project(id,name,ts) values(?,?,?);")
            .bind(entity.guid().to_string())
            .bind(entity.name())
            .bind(entity.ts().to_string())
            .execute(self.conn_mut())
            .await?;
        Ok(())
    }

    async fn remove_project(&mut self, key: String) -> Result<u32> {
        let count: u32 = sqlx::query("delete from project where id=?;")
            .bind(key.to_string())
            .execute(self.conn_mut())
            .await?
            .rows_affected() as u32;
        Ok(count)
    }

    async fn get_project(&mut self, key: String) -> Result<Project> {
        let item: Project = sqlx::query_as("select * from project where id=?;")
            .bind(key.to_string())
            .fetch_one(self.conn_mut())
            .await?;
        Ok(item)
    }

    async fn list_project(&mut self) -> Result<Vec<Project>> {
        let items: Vec<Project> = sqlx::query_as("select * from project;")
            .fetch_all(self.conn_mut())
            .await?;
        Ok(items)
    }

    async fn list_project_with_filter<T: Fn(&Project) -> bool + Send + Sync>(
        &mut self,
        pred: T,
    ) -> Result<Vec<Project>> {
        let items = self.list_project().await?;
        Ok(apply_filter(pred, items))
    }
}

fn apply_filter<T: Clone, K: Fn(&T) -> bool + Send + Sync>(pred: K, items: Vec<T>) -> Vec<T> {
    items
        .iter()
        .filter_map(|it| if pred(it) { Some(it.to_owned()) } else { None })
        .collect_vec()
}
