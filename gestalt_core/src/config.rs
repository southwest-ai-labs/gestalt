use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    pub url: String,
    pub user: String,
    #[serde(skip_serializing, default)]
    pub pass: String,
    pub namespace: String,
    pub database: String,
}

impl fmt::Debug for DatabaseSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseSettings")
            .field("url", &self.url)
            .field("user", &self.user)
            .field("pass", &"[REDACTED]")
            .field("namespace", &self.namespace)
            .field("database", &self.database)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_settings_debug_redacts_password() {
        let settings = DatabaseSettings {
            url: "localhost:8000".to_string(),
            user: "root".to_string(),
            pass: "super_secret_password".to_string(),
            namespace: "gestalt".to_string(),
            database: "test".to_string(),
        };

        let debug_str = format!("{:?}", settings);
        assert!(!debug_str.contains("super_secret_password"));
        assert!(debug_str.contains("[REDACTED]"));
    }

    #[test]
    fn test_database_settings_serialization_skips_pass() {
        let settings = DatabaseSettings {
            url: "localhost:8000".to_string(),
            user: "root".to_string(),
            pass: "super_secret_password".to_string(),
            namespace: "gestalt".to_string(),
            database: "test".to_string(),
        };

        let json = serde_json::to_string(&settings).unwrap();
        assert!(!json.contains("super_secret_password"));
        assert!(!json.contains("\"pass\""));

        // Deserializing without pass field defaults to empty string
        let deserialized: DatabaseSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.pass, "");

        // Deserializing with pass field populates pass
        let json_with_pass = r#"{"url":"localhost:8000","user":"root","pass":"custom_pass","namespace":"gestalt","database":"test"}"#;
        let deserialized_with_pass: DatabaseSettings = serde_json::from_str(json_with_pass).unwrap();
        assert_eq!(deserialized_with_pass.pass, "custom_pass");
    }
}

