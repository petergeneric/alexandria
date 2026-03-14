mod protocol;

use alexandria_core::extract;
use alexandria_core::page_store::PageStore;
use md5::{Digest, Md5};
use protocol::{ChunkAssembler, HostResponse, IncomingMessage};
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

fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
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
    url: &str,
    title: &str,
    html: &str,
    timestamp: Option<i64>,
) -> HostResponse {
    let source_hash = md5_hex(url);
    let domain = extract::extract_domain(url);
    let captured_at = timestamp.unwrap_or_else(|| chrono::Utc::now().timestamp());

    match store.upsert(url, title, html.as_bytes(), &domain, &source_hash, captured_at) {
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
            } => handle_snapshot(&store, &url, &title, &html, timestamp),
            IncomingMessage::Chunk {
                id,
                seq,
                total,
                data,
                meta,
            } => match assembler.add_chunk(id, seq, total, data, meta) {
                Ok(Some((html, meta))) => {
                    handle_snapshot(&store, &meta.url, &meta.title, &html, meta.timestamp)
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
