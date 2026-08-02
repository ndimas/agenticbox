//! `brain-ingest` — fetch connectors, distill + embed, upsert into the store.
//!
//! Usage:
//!   brain-ingest [--config path] [--source github|audit|all]
//!
//! Prints per-source inserted/updated/skipped counts, store coverage, and the
//! DeepSeek cache hit-ratio (the #1 cost lever — live from day 1).

use std::path::PathBuf;

use brain::mcp::{BrainConfig, BrainRuntime};
use brain::provider::DISTILL_SYSTEM_PREFIX;
use brain::store::NewKnowledge;
use tracing_subscriber::EnvFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mut args = std::env::args().skip(1);
    let mut config_path: Option<PathBuf> = None;
    let mut source_filter: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--config" => config_path = args.next().map(PathBuf::from),
            "--source" => source_filter = args.next(),
            other => {
                eprintln!("unknown arg: {other}");
                eprintln!("usage: brain-ingest [--config path] [--source github|audit|all]");
                std::process::exit(2);
            }
        }
    }
    let config_path = config_path.unwrap_or_else(BrainConfig::default_path);

    let runtime = tokio::runtime::Runtime::new()?;
    let result: anyhow::Result<()> = runtime.block_on(async move {
        let rt = BrainRuntime::from_config(&config_path).await?;
        let want = source_filter.as_deref().unwrap_or("all");

        let mut total_inserted = 0usize;
        let mut total_updated = 0usize;
        let mut total_skipped = 0usize;

        for connector in &rt.connectors {
            if want != "all" && connector.name() != want {
                continue;
            }
            let docs = connector.fetch().await?;
            tracing::info!(source = connector.name(), docs = docs.len(), "fetched");

            let mut rows = Vec::with_capacity(docs.len());
            for doc in &docs {
                // DeepSeek cache-friendly: DISTILL_SYSTEM_PREFIX is byte-stable
                // across every call; only the raw content is appended.
                let distilled = rt
                    .provider
                    .distill(DISTILL_SYSTEM_PREFIX, &doc.raw_text)
                    .await?;
                let embedding = match rt.provider.embed(&doc.raw_text).await {
                    Ok(e) => Some(e),
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            source = connector.name(),
                            source_id = &doc.source_id,
                            "embed failed — row stored without embedding"
                        );
                        None
                    }
                };
                rows.push(NewKnowledge {
                    source: doc.source.clone(),
                    source_id: doc.source_id.clone(),
                    chunk_key: format!("{}/{}", doc.source, doc.source_id),
                    kind: doc.kind.clone(),
                    title: distilled.title.clone().or_else(|| doc.title.clone()),
                    author: doc.author.clone(),
                    summary: Some(distilled.summary).filter(|s| !s.is_empty()),
                    question: distilled.question,
                    resolution: distilled.resolution,
                    systems: distilled.systems,
                    raw_text: doc.raw_text.clone(),
                    embedding,
                    distilled_by: Some(rt.provider.models().chat.clone()),
                    embedded_by: Some(rt.provider.models().embed.clone()),
                    project: doc.project.clone(),
                    created_at: doc.created_at,
                });
            }
            let (ins, upd, skip) = rt.store.upsert(&rows).await?;
            total_inserted += ins;
            total_updated += upd;
            total_skipped += skip;
            tracing::info!(
                source = connector.name(),
                inserted = ins,
                updated = upd,
                skipped = skip,
                "ingested"
            );
        }

        let stats = rt.store.stats().await?;
        println!(
            "inserted={} updated={} skipped={} total_rows={} embedded={}/{}",
            total_inserted,
            total_updated,
            total_skipped,
            stats.total_rows,
            stats.rows_with_embedding,
            stats.total_rows
        );
        for (source, n) in &stats.by_source {
            println!("  {source}: {n}");
        }
        let cs = rt.provider.cache_stats();
        println!(
            "cache: calls={} hit_tokens={} miss_tokens={} hit_ratio={}",
            cs.calls,
            cs.hit_tokens,
            cs.miss_tokens,
            cs.hit_ratio()
                .map(|r| format!("{:.1}%", r * 100.0))
                .unwrap_or_else(|| "n/a (provider reported none)".into())
        );
        Ok(())
    });
    result?;
    Ok(())
}
