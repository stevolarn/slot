//! The LaunchBox Games Database over HTTPS.

use std::io::{self, Read};
use std::time::Duration;

use crate::zip::Source;

pub const METADATA_URL: &str = "https://gamesdb.launchbox-app.com/Metadata.zip";
pub const IMAGES_URL: &str = "https://images.launchbox-app.com/";
pub const USER_AGENT: &str = "Shelf-fetch-art/1.0 (+https://github.com/stevolarn/slot)";

/// Why a download failed.
#[derive(Debug)]
pub enum NetError {
    /// The server asked for fewer requests (HTTP 429): stop for now.
    Busy,
    /// Not there (HTTP 404 and the like): skip it.
    Missing(u16),
    /// No connection, or it broke.
    Down(String),
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::Busy => write!(f, "the server is busy"),
            NetError::Missing(code) => write!(f, "HTTP {code}"),
            NetError::Down(e) => write!(f, "{e}"),
        }
    }
}

/// Where the metadata and the pictures come from: the web, or files in
/// tests.
pub trait Net: Send + Sync {
    /// The metadata zip, read in ranges.
    fn metadata(&self) -> Box<dyn Source + Send>;
    /// A picture by its LaunchBox file name.
    fn picture(&self, file: &str) -> Result<Vec<u8>, NetError>;
}

/// The real LaunchBox, over HTTPS.
pub struct Web {
    agent: ureq::Agent,
}

impl Default for Web {
    fn default() -> Self {
        Web {
            agent: ureq::AgentBuilder::new()
                .user_agent(USER_AGENT)
                .timeout_connect(Duration::from_secs(15))
                .timeout_read(Duration::from_secs(60))
                .build(),
        }
    }
}

fn net_error(e: ureq::Error) -> NetError {
    match e {
        ureq::Error::Status(429, _) => NetError::Busy,
        ureq::Error::Status(code, _) => NetError::Missing(code),
        ureq::Error::Transport(t) => NetError::Down(t.to_string()),
    }
}

fn io_error(e: NetError) -> io::Error {
    io::Error::other(e.to_string())
}

struct WebZip {
    agent: ureq::Agent,
}

impl Source for WebZip {
    fn len(&self) -> io::Result<u64> {
        let r = self
            .agent
            .head(METADATA_URL)
            .call()
            .map_err(|e| io_error(net_error(e)))?;
        r.header("Content-Length")
            .and_then(|l| l.parse().ok())
            .ok_or_else(|| io::Error::other("the server did not say how big the metadata is"))
    }

    fn range(&self, start: u64, end: u64) -> io::Result<Box<dyn Read + Send>> {
        let r = self
            .agent
            .get(METADATA_URL)
            .set("Range", &format!("bytes={start}-{}", end.saturating_sub(1)))
            .call()
            .map_err(|e| io_error(net_error(e)))?;
        if r.status() != 206 {
            return Err(io::Error::other("the server does not send parts of the metadata"));
        }
        Ok(Box::new(r.into_reader()))
    }
}

impl Net for Web {
    fn metadata(&self) -> Box<dyn Source + Send> {
        Box::new(WebZip {
            agent: self.agent.clone(),
        })
    }

    fn picture(&self, file: &str) -> Result<Vec<u8>, NetError> {
        let r = self
            .agent
            .get(&format!("{IMAGES_URL}{file}"))
            // A picture that takes longer than this is given up on, so Stop
            // is never kept waiting on a slow one.
            .timeout(Duration::from_secs(90))
            .call()
            .map_err(net_error)?;
        let mut bytes = Vec::new();
        r.into_reader()
            .take(32 << 20)
            .read_to_end(&mut bytes)
            .map_err(|e| NetError::Down(e.to_string()))?;
        Ok(bytes)
    }
}
