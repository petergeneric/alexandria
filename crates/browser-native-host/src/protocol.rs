use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum IncomingMessage {
    #[serde(rename = "snapshot")]
    Snapshot {
        url: String,
        title: String,
        html: String,
        #[allow(dead_code)]
        content_type: Option<String>,
        timestamp: Option<i64>,
    },
    #[serde(rename = "chunk")]
    Chunk {
        id: String,
        seq: usize,
        total: usize,
        data: String,
        meta: Option<ChunkMeta>,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "get_blocklist")]
    GetBlocklist,
}

#[derive(Debug, Deserialize)]
pub struct ChunkMeta {
    pub url: String,
    pub title: String,
    #[allow(dead_code)]
    pub content_type: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct HostResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<Vec<String>>,
}

impl HostResponse {
    pub fn ok() -> Self {
        Self {
            status: "ok".into(),
            message: None,
            version: None,
            blocked_domains: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            status: "error".into(),
            message: Some(msg.into()),
            version: None,
            blocked_domains: None,
        }
    }

    pub fn pong() -> Self {
        Self {
            status: "ok".into(),
            message: None,
            version: Some("1.0".into()),
            blocked_domains: None,
        }
    }

    pub fn blocklist(domains: Vec<String>) -> Self {
        Self {
            status: "ok".into(),
            message: None,
            version: None,
            blocked_domains: Some(domains),
        }
    }
}

const MAX_REASSEMBLED_SIZE: usize = 5 * 1024 * 1024; // 5MB

pub struct ChunkAssembler {
    buffers: HashMap<String, Vec<Option<String>>>,
}

impl ChunkAssembler {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    /// Add a chunk. Returns Some(assembled snapshot fields) when all chunks for an id are received.
    pub fn add_chunk(
        &mut self,
        id: String,
        seq: usize,
        total: usize,
        data: String,
        meta: Option<ChunkMeta>,
    ) -> Result<Option<(String, ChunkMeta)>, String> {
        let buf = self.buffers.entry(id.clone()).or_insert_with(|| vec![None; total]);

        if buf.len() != total {
            return Err(format!("chunk total mismatch for id {id}: expected {}, got {total}", buf.len()));
        }
        if seq >= total {
            return Err(format!("chunk seq {seq} out of range for total {total}"));
        }

        buf[seq] = Some(data);

        // Check if complete
        if buf.iter().all(|s| s.is_some()) {
            let chunks = self.buffers.remove(&id).unwrap();
            let total_size: usize = chunks.iter().map(|c| c.as_ref().unwrap().len()).sum();
            if total_size > MAX_REASSEMBLED_SIZE {
                return Err(format!("reassembled size {total_size} exceeds limit"));
            }
            let assembled: String = chunks.into_iter().map(|c| c.unwrap()).collect();

            let meta = meta.ok_or_else(|| "final chunk missing meta".to_string())?;
            Ok(Some((assembled, meta)))
        } else {
            Ok(None)
        }
    }
}
