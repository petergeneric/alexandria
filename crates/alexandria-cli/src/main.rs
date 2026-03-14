use chrono::{DateTime, Datelike, Local, Utc};
use clap::{Parser, Subcommand};
use alexandria_core::index::open_or_create_index;
use alexandria_core::search::SearchEngine;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "alex")]
#[command(about = "Full-text search for browsing history")]
#[command(version)]
struct Cli {
    /// Path to the index directory
    #[arg(long)]
    index_dir: Option<String>,

    /// Output results as JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search indexed pages
    Search {
        /// Search query
        query: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Result offset for pagination
        #[arg(short, long, default_value = "0")]
        offset: usize,

        /// Show full HTML content instead of snippet
        #[arg(long)]
        raw: bool,
    },
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn is_index_dir(path: &PathBuf) -> bool {
    path.join("meta.json").exists()
}

fn resolve_index_path(explicit: Option<&str>) -> PathBuf {
    // 1. Explicit --index-path flag
    if let Some(p) = explicit {
        let path = expand_tilde(p);
        if !is_index_dir(&path) {
            eprintln!("Error: no Alexandria index found at {}", path.display());
            std::process::exit(1);
        }
        return path;
    }

    // 2. Current working directory
    let cwd = std::env::current_dir().unwrap_or_default();
    if is_index_dir(&cwd) {
        return cwd;
    }

    // 3. macOS app default location
    if let Some(home) = dirs::home_dir() {
        let app_index = home
            .join("Library/Application Support/works.peter.alexandria/index");
        if is_index_dir(&app_index) {
            return app_index;
        }
    }

    eprintln!("Error: no Alexandria index found. Searched:");
    eprintln!("  - current directory: {}", cwd.display());
    if let Some(home) = dirs::home_dir() {
        eprintln!(
            "  - {}",
            home.join("Library/Application Support/works.peter.alexandria/index")
                .display()
        );
    }
    eprintln!();
    eprintln!("Use --index-dir to specify the index location.");
    std::process::exit(1);
}

fn highlight_keywords(text: &str, query: &str) -> String {
    let keywords: Vec<String> = query
        .split_whitespace()
        .filter(|w| !matches!(w.to_uppercase().as_str(), "AND" | "OR" | "NOT"))
        .map(|w| w.to_lowercase())
        .collect();

    if keywords.is_empty() {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len() + 64);
    let lower = text.to_lowercase();
    let mut last_end = 0;

    // Find all keyword matches, sorted by position
    let mut matches: Vec<(usize, usize)> = Vec::new();
    for keyword in &keywords {
        let mut start = 0;
        while let Some(pos) = lower[start..].find(keyword.as_str()) {
            let abs_pos = start + pos;
            matches.push((abs_pos, abs_pos + keyword.len()));
            start = abs_pos + keyword.len();
        }
    }
    matches.sort_by_key(|m| m.0);

    for (start, end) in matches {
        if start < last_end {
            continue; // skip overlapping matches
        }
        result.push_str(&text[last_end..start]);
        result.push_str("\x1b[1;33m"); // bold yellow
        result.push_str(&text[start..end]);
        result.push_str("\x1b[0m");
        last_end = end;
    }
    result.push_str(&text[last_end..]);

    result
}

fn format_relative_time(dt: &DateTime<Utc>) -> String {
    let now = Local::now();
    let local_dt = dt.with_timezone(&now.timezone());
    let duration = now.signed_duration_since(local_dt);

    let total_seconds = duration.num_seconds();
    if total_seconds < 0 {
        return local_dt.format("%H:%M").to_string();
    }

    // Less than 5 minutes: "just now"
    if total_seconds < 300 {
        return "just now".to_string();
    }

    // Last hour
    if total_seconds < 3600 {
        let mins = total_seconds / 60;
        return format!("{mins} minutes ago");
    }

    let today = now.date_naive();
    let dt_date = local_dt.date_naive();

    // Today: show time
    if dt_date == today {
        return local_dt.format("%H:%M").to_string();
    }

    // Yesterday
    if dt_date == today.pred_opt().unwrap_or(today) {
        return format!("yesterday {}", local_dt.format("%H:%M"));
    }

    // This week (within last 7 days)
    let days_ago = (today - dt_date).num_days();
    if days_ago < 7 {
        return format!("{} {}", local_dt.format("%A"), local_dt.format("%H:%M"));
    }

    // Last week
    if days_ago < 14 {
        return "last week".to_string();
    }

    // Last 10 months: "4 Jan"
    let months_diff = (now.year() - local_dt.year()) * 12 + (now.month() as i32 - local_dt.month() as i32);
    if months_diff < 10 {
        return local_dt.format("%-d %b").to_string();
    }

    // Older: "4 Jan 2025"
    local_dt.format("%-d %b %Y").to_string()
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let index_path = resolve_index_path(cli.index_dir.as_deref());

    match cli.command {
        Commands::Search { query, limit, offset, raw } => {
            let index = match open_or_create_index(&index_path) {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("Error opening index: {e}");
                    std::process::exit(1);
                }
            };

            let engine = SearchEngine::new(index);
            let results = match engine.search(&query, limit, offset) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Error searching: {e}");
                    std::process::exit(1);
                }
            };

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&results).unwrap());
            } else {
                if results.is_empty() {
                    println!("No results found.");
                    return;
                }
                for (i, result) in results.iter().enumerate() {
                    if i > 0 {
                        println!("---");
                    }
                    println!("{}", highlight_keywords(&result.title, &query));
                    let when = result.visited_at
                        .map(|dt| format_relative_time(&dt))
                        .unwrap_or_default();
                    println!("  URL:    {}", result.url);
                    println!("  Domain: {}", result.domain);
                    if !when.is_empty() {
                        println!("  When:   {}", when);
                    }
                    if raw {
                        println!("\n{}\n", result.html);
                    } else {
                        println!("  {}", highlight_keywords(&result.content_snippet, &query));
                    }
                }
            }
        }
    }
}
