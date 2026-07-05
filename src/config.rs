/// Configuration loaded from environment variables (mirrors config.ts)
pub struct Config {
    pub port: u16,
    pub allowed_origins: Vec<String>,
    pub board_size: usize,
}

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3001);

        let allowed_origins = std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let board_size = std::env::var("BOARD_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(15);

        Config {
            port,
            allowed_origins,
            board_size,
        }
    }
}

/// Global board size (set once at startup)
static BOARD_SIZE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

pub fn board_size() -> usize {
    *BOARD_SIZE.get_or_init(|| {
        std::env::var("BOARD_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(15)
    })
}
