//! OdinCode API Module
//!
//! The API module provides HTTP endpoints for the OdinCode system,
//! allowing integration with IDEs, editors, and other development tools.

pub mod handlers;
pub mod models;
pub mod server;

pub use handlers::*;
pub use models::*;
pub use server::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_api_config_creation() {
        let config = ApiConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: 4,
            version: "1.0.0".to_string(),
        };

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
    }
}
