use chrono::{DateTime, Datelike, Local, Utc};
use clap::{Parser, Subcommand};
use alexandria_core::blocklist::Blocklist;
use alexandria_core::extract;
use alexandria_core::index::open_or_create_index;
use alexandria_core::page_store::PageStore;
use alexandria_core::search::SearchEngine;
use std::collections::HashMap;
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

        /// Path to the page store database (for snippets)
        #[arg(long)]
        store: Option<String>,
    },

    /// Import browsing history from a Firefox places.sqlite copy
    ImportFirefox {
        /// Path to Firefox places.sqlite copy
        #[arg(long, default_value = "/tmp/places.sqlite")]
        places: String,

        /// Path to pages.db
        #[arg(long)]
        store: Option<String>,
    },

    /// Backfill domain (www-strip) and site_group columns in pages.db
    Migrate {
        /// Path to pages.db
        #[arg(long)]
        store: Option<String>,

        /// Print what would change without writing
        #[arg(long)]
        dry_run: bool,
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

fn resolve_store_path(explicit: Option<&str>) -> PathBuf {
    if let Some(s) = explicit {
        return expand_tilde(s);
    }
    dirs::home_dir()
        .map(|h| h.join("Library/Application Support/works.peter.alexandria/pages.db"))
        .unwrap_or_else(|| PathBuf::from("pages.db"))
}

fn import_firefox(places_path: &str, store: Option<&str>) {
    use rusqlite::OpenFlags;
    use xxhash_rust::xxh3::xxh3_128;

    let places = expand_tilde(places_path);
    if !places.exists() {
        eprintln!("Error: Firefox database not found at {}", places.display());
        eprintln!("Copy your places.sqlite first, e.g.:");
        eprintln!("  cp ~/Library/Application\\ Support/Firefox/Profiles/*.default-release/places.sqlite /tmp/places.sqlite");
        std::process::exit(1);
    }

    let ff_db = match rusqlite::Connection::open_with_flags(&places, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error opening Firefox database: {e}");
            std::process::exit(1);
        }
    };

    let store_path = resolve_store_path(store);
    let page_store = match PageStore::open(&store_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error opening page store at {}: {e}", store_path.display());
            std::process::exit(1);
        }
    };

    let blocklist = Blocklist::load();

    let mut stmt = match ff_db.prepare(
            "SELECT url, title, description, last_visit_date
             FROM moz_places
             WHERE title IS NOT NULL AND title != ''
               AND last_visit_date IS NOT NULL
             ORDER BY last_visit_date DESC",
        ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error preparing Firefox query: {e}");
            std::process::exit(1);
        }
    };

    // Collect rows, deduplicating by title (first occurrence = most recent)
    let mut seen_titles: HashMap<String, ()> = HashMap::new();
    let mut imported = 0u64;

    struct Row {
        url: String,
        title: String,
        description: Option<String>,
        last_visit_date: i64,
    }

    let rows: Vec<Row> = match stmt.query_map([], |row| {
            Ok(Row {
                url: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                last_visit_date: row.get(3)?,
            })
        }) {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(e) => {
            eprintln!("Error querying Firefox history: {e}");
            std::process::exit(1);
        }
    };

    // Collect filtered, deduplicated rows first
    let mut to_insert: Vec<Row> = Vec::new();
    for row in rows {
        // Skip blocked URLs
        if blocklist.is_url_blocked(&row.url) {
            continue;
        }

        // Skip IP-address hosts
        let hostname = extract::extract_domain(&row.url);
        if hostname.parse::<std::net::IpAddr>().is_ok() {
            continue;
        }

        // Skip Amazon/Apple domains
        let host_lower = hostname.to_lowercase();
        if host_lower.contains("amazon") || host_lower.contains("apple") {
            continue;
        }

        // Skip checkout/payment titles
        let title_lower = row.title.to_lowercase();
        if title_lower.contains("checkout") || title_lower.contains("pay now") {
            continue;
        }

        // Deduplicate by title
        if seen_titles.contains_key(&row.title) {
            continue;
        }
        seen_titles.insert(row.title.clone(), ());

        to_insert.push(row);
    }

    for row in &to_insert {
        let content = row.description.as_deref().unwrap_or_default();
        let domain = extract::extract_domain(&row.url);
        let site_group = extract::extract_site_group(&row.url);
        let captured_at = row.last_visit_date / 1_000_000; // Firefox uses microseconds
        let content_hash = xxh3_128(content.as_bytes()).to_le_bytes();
        if let Err(e) = page_store.insert(&row.url, &row.title, content.as_bytes(), &domain, &site_group, captured_at, &content_hash) {
            eprintln!("Warning: failed to insert {}: {e}", row.url);
            continue;
        }
        imported += 1;
    }

    println!("Imported {imported} pages from Firefox history.");
}

fn migrate_store(store: Option<&str>, dry_run: bool) {
    let store_path = resolve_store_path(store);
    if !store_path.exists() {
        eprintln!("Error: page store not found at {}", store_path.display());
        std::process::exit(1);
    }

    let db = match rusqlite::Connection::open(&store_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error opening page store: {e}");
            std::process::exit(1);
        }
    };

    // Ensure site_group column exists (may be missing on pre-migration DBs)
    let has_site_group: bool = db
        .prepare("SELECT COUNT(*) FROM pragma_table_info('pages') WHERE name='site_group'")
        .and_then(|mut s| s.query_row([], |row| row.get::<_, i64>(0)))
        .map(|c| c > 0)
        .unwrap_or(false);
    if !has_site_group {
        db.execute_batch("ALTER TABLE pages ADD COLUMN site_group TEXT NOT NULL DEFAULT ''")
            .unwrap();
        println!("Added site_group column to schema.");
    }

    let mut stmt = match db.prepare("SELECT id, url, domain, site_group FROM pages") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error preparing migration query: {e}");
            std::process::exit(1);
        }
    };

    struct Row {
        id: i64,
        url: String,
        old_domain: String,
        old_site_group: String,
    }

    let rows: Vec<Row> = match stmt.query_map([], |row| {
            Ok(Row {
                id: row.get(0)?,
                url: row.get(1)?,
                old_domain: row.get(2)?,
                old_site_group: row.get(3)?,
            })
        }) {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(e) => {
            eprintln!("Error querying pages for migration: {e}");
            std::process::exit(1);
        }
    };

    let total = rows.len();
    let mut domain_updates = 0u64;
    let mut group_updates = 0u64;

    if !dry_run {
        if let Err(e) = db.execute_batch("BEGIN") {
            eprintln!("Error starting transaction: {e}");
            std::process::exit(1);
        }
    }

    let mut update_stmt = if !dry_run {
        match db.prepare("UPDATE pages SET domain = ?1, site_group = ?2 WHERE id = ?3") {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("Error preparing update statement: {e}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    for row in &rows {
        let new_domain = extract::extract_domain(&row.url);
        let new_site_group = extract::extract_site_group(&row.url);

        let domain_changed = new_domain != row.old_domain;
        let group_changed = new_site_group != row.old_site_group;

        if !domain_changed && !group_changed {
            continue;
        }

        if domain_changed {
            domain_updates += 1;
        }
        if group_changed {
            group_updates += 1;
        }

        if dry_run {
            if domain_changed {
                println!(
                    "  id={}: domain '{}' → '{}'",
                    row.id, row.old_domain, new_domain
                );
            }
            if group_changed {
                println!(
                    "  id={}: site_group '{}' → '{}'",
                    row.id, row.old_site_group, new_site_group
                );
            }
        } else if let Some(ref mut stmt) = update_stmt {
            if let Err(e) = stmt.execute(rusqlite::params![new_domain, new_site_group, row.id]) {
                eprintln!("Warning: failed to update page {}: {e}", row.id);
            }
        }
    }

    if !dry_run {
        if let Err(e) = db.execute_batch("COMMIT") {
            eprintln!("Error committing transaction: {e}");
            std::process::exit(1);
        }
    }

    let verb = if dry_run { "Would update" } else { "Updated" };
    println!(
        "{verb} {domain_updates} domains, {group_updates} site_groups across {total} pages."
    );

    if !dry_run && (domain_updates > 0 || group_updates > 0) {
        println!("Run a reindex to update the search index.");
    }
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
        Commands::Migrate { store, dry_run } => {
            migrate_store(store.as_deref(), dry_run);
            return;
        }
        Commands::ImportFirefox { places, store } => {
            import_firefox(&places, store.as_deref());
        }
        Commands::Search { query, limit, offset, store } => {
            let index = match open_or_create_index(&index_path) {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("Error opening index: {e}");
                    std::process::exit(1);
                }
            };

            let engine = match SearchEngine::new(index) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Error initializing search: {e}");
                    std::process::exit(1);
                }
            };

            let store_path = store
                .map(|s| expand_tilde(&s))
                .or_else(|| {
                    dirs::home_dir().map(|h| {
                        h.join("Library/Application Support/works.peter.alexandria/pages.db")
                    })
                });
            let page_store = store_path
                .as_ref()
                .filter(|p| p.exists())
                .and_then(|p| PageStore::open(p).ok());

            let results = match engine.search(&query, limit, offset, page_store.as_ref()) {
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
                    if !result.content_snippet.is_empty() {
                        println!("  {}", highlight_keywords(&result.content_snippet, &query));
                    }
                }
            }
        }
    }
}
