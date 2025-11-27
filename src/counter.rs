use chrono::{DateTime, Utc};
use diesel::deserialize::QueryableByName;
use diesel::sql_types;
use diesel_async::RunQueryDsl;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, interval};

use crate::db_pool::DbPool;

#[derive(QueryableByName)]
struct BatchUpdateResult {
    #[diesel(sql_type = sql_types::Integer)]
    batch_update_clicks: i32,
}

#[derive(Debug, Clone)]
struct ClickData {
    count: i32,
    last_used: DateTime<Utc>,
}

// Shared counter state with last used timestamps
#[derive(Clone)]
pub struct ClickCounter {
    counts: Arc<RwLock<HashMap<String, ClickData>>>,
}

impl ClickCounter {
    pub fn new() -> Self {
        Self {
            counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn increment(&self, id: &str) {
        let mut counts = self.counts.write().await;
        counts
            .entry(id.to_string())
            .and_modify(|data| {
                data.count += 1;
                data.last_used = Utc::now();
            })
            .or_insert(ClickData {
                count: 1,
                last_used: Utc::now(),
            });
    }

    async fn drain(&self) -> HashMap<String, ClickData> {
        let mut counts = self.counts.write().await;
        std::mem::take(&mut *counts)
    }
}

pub async fn start_counter_flusher(
    counter: Arc<ClickCounter>,
    db: DbPool,
    interval_duration: Duration,
) {
    let mut ticker = interval(interval_duration);

    tracing::info!("counter flusher started");

    loop {
        ticker.tick().await;

        let counts = counter.drain().await;

        if counts.is_empty() {
            continue;
        }

        if let Err(e) = flush_counts_to_db(db.clone(), counts).await {
            tracing::error!("failed to flush click counts: {e}");
        }
    }
}

// Much cleaner - just call the stored function
async fn flush_counts_to_db(
    db: DbPool,
    counts: HashMap<String, ClickData>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ids: Vec<String> = counts.keys().cloned().collect();
    let increments: Vec<i32> = counts.values().map(|d| d.count).collect();
    let timestamps: Vec<DateTime<Utc>> = counts.values().map(|d| d.last_used).collect();

    let result: BatchUpdateResult = diesel::sql_query("SELECT batch_update_clicks($1, $2, $3)")
        .bind::<sql_types::Array<sql_types::Text>, _>(ids)
        .bind::<sql_types::Array<sql_types::Integer>, _>(increments)
        .bind::<sql_types::Array<sql_types::Timestamptz>, _>(timestamps)
        .get_result(&mut db.0.get().await?)
        .await?;

    let rows_updated = result.batch_update_clicks;

    tracing::info!(rows_updated, "flushed link counters");

    Ok(())
}
