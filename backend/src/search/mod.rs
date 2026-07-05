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

async fn search_notes(
    pool: &PgPool,
    engagement_id: Uuid,
    query: &str,
    enabled: bool,
) -> Result<Vec<SearchResult>, sqlx::Error> {
    if !enabled {
        return Ok(Vec::new());
    }
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

    Ok(rows
        .into_iter()
        .map(|r| SearchResult {
            result_type: "note".into(),
            id: r.id,
            title: r.title.unwrap_or_else(|| "Untitled note".into()),
            snippet: truncate(&r.body_md, 200),
        })
        .collect())
}

async fn search_findings(
    pool: &PgPool,
    engagement_id: Uuid,
    query: &str,
    enabled: bool,
) -> Result<Vec<SearchResult>, sqlx::Error> {
    if !enabled {
        return Ok(Vec::new());
    }
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

    Ok(rows
        .into_iter()
        .map(|r| SearchResult {
            result_type: "finding".into(),
            id: r.id,
            title: r.title,
            snippet: truncate(&r.description_md, 200),
        })
        .collect())
}

async fn search_hosts(
    pool: &PgPool,
    engagement_id: Uuid,
    query: &str,
    enabled: bool,
) -> Result<Vec<SearchResult>, sqlx::Error> {
    if !enabled {
        return Ok(Vec::new());
    }
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

    Ok(rows
        .into_iter()
        .map(|r| SearchResult {
            result_type: "host".into(),
            id: r.id,
            title: r.label,
            snippet: truncate(&r.general_info_md, 200),
        })
        .collect())
}

async fn search_credentials(
    pool: &PgPool,
    engagement_id: Uuid,
    query: &str,
    enabled: bool,
) -> Result<Vec<SearchResult>, sqlx::Error> {
    if !enabled {
        return Ok(Vec::new());
    }
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

    Ok(rows
        .into_iter()
        .map(|r| SearchResult {
            result_type: "credential".into(),
            id: r.id,
            title: r.username,
            snippet: r.domain.unwrap_or_default(),
        })
        .collect())
}

async fn search_attachments(
    pool: &PgPool,
    engagement_id: Uuid,
    query: &str,
    enabled: bool,
) -> Result<Vec<SearchResult>, sqlx::Error> {
    if !enabled {
        return Ok(Vec::new());
    }
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

    Ok(rows
        .into_iter()
        .map(|r| SearchResult {
            result_type: "attachment".into(),
            id: r.id,
            title: r.filename,
            snippet: r.caption.unwrap_or_default(),
        })
        .collect())
}

/// Cross-entity full-text search over the search_vector columns seeded in
/// Phase 2 (migration 0007). `types` filters which entity kinds to search;
/// empty means all of them. Each type is queried and ranked independently
/// (ts_rank isn't meaningfully comparable across differently-shaped
/// documents) -- concurrently, since they're all independent reads against
/// the same pool -- then concatenated in a stable (not completion) order.
pub async fn search(
    pool: &PgPool,
    engagement_id: Uuid,
    query: &str,
    types: &[String],
) -> Result<Vec<SearchResult>, sqlx::Error> {
    let want = |t: &str| types.is_empty() || types.iter().any(|x| x == t);

    let (notes, findings, hosts, credentials, attachments) = tokio::try_join!(
        search_notes(pool, engagement_id, query, want("notes")),
        search_findings(pool, engagement_id, query, want("findings")),
        search_hosts(pool, engagement_id, query, want("hosts")),
        search_credentials(pool, engagement_id, query, want("credentials")),
        search_attachments(pool, engagement_id, query, want("attachments")),
    )?;

    Ok([notes, findings, hosts, credentials, attachments]
        .into_iter()
        .flatten()
        .collect())
}
