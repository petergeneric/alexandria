use clap::{Parser, Subcommand};
use alexandria_core::index::{build_schema, index_snapshots, open_or_create_index};
use alexandria_core::ingest::{IngestSource, RecollFileSource};
use alexandria_core::search::SearchEngine;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Parser)]
#[command(name = "alex")]
#[command(about = "Full-text search for browsing history")]
#[command(version)]
struct Cli {
    /// Path to the index directory
    #[arg(long, default_value = "./alexandria_index")]
    index_dir: String,

    /// Output results as JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index pages from a webcache directory
    Index {
        /// Path to the Recoll webcache directory
        #[arg(default_value = "~/Downloads/webcache")]
        source: String,
    },
    /// Delete existing index and rebuild from scratch
    Reindex {
        /// Path to the Recoll webcache directory
        #[arg(default_value = "~/Downloads/webcache")]
        source: String,
    },
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

fn read_last_indexed(index_path: &Path) -> Option<SystemTime> {
    let marker = index_path.join(".last-indexed");
    marker.metadata().ok()?.modified().ok()
}

fn write_last_indexed(index_path: &Path) {
    let marker = index_path.join(".last-indexed");
    if marker.exists() {
        // Update mtime by writing to it
        let _ = std::fs::write(&marker, "");
    } else {
        let _ = std::fs::create_dir_all(index_path);
        let _ = std::fs::write(&marker, "");
    }
}

fn run_index(source: &str, index_path: &PathBuf) {
    let source_path = expand_tilde(source);
    if !source_path.is_dir() {
        eprintln!("Error: source directory does not exist: {}", source_path.display());
        std::process::exit(1);
    }

    let mut file_source = RecollFileSource::new(&source_path);
    file_source.modified_since = read_last_indexed(index_path);

    let snapshots = match file_source.scan() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error scanning webcache: {e}");
            std::process::exit(1);
        }
    };

    let index = match open_or_create_index(index_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Error opening index: {e}");
            std::process::exit(1);
        }
    };

    let (_schema, fields) = build_schema();
    let mut writer = match index.writer(50_000_000) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Error creating index writer: {e}");
            std::process::exit(1);
        }
    };

    let count = snapshots.len();
    match index_snapshots(&mut writer, &fields, &index, snapshots) {
        Ok(indexed) => {
            write_last_indexed(index_path);
            println!("Scanned {count} files, indexed {indexed} new documents");
        }
        Err(e) => {
            eprintln!("Error indexing: {e}");
            std::process::exit(1);
        }
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
    let index_path = expand_tilde(&cli.index_dir);

    match cli.command {
        Commands::Index { source } => {
            run_index(&source, &index_path);
        }
        Commands::Reindex { source } => {
            if index_path.exists() {
                if let Err(e) = std::fs::remove_dir_all(&index_path) {
                    eprintln!("Error removing old index: {e}");
                    std::process::exit(1);
                }
                println!("Removed old index at {}", index_path.display());
            }
            run_index(&source, &index_path);
        }
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
                    println!("  URL:    {}", result.url);
                    println!("  Domain: {}", result.domain);
                    println!("  Score:  {:.2}", result.score);
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
