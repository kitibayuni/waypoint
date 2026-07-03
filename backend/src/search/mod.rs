use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize)]
pub struct SearchResult {
    pub result_type: String,
    pub id: Uuid,
    pub title: String,
    pub snippet: String,
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

/// Cross-entity full-text search over the search_vector columns seeded in
/// Phase 2 (migration 0007). `types` filters which entity kinds to search;
/// empty means all of them. Each type is queried and ranked independently
/// (ts_rank isn't meaningfully comparable across differently-shaped
/// documents), then the results are concatenated in a stable order.
pub async fn search(
    pool: &PgPool,
    engagement_id: Uuid,
    query: &str,
    types: &[String],
) -> Result<Vec<SearchResult>, sqlx::Error> {
    let want = |t: &str| types.is_empty() || types.iter().any(|x| x == t);
    let mut results = Vec::new();

    if want("notes") {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            title: Option<String>,
            body_md: String,
        }
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, title, body_md FROM notes
             WHERE engagement_id = $1 AND search_vector @@ websearch_to_tsquery('english', $2)
             ORDER BY ts_rank(search_vector, websearch_to_tsquery('english', $2)) DESC LIMIT 25",
        )
        .bind(engagement_id)
        .bind(query)
        .fetch_all(pool)
        .await?;
        for r in rows {
            results.push(SearchResult {
                result_type: "note".into(),
                id: r.id,
                title: r.title.unwrap_or_else(|| "Untitled note".into()),
                snippet: truncate(&r.body_md, 200),
            });
        }
    }

    if want("findings") {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            title: String,
            description_md: String,
        }
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, title, description_md FROM findings
             WHERE engagement_id = $1 AND search_vector @@ websearch_to_tsquery('english', $2)
             ORDER BY ts_rank(search_vector, websearch_to_tsquery('english', $2)) DESC LIMIT 25",
        )
        .bind(engagement_id)
        .bind(query)
        .fetch_all(pool)
        .await?;
        for r in rows {
            results.push(SearchResult {
                result_type: "finding".into(),
                id: r.id,
                title: r.title,
                snippet: truncate(&r.description_md, 200),
            });
        }
    }

    if want("hosts") {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            label: String,
            general_info_md: String,
        }
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, label, general_info_md FROM hosts
             WHERE engagement_id = $1 AND search_vector @@ websearch_to_tsquery('english', $2)
             ORDER BY ts_rank(search_vector, websearch_to_tsquery('english', $2)) DESC LIMIT 25",
        )
        .bind(engagement_id)
        .bind(query)
        .fetch_all(pool)
        .await?;
        for r in rows {
            results.push(SearchResult {
                result_type: "host".into(),
                id: r.id,
                title: r.label,
                snippet: truncate(&r.general_info_md, 200),
            });
        }
    }

    if want("credentials") {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            username: String,
            domain: Option<String>,
        }
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, username, domain FROM credentials
             WHERE engagement_id = $1 AND search_vector @@ websearch_to_tsquery('english', $2)
             ORDER BY ts_rank(search_vector, websearch_to_tsquery('english', $2)) DESC LIMIT 25",
        )
        .bind(engagement_id)
        .bind(query)
        .fetch_all(pool)
        .await?;
        for r in rows {
            results.push(SearchResult {
                result_type: "credential".into(),
                id: r.id,
                title: r.username,
                snippet: r.domain.unwrap_or_default(),
            });
        }
    }

    if want("attachments") {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            filename: String,
            caption: Option<String>,
        }
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, filename, caption FROM attachments
             WHERE engagement_id = $1 AND search_vector @@ websearch_to_tsquery('english', $2)
             ORDER BY ts_rank(search_vector, websearch_to_tsquery('english', $2)) DESC LIMIT 25",
        )
        .bind(engagement_id)
        .bind(query)
        .fetch_all(pool)
        .await?;
        for r in rows {
            results.push(SearchResult {
                result_type: "attachment".into(),
                id: r.id,
                title: r.filename,
                snippet: r.caption.unwrap_or_default(),
            });
        }
    }

    Ok(results)
}
