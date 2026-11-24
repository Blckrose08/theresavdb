//! Database server configuration and runtime.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub max_frame_size: u32,
    pub data_dir: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 7432,
            max_connections: 100,
            max_frame_size: 16 * 1024 * 1024,
            data_dir: PathBuf::from("./data"),
        }
    }
}
