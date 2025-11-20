//! OdinCode - Next-Generation AI Code Engineering System
//!
//! The main entry point for the OdinCode application.

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use odincode_agents::AgentCoordinator;
use odincode_core::llm_integration::LLMIntegrationManager;
use odincode_core::CodeEngine;
#[cfg(not(feature = "simple-ltmc"))]
use odincode_ltmc::LTMManager;

/// Command line interface for OdinCode
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Run in server mode (API)
    #[arg(long, conflicts_with = "tui")]
    server: bool,

    /// Run in TUI mode
    #[arg(long, conflicts_with = "server")]
    tui: bool,

    /// Run in CLI mode (commands)
    #[arg(long, conflicts_with = "server")]
    cli: bool,
}

/// Main application structure
pub struct OdinCodeApp {
    /// The core code engine
    core_engine: Arc<CodeEngine>,
    /// The LTMC manager for persistent learning
    #[cfg(not(feature = "simple-ltmc"))]
    ltmc_manager: Arc<LTMManager>,
    /// The LTMC integration for agent learning
    #[cfg(not(feature = "simple-ltmc"))]
    ltmc_integration: Arc<LTMCIntegration>,
    /// The agent coordinator
    agent_coordinator: AgentCoordinator,
    /// The LLM integration manager
    llm_manager: Arc<LLMIntegrationManager>,
    /// ML integration configuration
    ml_config: odincode_core::ml_integration::MLIntegrationConfig,
}

impl OdinCodeApp {
    /// Create a new OdinCode application instance
    pub async fn new() -> Result<Self> {
        // Create ML integration configuration
        let ml_config = odincode_core::ml_integration::MLIntegrationConfig {
            model_name: "odincode_ml_v1".to_string(),
            confidence_threshold: 0.6,
            max_suggestions: 15,
            use_ltmc_context: cfg!(feature = "simple-ltmc"), // Use LTMC context if simple-ltmc feature enabled
            use_llm: true,
            default_llm_provider: "openai".to_string(),
            default_llm_model: "gpt-4".to_string(),
            llm_max_tokens: 2048,
            llm_temperature: 0.7,
            llm_config: odincode_core::ml_integration::LLMConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                max_tokens: 2048,
                temperature: 0.7,
            },
            model_registry_config: odincode_core::ml_integration::ModelRegistryConfig {
                max_models: 100,
                cache_size_mb: 512,
                max_models_in_memory: 10,
                persist_to_disk: true,
                persistence_directory: "./models".to_string(),
                enable_versioning: true,
                max_versions_per_model: 5,
            },
            performance_tracking_config: odincode_core::ml_integration::PerformanceTrackingConfig {
                enabled: true,
                retention_days: 30,
            },
        };

        // Create LLM integration manager first
        let llm_manager = LLMIntegrationManager::new()?;
        let llm_manager = Arc::new(llm_manager);

        #[cfg(not(feature = "simple-ltmc"))]
        {
            // Full LTMC mode - use external LTMC manager
            let ltmc_manager = Arc::new(LTMManager::new());

            // Create core engine with ML and LLM integration
            let core_engine = Arc::new(
                CodeEngine::new_with_ml_and_llm(
                    ml_config.clone(),
                    Arc::clone(&ltmc_manager),
                    Arc::clone(&llm_manager),
                )
                .await?,
            );

            // Create the LTMC integration
            let ltmc_integration = Arc::new(LTMCIntegration::new(
                Arc::clone(&ltmc_manager),
                Arc::clone(&core_engine),
                Arc::clone(&llm_manager),
            ));

            // Create the agent coordinator with the core engine, LTMC manager, and LTMC integration
            let agent_coordinator = AgentCoordinator::new(
                Arc::clone(&core_engine),
                Arc::clone(&ltmc_manager),
                Arc::clone(&ltmc_integration),
            )
            .await;

            // Now that we have the complete app, let's initialize the semantic analysis engine
            // in the ML integration manager if it exists
            if let Some(ml_integration) = core_engine.get_ml_integration().await {
                // Initialize the neural network model in the semantic analysis engine
                // Note: We need to clone the Arc to avoid borrowing issues
                let ml_integration_clone = Arc::clone(&ml_integration);
                // Lock the mutex to access the semantic analyzer
                let semantic_analyzer_result = {
                    let mut semantic_analyzer = ml_integration_clone.semantic_analyzer.lock().await;
                    semantic_analyzer.initialize_pattern_recognition_model()
                };
                if let Err(e) = semantic_analyzer_result {
                    eprintln!(
                        "Warning: Failed to initialize semantic analysis neural network: {}",
                        e
                    );
                    // Continue execution even if neural network initialization fails
                }
            }

            Ok(Self {
                core_engine,
                ltmc_manager,
                ltmc_integration,
                agent_coordinator,
                llm_manager,
                ml_config,
            })
        }

        #[cfg(feature = "simple-ltmc")]
        {
            // Simple LTMC mode - use dummy LTMC manager since SimpleLTMCManager doesn't exist
            // We'll use the basic LTMManager as a placeholder
            let simple_ltmc_manager = Arc::new(odincode_ltmc::LTMManager::new());

            // Create core engine with simple LTMC integration
            let engine =
                CodeEngine::new_with_ml(ml_config.clone(), Arc::clone(&simple_ltmc_manager))
                    .await?;
            let core_engine = Arc::new(engine);

            // Create a dummy LTMC manager for agent coordinator compatibility
            let dummy_ltmc_manager = Arc::new(odincode_ltmc::LTMManager::new());
            let dummy_ltmc_integration =
                Arc::new(odincode_agents::ltmc_integration::LTMCIntegration::new(
                    Arc::clone(&dummy_ltmc_manager),
                    Arc::clone(&core_engine),
                    Arc::clone(&llm_manager),
                ));

            // Create the agent coordinator with the core engine and dummy managers for compatibility
            let agent_coordinator = AgentCoordinator::new(
                Arc::clone(&core_engine),
                Arc::clone(&dummy_ltmc_manager),
                Arc::clone(&dummy_ltmc_integration),
            );

            // Now that we have the complete app, let's initialize the semantic analysis engine
            // in the ML integration manager if it exists
            {
                if let Some(ml_integration) = core_engine.get_ml_integration().await {
                    // Initialize the neural network model in the semantic analysis engine
                    // Clone the Arc to avoid borrowing issues
                    let ml_integration_clone = Arc::clone(&ml_integration);
                    // Lock the mutex to access the semantic analyzer
                    // TODO: Re-implement semantic analyzer initialization
                    // let semantic_analyzer_result = {
                    //     let mut semantic_analyzer =
                    //         ml_integration_clone.semantic_analyzer.lock().await;
                    //     semantic_analyzer.initialize_pattern_recognition_model()
                    // };
                    // if let Err(e) = semantic_analyzer_result {
                    //     eprintln!(
                    //         "Warning: Failed to initialize semantic analysis neural network: {}",
                    //         e
                    //     );
                    // }
                }
            }

            Ok(Self {
                core_engine,
                agent_coordinator,
                llm_manager,
                ml_config,
            })
        }
    }

    /// Initialize the application
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing OdinCode application...");

        // Initialize LTMC databases with database manager
        let _database_manager = odincode_databases::DatabaseManager::new();
        // We'll initialize the LTMC manager separately since it's in an Arc
        info!("LTMC databases will be initialized separately");

        // TODO: Fix LLM manager connection to ML integration
        // let ml_integration = self.core_engine.get_ml_integration().await;
        // if let Some(ml_integration) = ml_integration {
        //     // Set the LLM integration in the ML integration manager
        //     ml_integration
        //         .set_llm_integration(Arc::clone(&self.llm_manager))
        //         .await;
        //     info!("Connected LLM manager to ML integration");
        // }

        // Register default agents
        self.register_default_agents().await?;
        info!("Default agents registered");

        info!("OdinCode application initialized successfully");
        Ok(())
    }

    /// Register default agents
    async fn register_default_agents(&self) -> Result<()> {
        // Register code generation agent
        self.agent_coordinator
            .register_agent(
                odincode_agents::AgentType::CodeGenerator,
                "Code Generator".to_string(),
                "Generates code based on descriptions or context".to_string(),
                vec![
                    "code_generation".to_string(),
                    "completion".to_string(),
                    "suggestion".to_string(),
                ],
                0.7,
            )
            .await?;

        // Register refactoring agent
        self.agent_coordinator
            .register_agent(
                odincode_agents::AgentType::Refactorer,
                "Refactorer".to_string(),
                "Suggests and applies code refactoring".to_string(),
                vec![
                    "refactoring".to_string(),
                    "optimization".to_string(),
                    "best_practices".to_string(),
                ],
                0.75,
            )
            .await?;

        // Register bug detection agent
        self.agent_coordinator
            .register_agent(
                odincode_agents::AgentType::BugDetector,
                "Bug Detector".to_string(),
                "Detects potential bugs and security issues".to_string(),
                vec!["bug_detection".to_string(), "security".to_string()],
                0.8,
            )
            .await?;

        // Register documentation agent
        self.agent_coordinator
            .register_agent(
                odincode_agents::AgentType::Documenter,
                "Documenter".to_string(),
                "Generates documentation for code".to_string(),
                vec!["documentation".to_string(), "comments".to_string()],
                0.7,
            )
            .await?;

        // Register test generation agent
        self.agent_coordinator
            .register_agent(
                odincode_agents::AgentType::TestGenerator,
                "Test Generator".to_string(),
                "Generates unit and integration tests".to_string(),
                vec!["testing".to_string(), "unit_tests".to_string()],
                0.75,
            )
            .await?;

        // Register code understanding agent
        self.agent_coordinator
            .register_agent(
                odincode_agents::AgentType::CodeUnderstanding,
                "Code Understanding".to_string(),
                "Analyzes and explains code functionality".to_string(),
                vec!["analysis".to_string(), "understanding".to_string()],
                0.8,
            )
            .await?;

        Ok(())
    }

    /// Run the application
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting OdinCode application...");

        // Initialize the application
        self.initialize().await?;

        // Initialize the LTMC manager with database connections
        // Note: This would require mutable access, which we don't have with Arc
        // In a real implementation, we would handle initialization differently

        // Run the enhanced demonstration with ML integration
        self.run_enhanced_demo().await?;

        info!("OdinCode application finished");
        Ok(())
    }

    /// Run a simple demonstration
    async fn run_demo(&self) -> Result<()> {
        info!("Running OdinCode demonstration...");

        // Load a sample file
        let sample_code = r#"
fn main() {
    println!("Hello, OdinCode!");
}
"#
        .to_string();

        let file_id = self
            .core_engine
            .load_file("hello.rs".to_string(), sample_code, "rust".to_string())
            .await?;

        info!("Loaded sample file with ID: {}", file_id);

        // Get all registered agents
        let agents = self.agent_coordinator.get_all_agents().await?;
        info!("Found {} registered agents", agents.len());

        // List agents by type
        let generator_agents = self
            .agent_coordinator
            .list_agents_by_type(odincode_agents::AgentType::CodeGenerator)
            .await?;
        info!("Found {} code generator agents", generator_agents.len());

        // Perform analysis on the file
        if let Some(analysis) = self.core_engine.analyze_file(file_id).await? {
            info!("Completed analysis for file: {}", analysis.id);
            info!("Found {} issues", analysis.issues.len());
            info!("Generated {} suggestions", analysis.suggestions.len());
        }

        // Get all agents again to verify functionality
        let all_agents = self.agent_coordinator.get_all_agents().await?;
        info!("Verified {} agents in system", all_agents.len());

        Ok(())
    }

    /// Run an enhanced demonstration with ML integration and advanced tools
    async fn run_enhanced_demo(&self) -> Result<()> {
        info!("Running OdinCode enhanced demonstration...");

        // Load a sample file with more complex content
        let sample_code = r#"
fn main() {
    let numbers = vec![1, 2, 3, 4, 5];
    let doubled: Vec<i32> = numbers.iter().map(|x| x * 2).collect();
    println!("Doubled numbers: {:?}", doubled);
    
    // TODO: Add error handling
    // FIXME: Optimize this function
}

// This function could be more efficient
fn inefficient_function() -> Vec<i32> {
    let mut result = Vec::new();
    for i in 0..1000 {
        result.push(i * 2);
    }
    result
}
"#
        .to_string();

        let file_id = self
            .core_engine
            .load_file("demo.rs".to_string(), sample_code, "rust".to_string())
            .await?;

        info!("Loaded sample file with ID: {}", file_id);

        // Perform analysis on the file
        if let Some(analysis) = self.core_engine.analyze_file(file_id).await? {
            info!("Completed analysis for file: {}", analysis.id);
            info!("Found {} issues", analysis.issues.len());
            info!("Generated {} suggestions", analysis.suggestions.len());

            // Print out the issues found
            for (i, issue) in analysis.issues.iter().enumerate() {
                info!(
                    "  Issue {}: {:?} - {} at line {}",
                    i + 1,
                    issue.severity,
                    issue.description,
                    issue.line_number
                );
            }
        }

        // Demonstrate ML integration
        self.run_ml_integration_demo(file_id).await?;

        // Demonstrate multi-edit functionality
        self.run_multi_edit_demo(file_id).await?;

        // Demonstrate linter functionality
        self.run_linter_demo(file_id).await?;

        // Get all registered agents
        let agents = self.agent_coordinator.get_all_agents().await?;
        info!("Found {} registered agents", agents.len());

        // Execute a refactoring agent on the file
        for agent in &agents {
            if matches!(agent.agent_type, odincode_agents::AgentType::Refactorer) {
                info!("Executing refactoring agent: {}", agent.name);
                if let Some(suggestions) = self
                    .agent_coordinator
                    .execute_agent_on_file(agent.id, file_id)
                    .await?
                {
                    info!("Agent generated {} suggestions", suggestions.len());
                }
                break; // Just run the first refactoring agent
            }
        }

        Ok(())
    }

    /// Run ML integration demonstration
    async fn run_ml_integration_demo(&self, file_id: Uuid) -> Result<()> {
        info!("Running ML integration demonstration...");

        // Get the ML integration from the core engine
        if let Some(ml_integration) = self.core_engine.get_ml_integration().await {
            info!("ML integration is available, analyzing file with ML models...");

            // Get file content for ML analysis
            if let Some(file) = self.core_engine.get_file(file_id).await? {
                match ml_integration.analyze_with_ml(&file.content).await {
                    Ok(ml_suggestions) => {
                        info!(
                            "ML integration generated {} suggestions",
                            ml_suggestions.len()
                        );
                    }
                    Err(e) => {
                        error!("ML analysis failed: {}", e);
                    }
                }
            } else {
                info!("File not found");
            }
        } else {
            info!("ML integration not available in core engine");
        }

        Ok(())
    }

    /// Run multi-edit demonstration
    async fn run_multi_edit_demo(&self, _file_id: Uuid) -> Result<()> {
        use odincode_tools::{EditOperationType, EditTask};

        info!("Running multi-edit demonstration...");

        // Example: Create a multi-edit operation to fix TODO comments
        let edit_task = EditTask {
            id: uuid::Uuid::new_v4(),
            file_id: _file_id,
            operation_type: EditOperationType::Replace,
            start_pos: (1, 1),
            end_pos: (1, 10),
            content: "Fixed TODO comment".to_string(),
            description: "Fix TODO comments".to_string(),
        };

        info!("Created multi-edit task: {:?}", edit_task);

        Ok(())
    }

    /// Run linter demonstration
    async fn run_linter_demo(&self, _file_id: Uuid) -> Result<()> {
        use odincode_core::{IssueType, Severity};
        use odincode_tools::linters::{LinterConfig, LinterManager};

        info!("Running linter demonstration...");

        // Create a linter manager
        let linter_manager = LinterManager::new(std::sync::Arc::clone(&self.core_engine));

        // Register a Rust-specific linter
        let rust_config = LinterConfig {
            language: "rust".to_string(),
            name: "RustAnalyzer".to_string(),
            description: "Advanced Rust linter".to_string(),
            enabled_rules: vec![
                "trailing_whitespace".to_string(),
                "line_length".to_string(),
                "todo_comments".to_string(),
                "inefficient_patterns".to_string(),
            ],
            disabled_rules: vec![],
            severity_overrides: std::collections::HashMap::new(),
            custom_params: std::collections::HashMap::new(),
        };

        linter_manager.register_linter(rust_config).await?;

        // Note: In a real implementation, we would run the linter on the file
        info!("Linting demonstration completed");

        Ok(())
    }

    /// Run CLI operations
    async fn run_cli_operations(&self) -> Result<()> {
        info!("Running OdinCode CLI operations...");

        // CLI operations would be implemented here
        // For now, we'll just run a simple demo
        self.run_demo().await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("Starting OdinCode - Next-Generation AI Code Engineering System");

    // Parse command line arguments
    let args = Args::parse();

    // Set log level based on verbose flag
    if args.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }

    info!("OdinCode starting with args: {:?}", args);

    // Create the application instance
    let mut app = OdinCodeApp::new().await?;

    // Determine which mode to run based on command line arguments
    if args.tui {
        // Run in TUI mode
        info!("Starting OdinCode in TUI mode...");
        run_tui_mode(app).await?;
    } else if args.server {
        // Run in API server mode
        info!("Starting OdinCode in API server mode...");
        run_api_mode(app).await?;
    } else if args.cli {
        // Run in CLI mode
        info!("Starting OdinCode in CLI mode...");
        run_cli_mode(app).await?;
    } else {
        // Default to interactive demo mode if no specific mode is specified
        info!("Starting OdinCode in default mode (enhanced demo)...");
        app.run().await?;
    }

    info!("OdinCode finished successfully");
    Ok(())
}

/// Run the application in TUI mode
async fn run_tui_mode(app: OdinCodeApp) -> Result<()> {
    // Import the TUI module
    use odincode_tui::TuiRunner;

    // Extract all needed components
    let core_engine = Arc::clone(&app.core_engine);
    #[cfg(not(feature = "simple-ltmc"))]
    let ltmc_manager = Arc::clone(&app.ltmc_manager);
    let agent_coordinator = app.agent_coordinator.clone();
    let llm_manager = Arc::clone(&app.llm_manager);

    // Create a TUI runner with the application's components
    let tui_runner = TuiRunner::new(
        Arc::clone(&app.core_engine),
        // For TUI mode, we need to extract the components properly
        // Create a new LTMC manager for TUI mode
        #[cfg(not(feature = "simple-ltmc"))]
        {
            Arc::clone(&app.ltmc_manager)
        },
        #[cfg(feature = "simple-ltmc")]
        {
            // Use a dummy LTMC manager for simple mode
            Arc::new(odincode_ltmc::LTMManager::new())
        },
        app.agent_coordinator.clone(),
        // Create a tool manager
        odincode_tools::ToolManager::new_with_arcs(
            Arc::clone(&app.core_engine),
            #[cfg(not(feature = "simple-ltmc"))]
            Arc::clone(&app.ltmc_manager),
            #[cfg(feature = "simple-ltmc")]
            Arc::new(odincode_ltmc::LTMManager::new()),
            app.agent_coordinator.clone(),
        ),
    );

    // Run the TUI application
    tui_runner.run().await?;
    Ok(())
}

/// Run the application in API server mode
async fn run_api_mode(app: OdinCodeApp) -> Result<()> {
    // Import the API module
    use odincode_api::{models::ApiConfig, ApiServer};

    // Create API server configuration
    let config = ApiConfig {
        host: "0.0.0.0".to_string(),
        port: 8080,
        workers: 4,
        version: "1.0.0".to_string(),
    };

    // Create an API server with the application's components
    let api_server = ApiServer::new(
        config,
        Arc::clone(&app.core_engine),
        #[cfg(not(feature = "simple-ltmc"))]
        Arc::clone(&app.ltmc_manager),
        #[cfg(feature = "simple-ltmc")]
        Arc::new(odincode_ltmc::LTMManager::new()),
        Arc::new(app.agent_coordinator.clone()),
        Arc::new(odincode_tools::ToolManager::new_with_arcs(
            Arc::clone(&app.core_engine),
            #[cfg(not(feature = "simple-ltmc"))]
            Arc::clone(&app.ltmc_manager),
            #[cfg(feature = "simple-ltmc")]
            Arc::new(odincode_ltmc::LTMManager::new()),
            app.agent_coordinator.clone(),
        )), // We need to create a new ToolManager for the API
    );

    // Start the API server
    api_server.start().await?;
    Ok(())
}

/// Run the application in CLI mode
async fn run_cli_mode(app: OdinCodeApp) -> Result<()> {
    // For CLI mode, we can implement specific command-line operations
    // This is where we'd handle file analysis, refactoring, etc. as commands
    println!("OdinCode CLI mode is running...");

    // Example: run code analysis on specified files
    // This would be expanded with actual CLI functionality
    app.run_cli_operations().await?;

    Ok(())
}
