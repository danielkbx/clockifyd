#![allow(dead_code)]

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use crate::error::CfdError;
use crate::types::{RoundingMode, StoredConfig};

const APP_DIR: &str = "cfd";
const CONFIG_FILE: &str = "config.json";

pub fn get_config() -> Result<StoredConfig, CfdError> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(StoredConfig::default());
    }

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    if contents.trim().is_empty() {
        return Ok(StoredConfig::default());
    }

    Ok(serde_json::from_str(&contents)?)
}

pub fn save_config(config: &StoredConfig) -> Result<(), CfdError> {
    let path = config_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| CfdError::message("failed to resolve config directory"))?;

    fs::create_dir_all(parent)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&path)?;
        let json = serde_json::to_string_pretty(config)?;
        file.write_all(json.as_bytes())?;
        file.write_all(b"\n")?;
    }

    #[cfg(not(unix))]
    {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)?;
        let json = serde_json::to_string_pretty(config)?;
        file.write_all(json.as_bytes())?;
        file.write_all(b"\n")?;
    }

    Ok(())
}

pub fn clear_config() -> Result<(), CfdError> {
    let path = config_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn resolve_api_key(config: &StoredConfig) -> Result<String, CfdError> {
    match std::env::var("CLOCKIFY_API_KEY") {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => config
            .api_key
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| CfdError::message("missing Clockify API key")),
    }
}

pub fn resolve_workspace(
    explicit_workspace: Option<&str>,
    config: &StoredConfig,
) -> Result<String, CfdError> {
    if let Some(workspace) = explicit_workspace.filter(|value| !value.trim().is_empty()) {
        return Ok(workspace.to_owned());
    }

    match std::env::var("CFD_WORKSPACE") {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => config
            .workspace
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                CfdError::message("missing workspace; set --workspace, CFD_WORKSPACE, or config")
            }),
    }
}

pub fn resolve_project(
    explicit_project: Option<&str>,
    config: &StoredConfig,
) -> Result<String, CfdError> {
    explicit_project
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .or_else(|| {
            config
                .project
                .clone()
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or_else(|| CfdError::message("missing project; set --project or config"))
}

pub fn resolve_rounding(
    no_rounding: bool,
    config: &StoredConfig,
) -> Result<RoundingMode, CfdError> {
    if no_rounding {
        return Ok(RoundingMode::Off);
    }

    match std::env::var("CFD_ROUNDING") {
        Ok(value) if !value.trim().is_empty() => parse_rounding_mode(&value),
        _ => Ok(config.rounding.unwrap_or(RoundingMode::Off)),
    }
}

fn config_path() -> Result<PathBuf, CfdError> {
    if let Ok(path) = std::env::var("CFD_CONFIG") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        let trimmed = xdg_config_home.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed).join(APP_DIR).join(CONFIG_FILE));
        }
    }

    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| CfdError::message("failed to resolve config path: HOME is not set"))?;

    Ok(home.join(".config").join(APP_DIR).join(CONFIG_FILE))
}

pub fn parse_rounding_mode(value: &str) -> Result<RoundingMode, CfdError> {
    match value.trim() {
        "off" => Ok(RoundingMode::Off),
        "1m" => Ok(RoundingMode::OneMinute),
        "5m" => Ok(RoundingMode::FiveMinutes),
        "10m" => Ok(RoundingMode::TenMinutes),
        "15m" => Ok(RoundingMode::FifteenMinutes),
        other => Err(CfdError::message(format!("invalid rounding mode: {other}"))),
    }
}

#[cfg(test)]
pub(crate) fn test_env_mutex() -> &'static Mutex<()> {
    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_MUTEX.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn set_var(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_var(key: &str) {
        unsafe { std::env::remove_var(key) };
    }

    struct TestEnvGuard {
        keys: Vec<&'static str>,
    }

    impl TestEnvGuard {
        fn new() -> Self {
            let keys = vec![
                "CFD_CONFIG",
                "CLOCKIFY_API_KEY",
                "CFD_WORKSPACE",
                "CFD_ROUNDING",
                "XDG_CONFIG_HOME",
                "HOME",
            ];

            for key in &keys {
                remove_var(key);
            }

            Self { keys }
        }
    }

    impl Drop for TestEnvGuard {
        fn drop(&mut self) {
            for key in &self.keys {
                remove_var(key);
            }
        }
    }

    #[test]
    fn save_get_and_clear_config_round_trip() {
        let _lock = test_env_mutex().lock().unwrap();
        let _guard = TestEnvGuard::new();
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("state.json");

        set_var("CFD_CONFIG", config_path.to_str().unwrap());

        let config = StoredConfig {
            api_key: Some("secret".into()),
            workspace: Some("ws1".into()),
            rounding: Some(RoundingMode::TenMinutes),
            project: Some("pr1".into()),
        };

        save_config(&config).unwrap();

        let loaded = get_config().unwrap();
        assert_eq!(loaded, config);

        clear_config().unwrap();
        assert_eq!(get_config().unwrap(), StoredConfig::default());
    }

    #[test]
    fn cfd_config_overrides_default_path() {
        let _lock = test_env_mutex().lock().unwrap();
        let _guard = TestEnvGuard::new();
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("nested").join("custom.json");

        set_var("CFD_CONFIG", config_path.to_str().unwrap());

        save_config(&StoredConfig {
            workspace: Some("ws1".into()),
            ..StoredConfig::default()
        })
        .unwrap();

        assert!(config_path.exists());
        assert_eq!(get_config().unwrap().workspace.as_deref(), Some("ws1"));
    }

    #[test]
    fn api_key_resolution_prefers_env_then_config() {
        let _lock = test_env_mutex().lock().unwrap();
        let _guard = TestEnvGuard::new();
        let config = StoredConfig {
            api_key: Some("stored-key".into()),
            ..StoredConfig::default()
        };

        assert_eq!(resolve_api_key(&config).unwrap(), "stored-key");

        set_var("CLOCKIFY_API_KEY", "env-key");
        assert_eq!(resolve_api_key(&config).unwrap(), "env-key");
    }

    #[test]
    fn workspace_resolution_prefers_explicit_then_env_then_config() {
        let _lock = test_env_mutex().lock().unwrap();
        let _guard = TestEnvGuard::new();
        let config = StoredConfig {
            workspace: Some("stored-ws".into()),
            ..StoredConfig::default()
        };

        assert_eq!(resolve_workspace(None, &config).unwrap(), "stored-ws");

        set_var("CFD_WORKSPACE", "env-ws");
        assert_eq!(resolve_workspace(None, &config).unwrap(), "env-ws");
        assert_eq!(
            resolve_workspace(Some("cli-ws"), &config).unwrap(),
            "cli-ws"
        );
    }

    #[test]
    fn project_resolution_prefers_explicit_then_config() {
        let _lock = test_env_mutex().lock().unwrap();
        let _guard = TestEnvGuard::new();
        let config = StoredConfig {
            project: Some("stored-project".into()),
            ..StoredConfig::default()
        };

        assert_eq!(resolve_project(None, &config).unwrap(), "stored-project");
        assert_eq!(
            resolve_project(Some("explicit-project"), &config).unwrap(),
            "explicit-project"
        );
    }

    #[test]
    fn rounding_resolution_prefers_no_rounding_then_env_then_config_then_off() {
        let _lock = test_env_mutex().lock().unwrap();
        let _guard = TestEnvGuard::new();
        let config = StoredConfig {
            rounding: Some(RoundingMode::FifteenMinutes),
            ..StoredConfig::default()
        };

        assert_eq!(
            resolve_rounding(false, &StoredConfig::default()).unwrap(),
            RoundingMode::Off
        );
        assert_eq!(
            resolve_rounding(false, &config).unwrap(),
            RoundingMode::FifteenMinutes
        );

        set_var("CFD_ROUNDING", "5m");
        assert_eq!(
            resolve_rounding(false, &config).unwrap(),
            RoundingMode::FiveMinutes
        );
        assert_eq!(resolve_rounding(true, &config).unwrap(), RoundingMode::Off);
    }

    #[test]
    fn unix_save_uses_private_permissions() {
        let _lock = test_env_mutex().lock().unwrap();
        let _guard = TestEnvGuard::new();
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");

        set_var("CFD_CONFIG", config_path.to_str().unwrap());
        save_config(&StoredConfig::default()).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = fs::metadata(config_path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }
}
