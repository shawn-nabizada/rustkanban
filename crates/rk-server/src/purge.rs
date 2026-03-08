use sqlx::PgPool;
use uuid::Uuid;

pub fn spawn_purge_job(pool: PgPool) {
    tokio::spawn(async move {
        // Wait 60 seconds after startup before first run
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400)); // 24h
        loop {
            interval.tick().await;
            if let Err(e) = run_purge(&pool).await {
                tracing::error!("Purge job failed: {}", e);
            }
        }
    });
}

async fn run_purge(pool: &PgPool) -> Result<(), sqlx::Error> {
    tracing::info!("Running purge job");

    // Mark stale devices (no sync in 90 days)
    let stale_count = sqlx::query(
        "UPDATE devices SET stale = TRUE \
         WHERE last_synced_at < NOW() - INTERVAL '90 days' AND stale = FALSE",
    )
    .execute(pool)
    .await?
    .rows_affected();
    if stale_count > 0 {
        tracing::info!("Marked {} devices as stale", stale_count);
    }

    // For each user with non-stale devices: purge soft-deleted records
    let users: Vec<Uuid> =
        sqlx::query_scalar("SELECT DISTINCT user_id FROM devices WHERE stale = FALSE")
            .fetch_all(pool)
            .await?;

    for user_id in users {
        let oldest_sync: Option<chrono::NaiveDateTime> = sqlx::query_scalar(
            "SELECT MIN(last_synced_at) FROM devices \
             WHERE user_id = $1 AND stale = FALSE AND last_synced_at IS NOT NULL",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        if let Some(cutoff) = oldest_sync {
            // Hard-delete task_tags for deleted tasks past cutoff
            sqlx::query(
                "DELETE FROM task_tags WHERE task_uuid IN \
                 (SELECT uuid FROM tasks WHERE user_id = $1 AND deleted = TRUE AND deleted_at < $2)",
            )
            .bind(user_id)
            .bind(cutoff)
            .execute(pool)
            .await?;

            // Hard-delete tasks
            let task_count = sqlx::query(
                "DELETE FROM tasks WHERE user_id = $1 AND deleted = TRUE AND deleted_at < $2",
            )
            .bind(user_id)
            .bind(cutoff)
            .execute(pool)
            .await?
            .rows_affected();

            // Hard-delete tags
            let tag_count = sqlx::query(
                "DELETE FROM tags WHERE user_id = $1 AND deleted = TRUE AND deleted_at < $2",
            )
            .bind(user_id)
            .bind(cutoff)
            .execute(pool)
            .await?
            .rows_affected();

            if task_count > 0 || tag_count > 0 {
                tracing::info!(
                    "Purged {} tasks and {} tags for user {}",
                    task_count,
                    tag_count,
                    user_id
                );
            }
        }
    }

    tracing::info!("Purge job complete");
    Ok(())
}
