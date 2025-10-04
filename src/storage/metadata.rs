use std::{path::Path, str::FromStr};

use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions};

use crate::{error::Result, models::ObjectMetadata};

#[derive(Clone)]
pub struct MetadataStore {
    pool: SqlitePool,
}

impl MetadataStore {
    pub async fn new(database_url: &str) -> Result<Self> {
        if let Some(db_path) = database_url.strip_prefix("sqlite:") {
            if let Some(parent) = Path::new(db_path).parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS objects (
                id TEXT PRIMARY KEY,
                key TEXT NOT NULL UNIQUE,
                size INTEGER NOT NULL,
                content_type TEXT NOT NULL,
                etag TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_objects_key ON objects(key)")
            .execute(&pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_objects_content_type ON objects(content_type)")
            .execute(&pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_objects_size ON objects(size)")
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    pub async fn insert(&self, metadata: &ObjectMetadata) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO objects (id, key, size, content_type, etag, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
                size = excluded.size,
                content_type = excluded.content_type,
                etag = excluded.etag,
                created_at = excluded.created_at
            "#,
        )
        .bind(&metadata.id)
        .bind(&metadata.key)
        .bind(metadata.size)
        .bind(&metadata.content_type)
        .bind(&metadata.etag)
        .bind(metadata.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Option<ObjectMetadata>> {
        let row = sqlx::query(
            "SELECT id, key, size, content_type, etag, created_at FROM objects WHERE key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let created_at_str: String = row.get("created_at");
                Ok(Some(ObjectMetadata {
                    id: row.get("id"),
                    key: row.get("key"),
                    size: row.get("size"),
                    content_type: row.get("content_type"),
                    etag: row.get("etag"),
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn list(
        &self,
        prefix: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<ObjectMetadata>> {
        let query = match prefix {
            Some(p) => {
                let pattern = format!("{}%", p);
                sqlx::query(
                    "SELECT id, key, size, content_type, etag, created_at 
                     FROM objects 
                     WHERE key LIKE ? 
                     ORDER BY key 
                     LIMIT ?",
                )
                .bind(pattern)
                .bind(limit.unwrap_or(1000))
            }
            None => sqlx::query(
                "SELECT id, key, size, content_type, etag, created_at 
                     FROM objects 
                     ORDER BY key 
                     LIMIT ?",
            )
            .bind(limit.unwrap_or(1000)),
        };

        let rows = query.fetch_all(&self.pool).await?;

        let mut objects = Vec::new();
        for row in rows {
            let created_at_str: String = row.get("created_at");
            objects.push(ObjectMetadata {
                id: row.get("id"),
                key: row.get("key"),
                size: row.get("size"),
                content_type: row.get("content_type"),
                etag: row.get("etag"),
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            });
        }

        Ok(objects)
    }

    pub async fn search(
        &self,
        key_pattern: Option<&str>,
        content_type: Option<&str>,
        min_size: Option<i64>,
        max_size: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<ObjectMetadata>> {
        let mut conditions = Vec::new();
        let mut query_str = String::from(
            "SELECT id, key, size, content_type, etag, created_at FROM objects WHERE 1=1",
        );

        if key_pattern.is_some() {
            conditions.push("key LIKE ?");
        }
        if content_type.is_some() {
            conditions.push("content_type = ?");
        }
        if min_size.is_some() {
            conditions.push("size >= ?");
        }
        if max_size.is_some() {
            conditions.push("size <= ?");
        }

        for condition in conditions {
            query_str.push_str(" AND ");
            query_str.push_str(condition);
        }

        query_str.push_str(" ORDER BY created_at DESC LIMIT ?");

        let mut query = sqlx::query(&query_str);

        if let Some(pattern) = key_pattern {
            query = query.bind(format!("%{}%", pattern));
        }
        if let Some(ct) = content_type {
            query = query.bind(ct);
        }
        if let Some(min) = min_size {
            query = query.bind(min);
        }
        if let Some(max) = max_size {
            query = query.bind(max);
        }

        query = query.bind(limit.unwrap_or(100));

        let rows = query.fetch_all(&self.pool).await?;

        let mut objects = Vec::new();
        for row in rows {
            let created_at_str: String = row.get("created_at");
            objects.push(ObjectMetadata {
                id: row.get("id"),
                key: row.get("key"),
                size: row.get("size"),
                content_type: row.get("content_type"),
                etag: row.get("etag"),
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            });
        }

        Ok(objects)
    }

    pub async fn delete(&self, key: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM objects WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_by_prefix(&self, prefix: &str) -> Result<i64> {
        let pattern = format!("{}%", prefix);
        let result = sqlx::query("DELETE FROM objects WHERE key LIKE ?")
            .bind(pattern)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as i64)
    }

    pub async fn get_stats(&self) -> Result<(i64, i64)> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count, COALESCE(SUM(size), 0) as total_size FROM objects",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((row.get(0), row.get(1)))
    }
}
