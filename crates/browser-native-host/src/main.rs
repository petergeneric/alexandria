mod protocol;

use alexandria_core::extract;
use alexandria_core::filter;
use alexandria_core::page_store::PageStore;
use xxhash_rust::xxh3::xxh3_128;
use protocol::{ChunkAssembler, HostResponse, IncomingMessage};
use std::collections::{HashSet, VecDeque};
use std::io::{self, Read, Write};
use std::path::PathBuf;

fn store_path() -> PathBuf {
    let base = dirs_base().join("pages.db");
    base
}

fn dirs_base() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library/Application Support/works.peter.alexandria")
}

fn content_hash(input: &[u8]) -> [u8; 16] {
    xxh3_128(input).to_le_bytes()
}

struct DedupCache {
    ring: VecDeque<[u8; 16]>,
    set: HashSet<[u8; 16]>,
    capacity: usize,
}

impl DedupCache {
    fn new(capacity: usize) -> Self {
        Self {
            ring: VecDeque::with_capacity(capacity),
            set: HashSet::with_capacity(capacity),
            capacity,
        }
    }

    /// Populate cache from existing hashes (oldest first).
    fn populate(&mut self, hashes: Vec<[u8; 16]>) {
        for hash in hashes {
            if self.ring.len() >= self.capacity {
                self.ring.pop_front();
            }
            self.ring.push_back(hash);
            self.set.insert(hash);
        }
    }

    /// Returns true if the hash was already in the cache (duplicate).
    /// If not, inserts it and returns false.
    fn check_and_insert(&mut self, hash: [u8; 16]) -> bool {
        if self.set.contains(&hash) {
            return true;
        }
        if self.ring.len() >= self.capacity {
            self.ring.pop_front();
        }
        self.ring.push_back(hash);
        self.set.insert(hash);
        // Rebuild set from ring when it grows too large from stale entries
        if self.set.len() > self.capacity * 2 {
            self.set = self.ring.iter().copied().collect();
        }
        false
    }
}

fn read_message(stdin: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match stdin.read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 {
        return Ok(Some(Vec::new()));
    }
    let mut buf = vec![0u8; len];
    stdin.read_exact(&mut buf)?;
    Ok(Some(buf))
}

fn write_message(stdout: &mut impl Write, response: &HostResponse) -> io::Result<()> {
    let json = serde_json::to_vec(response).unwrap();
    let len = (json.len() as u32).to_le_bytes();
    stdout.write_all(&len)?;
    stdout.write_all(&json)?;
    stdout.flush()?;
    Ok(())
}

fn handle_snapshot(
    store: &PageStore,
    dedup: &mut DedupCache,
    url: &str,
    title: &str,
    html: &str,
    timestamp: Option<i64>,
) -> HostResponse {
    // Dedup on page content — same URL may produce different HTML over time
    let hash = content_hash(html.as_bytes());
    if dedup.check_and_insert(hash) {
        return HostResponse::ok();
    }

    let domain = extract::extract_domain(url);
    let site_group = extract::extract_site_group(url);
    let captured_at = timestamp.unwrap_or_else(|| chrono::Utc::now().timestamp());

    // Sites with filter rules: store raw HTML (needs '<' prefix for detection).
    // All other sites: store plaintext to save space (~74% smaller).
    let content = if filter::has_filter(&domain) {
        if html.starts_with('<') {
            html.to_string()
        } else {
            format!("<!doctype html>{html}")
        }
    } else {
        let plaintext = extract::html_to_plaintext(html);
        if plaintext.starts_with('<') {
            format!(" {plaintext}")
        } else {
            plaintext
        }
    };

    match store.insert(url, title, content.as_bytes(), &domain, &site_group, captured_at, &hash) {
        Ok(()) => HostResponse::ok(),
        Err(e) => HostResponse::error(e.to_string()),
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let db_path = store_path();
    let store = match PageStore::open(&db_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open page store at {}: {e}", db_path.display());
            std::process::exit(1);
        }
    };

    tracing::info!("Native host started, store at {}", db_path.display());

    let mut dedup = DedupCache::new(1000);
    match store.recent_content_hashes(1000) {
        Ok(hashes) => {
            tracing::info!("Loaded {} content hashes from store", hashes.len());
            dedup.populate(hashes);
        }
        Err(e) => tracing::warn!("Failed to load content hashes: {e}"),
    }

    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    let mut assembler = ChunkAssembler::new();

    loop {
        let msg_bytes = match read_message(&mut stdin) {
            Ok(Some(b)) => b,
            Ok(None) => break, // EOF
            Err(e) => {
                tracing::error!("Failed to read message: {e}");
                break;
            }
        };

        let msg: IncomingMessage = match serde_json::from_slice(&msg_bytes) {
            Ok(m) => m,
            Err(e) => {
                let _ = write_message(&mut stdout, &HostResponse::error(format!("parse error: {e}")));
                continue;
            }
        };

        let response = match msg {
            IncomingMessage::Ping => HostResponse::pong(),
            IncomingMessage::Snapshot {
                url,
                title,
                html,
                timestamp,
                ..
            } => handle_snapshot(&store, &mut dedup, &url, &title, &html, timestamp),
            IncomingMessage::Chunk {
                id,
                seq,
                total,
                data,
                meta,
            } => match assembler.add_chunk(id, seq, total, data, meta) {
                Ok(Some((html, meta))) => {
                    handle_snapshot(&store, &mut dedup, &meta.url, &meta.title, &html, meta.timestamp)
                }
                Ok(None) => HostResponse::ok(),
                Err(e) => HostResponse::error(e),
            },
        };

        if let Err(e) = write_message(&mut stdout, &response) {
            tracing::error!("Failed to write response: {e}");
            break;
        }
    }
}
