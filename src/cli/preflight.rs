//! LLM Preflight Configuration
//!
//! First-run LLM configuration wizard.
//! Detects missing/invalid config, prompts user, writes config.toml.
//!
//! NO async, NO network calls, NO provider validation.

use crate::cli::{Error, Result};
use std::io::{self, Write};
use std::path::Path;

/// Outcome of LLM preflight check
#[derive(Debug, Clone, PartialEq)]
pub enum PreflightOutcome {
    /// Continue with normal operation
    Proceed,
    /// Exit cleanly (config written, requires restart)
    Exit,
}

/// Config file location: <db_root>/config.toml
const CONFIG_FILE: &str = "config.toml";

/// Run LLM preflight check
///
/// # Returns
/// * `Ok(PreflightOutcome)` - Proceed or Exit
/// * `Err(Error)` - Fatal error (I/O failure)
///
/// # Behavior
/// - If config missing or invalid: prompt user, write config
/// - If valid config exists: return Proceed immediately
/// - If disabled config exists: return Proceed immediately
pub fn run_llm_preflight(db_root: &Path) -> Result<PreflightOutcome> {
    let config_path = db_root.join(CONFIG_FILE);

    // Check if config exists
    if !config_path.exists() {
        return run_preflight_wizard(db_root, &config_path);
    }

    // Config exists - validate it
    match validate_config(&config_path) {
        Ok(()) => {
            // Valid config - no prompt needed
            Ok(PreflightOutcome::Proceed)
        }
        Err(e) => {
            // Invalid config - prompt for recovery
            handle_invalid_config(db_root, &config_path, e)
        }
    }
}

/// Run the preflight wizard for first-time setup
fn run_preflight_wizard(db_root: &Path, config_path: &Path) -> Result<PreflightOutcome> {
    println!("No LLM configuration found.");
    println!();
    println!("Do you want to use an LLM with OdinCode?");
    println!("  [1] Yes — external API provider");
    println!("  [2] Yes — local model (ollama / llama.cpp / vLLM)");
    println!("  [3] No — continue without LLM");
    println!();
    print!("Choice: ");
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).map_err(Error::Io)?;

    match choice.trim() {
        "1" => configure_external_provider(db_root, config_path),
        "2" => configure_local_provider(db_root, config_path),
        "3" => configure_disabled(db_root, config_path),
        _ => {
            eprintln!("Invalid choice. Defaulting to continue without LLM.");
            configure_disabled(db_root, config_path)
        }
    }
}

/// Configure external API provider
fn configure_external_provider(db_root: &Path, config_path: &Path) -> Result<PreflightOutcome> {
    println!();
    println!("Configuring external API provider...");
    println!();

    // Provider type
    print!("Provider type (glm/openai-compatible/other): ");
    io::stdout().flush()?;
    let mut provider = String::new();
    io::stdin().read_line(&mut provider)?;
    let provider = provider.trim();

    // Base URL
    print!("Base URL: ");
    io::stdout().flush()?;
    let mut base_url = String::new();
    io::stdin().read_line(&mut base_url)?;
    let base_url = base_url.trim();

    // Phase 4.2: Ask how to store API key
    println!();
    println!("How do you want to store your API key on this machine?");
    println!("  [1] Store directly in config file [DEFAULT]");
    println!("  [2] Use environment variable");
    println!("  [3] Disable LLM");
    println!();
    print!("Choice [1]: ");
    io::stdout().flush()?;
    let mut storage_choice = String::new();
    io::stdin().read_line(&mut storage_choice)?;
    let storage_choice = storage_choice.trim();

    // Empty input means default (option 1)
    let storage_choice = if storage_choice.is_empty() {
        "1"
    } else {
        storage_choice
    };

    match storage_choice {
        "1" => {
            // Direct storage - ask for API key literal
            print!("API key: ");
            io::stdout().flush()?;
            let mut api_key = String::new();
            io::stdin().read_line(&mut api_key)?;
            let api_key = api_key.trim();

            // Empty key is invalid - exit with error
            if api_key.is_empty() {
                eprintln!("API key cannot be empty for direct storage.");
                return Ok(PreflightOutcome::Exit);
            }

            // Model name
            print!("Model name: ");
            io::stdout().flush()?;
            let mut model = String::new();
            io::stdin().read_line(&mut model)?;
            let model = model.trim();

            // Write config with literal API key
            let config_content = format!(
                r#"[llm]
mode = "external"
provider = "{}"
base_url = "{}"
api_key = "{}"
model = "{}"
"#,
                provider, base_url, api_key, model
            );

            std::fs::write(config_path, config_content).map_err(Error::Io)?;
            log_preflight_decision(db_root, "external_direct", provider);

            println!();
            println!("Configuration written to: {}", config_path.display());
            println!();
            println!("Please restart OdinCode to apply the configuration.");

            Ok(PreflightOutcome::Exit)
        }
        "2" => {
            // Env var storage - ask for env var name
            print!("Environment variable name [ODINCODE_LLM_API_KEY]: ");
            io::stdout().flush()?;
            let mut env_var = String::new();
            io::stdin().read_line(&mut env_var)?;
            let env_var = env_var.trim();

            // Empty input means default
            let env_var = if env_var.is_empty() {
                "ODINCODE_LLM_API_KEY"
            } else {
                env_var
            };

            // Model name
            print!("Model name: ");
            io::stdout().flush()?;
            let mut model = String::new();
            io::stdin().read_line(&mut model)?;
            let model = model.trim();

            // Write config with env reference
            let config_content = format!(
                r#"[llm]
mode = "external"
provider = "{}"
base_url = "{}"
api_key = "env:{}"
model = "{}"
"#,
                provider, base_url, env_var, model
            );

            std::fs::write(config_path, config_content).map_err(Error::Io)?;
            log_preflight_decision(db_root, "external_env", provider);

            println!();
            println!("Configuration written to: {}", config_path.display());
            println!();
            println!("Before using OdinCode, set the API key:");
            println!("  export {}=<your-api-key>", env_var);
            println!();
            println!("Please restart OdinCode to apply the configuration.");

            Ok(PreflightOutcome::Exit)
        }
        "3" => {
            // User changed mind - disable LLM
            configure_disabled(db_root, config_path)
        }
        _ => {
            eprintln!("Invalid choice. Defaulting to direct storage.");
            // Fall through to direct storage with prompts
            print!("API key: ");
            io::stdout().flush()?;
            let mut api_key = String::new();
            io::stdin().read_line(&mut api_key)?;
            let api_key = api_key.trim();

            if api_key.is_empty() {
                eprintln!("API key cannot be empty.");
                return Ok(PreflightOutcome::Exit);
            }

            print!("Model name: ");
            io::stdout().flush()?;
            let mut model = String::new();
            io::stdin().read_line(&mut model)?;
            let model = model.trim();

            let config_content = format!(
                r#"[llm]
mode = "external"
provider = "{}"
base_url = "{}"
api_key = "{}"
model = "{}"
"#,
                provider, base_url, api_key, model
            );

            std::fs::write(config_path, config_content).map_err(Error::Io)?;
            log_preflight_decision(db_root, "external_direct", provider);

            println!();
            println!("Configuration written to: {}", config_path.display());
            println!("Please restart OdinCode to apply the configuration.");

            Ok(PreflightOutcome::Exit)
        }
    }
}

/// Configure local model provider
fn configure_local_provider(db_root: &Path, config_path: &Path) -> Result<PreflightOutcome> {
    println!();
    println!("Configuring local model provider...");
    println!();

    // Backend
    print!("Backend (ollama/llama.cpp/vllm): ");
    io::stdout().flush()?;
    let mut backend = String::new();
    io::stdin().read_line(&mut backend)?;
    let backend = backend.trim();

    // Host (default to 127.0.0.1)
    print!("Host (default: 127.0.0.1): ");
    io::stdout().flush()?;
    let mut host = String::new();
    io::stdin().read_line(&mut host)?;
    let host = host.trim();
    let host = if host.is_empty() { "127.0.0.1" } else { host };

    // Port
    print!("Port: ");
    io::stdout().flush()?;
    let mut port = String::new();
    io::stdin().read_line(&mut port)?;
    let port = port.trim();

    // Model name
    print!("Model name: ");
    io::stdout().flush()?;
    let mut model = String::new();
    io::stdin().read_line(&mut model)?;
    let model = model.trim();

    // Write config
    let config_content = format!(
        r#"[llm]
mode = "local"
backend = "{}"
host = "{}"
port = "{}"
model = "{}"
"#,
        backend, host, port, model
    );

    std::fs::write(config_path, config_content).map_err(Error::Io)?;

    // Log to execution memory
    log_preflight_decision(db_root, "local", backend);

    println!();
    println!("Configuration written to: {}", config_path.display());
    println!();
    println!("Please restart OdinCode to apply the configuration.");

    Ok(PreflightOutcome::Exit)
}

/// Configure disabled mode
fn configure_disabled(db_root: &Path, config_path: &Path) -> Result<PreflightOutcome> {
    println!();

    let config_content = r#"[llm]
mode = "disabled"
"#;

    std::fs::write(config_path, config_content).map_err(Error::Io)?;

    // Log to execution memory
    log_preflight_decision(db_root, "disabled", "none");

    println!(
        "LLM disabled. You can enable it later by editing: {}",
        config_path.display()
    );

    Ok(PreflightOutcome::Proceed)
}

/// Handle invalid existing config
fn handle_invalid_config(
    db_root: &Path,
    config_path: &Path,
    error: String,
) -> Result<PreflightOutcome> {
    eprintln!("Invalid LLM configuration: {}", error);
    eprintln!();
    eprintln!("  [1] Continue without LLM");
    eprintln!("  [2] Exit and edit config file");
    eprintln!();
    print!("Choice: ");
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).map_err(Error::Io)?;

    match choice.trim() {
        "1" => {
            // Continue without LLM - update config to disabled
            let config_content = r#"[llm]
mode = "disabled"
"#;
            std::fs::write(config_path, config_content).map_err(Error::Io)?;
            log_preflight_decision(db_root, "disabled", "invalid_config_recovery");
            Ok(PreflightOutcome::Proceed)
        }
        "2" => {
            println!();
            println!("Edit the config file at: {}", config_path.display());
            println!("Then restart OdinCode.");
            Ok(PreflightOutcome::Exit)
        }
        _ => {
            eprintln!("Invalid choice. Exiting.");
            Ok(PreflightOutcome::Exit)
        }
    }
}

/// Validate existing config file
///
/// # Returns
/// * `Ok(())` - Config is valid
/// * `Err(String)` - Config is invalid with reason
fn validate_config(config_path: &Path) -> std::result::Result<(), String> {
    let content = std::fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    // Parse as TOML (basic validation)
    let mut has_llm_section = false;
    let mut has_mode = false;

    for line in content.lines() {
        let line = line.trim();
        if line == "[llm]" {
            has_llm_section = true;
        } else if line.starts_with("mode = ") {
            has_mode = true;
        }
    }

    if !has_llm_section {
        return Err("Missing [llm] section".to_string());
    }

    if !has_mode {
        return Err("Missing mode field".to_string());
    }

    // If mode is external or local, check required fields
    if content.contains("mode = \"external\"") {
        if !content.contains("provider = ") {
            return Err("External mode requires 'provider' field".to_string());
        }
        if !content.contains("base_url = ") {
            return Err("External mode requires 'base_url' field".to_string());
        }
        if !content.contains("model = ") {
            return Err("External mode requires 'model' field".to_string());
        }
    }

    if content.contains("mode = \"local\"") {
        if !content.contains("backend = ") {
            return Err("Local mode requires 'backend' field".to_string());
        }
        if !content.contains("model = ") {
            return Err("Local mode requires 'model' field".to_string());
        }
    }

    Ok(())
}

/// Log preflight decision to execution memory
///
/// Creates a single execution artifact with artifact_type = "llm_preflight"
fn log_preflight_decision(db_root: &Path, mode: &str, provider: &str) {
    use crate::execution_tools::ExecutionDb;

    let exec_db = match ExecutionDb::open(db_root) {
        Ok(db) => db,
        Err(_) => return, // Silently skip if we can't log
    };

    let decision = format!("{}|{}", mode, provider);

    // Generate execution ID from decision (deterministic)
    let exec_id = format!("llm_preflight_{}", {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        decision.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    });

    // Current timestamp (milliseconds since UNIX epoch)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    // Build arguments JSON
    let arguments = serde_json::json!({
        "decision": decision,
        "mode": mode,
        "provider": provider,
    });

    // Build artifact content
    let artifact_content = serde_json::json!({
        "decision": decision,
        "mode": mode,
        "provider": provider,
    });

    // Record execution with artifact in one call
    let _ = exec_db.record_execution_with_artifacts(
        &exec_id,
        "llm_preflight",
        &arguments,
        timestamp,
        true,    // succeeded
        None,    // exit_code
        Some(0), // duration_ms
        None,    // error_message
        &[("llm_preflight", &artifact_content)],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_config_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let result = validate_config(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_missing_llm_section() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        std::fs::write(&config_path, "[other]\nkey = \"value\"\n").unwrap();

        let result = validate_config(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("llm"));
    }

    #[test]
    fn test_validate_config_missing_mode() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        std::fs::write(&config_path, "[llm]\n").unwrap();

        let result = validate_config(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mode"));
    }

    #[test]
    fn test_validate_config_valid_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        std::fs::write(&config_path, "[llm]\nmode = \"disabled\"\n").unwrap();

        let result = validate_config(&config_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_valid_external() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        std::fs::write(
            &config_path,
            "[llm]\nmode = \"external\"\nprovider = \"openai\"\nbase_url = \"http://localhost\"\nmodel = \"gpt-4\"\n"
        ).unwrap();

        let result = validate_config(&config_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_external_missing_fields() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        std::fs::write(&config_path, "[llm]\nmode = \"external\"\n").unwrap();

        let result = validate_config(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_valid_local() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        std::fs::write(
            &config_path,
            "[llm]\nmode = \"local\"\nbackend = \"ollama\"\nmodel = \"codellama\"\n",
        )
        .unwrap();

        let result = validate_config(&config_path);
        assert!(result.is_ok());
    }
}
