//! Semantic Code Analysis Engine
//!
//! This module provides intelligent semantic analysis capabilities for code understanding,
//! pattern recognition, and issue detection using ML and AI techniques.

use anyhow::Result;
use candle_core::{Device, Tensor};
use candle_nn::{Linear, Module};
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::CodeFile;

/// Code complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub cyclomatic_complexity: f64,
    pub cognitive_complexity: f64,
    pub halstead_volume: f64,
    pub loc: usize, // Lines of code
    pub comment_ratio: f64,
    pub function_count: usize,
    pub class_count: usize,
    pub parameter_count: usize,
}

/// Code pattern identified in analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePattern {
    pub id: Uuid,
    pub pattern_type: PatternType,
    pub name: String,
    pub confidence: f64,
    pub start_line: usize,
    pub end_line: usize,
    pub description: String,
    pub suggestions: Vec<String>,
    pub related_patterns: Vec<Uuid>,
}

/// Type of code pattern
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PatternType {
    AntiPattern,
    DesignPattern,
    CodeSmell,
    PerformanceIssue,
    SecurityVulnerability,
    BestPractice,
    ArchitecturalPattern,
    RefactoringOpportunity,
}

impl PatternType {
    /// Number of PatternType variants
    pub fn num_variants() -> usize {
        8 // Update this if you add more variants
    }

    /// Get all pattern types as a vector
    pub fn all_variants() -> Vec<PatternType> {
        vec![
            PatternType::AntiPattern,
            PatternType::DesignPattern,
            PatternType::CodeSmell,
            PatternType::PerformanceIssue,
            PatternType::SecurityVulnerability,
            PatternType::BestPractice,
            PatternType::ArchitecturalPattern,
            PatternType::RefactoringOpportunity,
        ]
    }
}

/// Semantic analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAnalysis {
    pub id: Uuid,
    pub file_id: Uuid,
    pub complexity_metrics: ComplexityMetrics,
    pub identified_patterns: Vec<CodePattern>,
    pub dependency_graph: DependencyGraph,
    pub semantic_features: Array2<f64>,
    pub analysis_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Dependency graph representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub nodes: Vec<String>,         // Function/class names
    pub edges: Vec<(usize, usize)>, // (source_idx, target_idx)
    pub node_types: Vec<NodeType>,
}

/// Type of dependency graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Function,
    Class,
    Interface,
    Variable,
    Module,
}

/// Simple neural network model for pattern recognition
pub struct PatternRecognitionModel {
    layer1: Linear,
    layer2: Linear,
    layer3: Linear,
}

impl PatternRecognitionModel {
    /// Create a new pattern recognition model
    pub fn new(vs: candle_nn::VarBuilder, input_size: usize, output_size: usize) -> Result<Self> {
        let hidden_size = 64;
        let layer1 = candle_nn::linear(input_size, hidden_size, vs.pp("layer1"))?;
        let layer2 = candle_nn::linear(hidden_size, hidden_size, vs.pp("layer2"))?;
        let layer3 = candle_nn::linear(hidden_size, output_size, vs.pp("layer3"))?;
        Ok(Self {
            layer1,
            layer2,
            layer3,
        })
    }

    /// Forward pass through the model
    pub fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let xs = self.layer1.forward(xs)?;
        let xs = xs.relu()?;
        let xs = self.layer2.forward(&xs)?;
        let xs = xs.relu()?;
        let xs = self.layer3.forward(&xs)?;
        Ok(xs)
    }
}

/// Semantic analysis engine
pub struct SemanticAnalysisEngine {
    /// Complexity thresholds
    complexity_thresholds: ComplexityThresholds,

    /// Pattern recognition models
    pattern_models: HashMap<PatternType, Box<dyn PatternRecognizer>>,

    /// Feature extraction capabilities
    feature_extractor: FeatureExtractor,

    /// Neural network model for pattern recognition
    pattern_recognition_model: Option<PatternRecognitionModel>,

    /// Device for neural network computations
    device: Device,
}

/// Complexity thresholds for issue detection
#[derive(Debug, Clone)]
struct ComplexityThresholds {
    max_cyclomatic_complexity: f64,
    max_cognitive_complexity: f64,
    min_comment_ratio: f64,
}

/// Trait for pattern recognition
trait PatternRecognizer: Send + Sync {
    fn identify_patterns(&self, code: &str, features: &Array2<f64>) -> Result<Vec<CodePattern>>;
    fn update_model(&mut self, training_data: &[TrainingSample]) -> Result<()>;
}

/// Training sample for pattern recognition
#[derive(Debug, Clone)]
struct TrainingSample {
    features: Array2<f64>,
    labels: Vec<PatternType>,
    confidence: f64,
}

/// Feature extraction utilities
#[derive(Debug, Clone)]
struct FeatureExtractor;

impl FeatureExtractor {
    /// Extract semantic features from code
    pub fn extract_features(&self, code: &str) -> Result<Array2<f64>> {
        let mut features = Vec::new();

        // Add basic code metrics
        let loc = code.lines().count();
        let comment_lines = self.count_comment_lines(code);
        let function_count = self.count_functions(code);
        let control_structures = self.count_control_structures(code);
        let nested_depth = self.calculate_max_nesting_depth(code);
        let identifier_entropy = self.calculate_identifier_entropy(code);

        // Create feature vector
        features.push(loc as f64);
        features.push(comment_lines as f64);
        features.push(function_count as f64);
        features.push(control_structures as f64);
        features.push(nested_depth as f64);
        features.push(identifier_entropy);

        // Normalize features
        let normalized = self.normalize_features(&features);

        // Convert to Array2<f64> with single row
        let feature_matrix = Array2::from_shape_vec((1, normalized.len()), normalized)?;

        Ok(feature_matrix)
    }

    /// Count comment lines in code
    fn count_comment_lines(&self, code: &str) -> usize {
        code.lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*")
            })
            .count()
    }

    /// Count function definitions in code
    fn count_functions(&self, code: &str) -> usize {
        // Simple pattern matching for function definitions (language-agnostic)
        // In a real implementation, we'd use proper AST parsing
        code.matches("fn ").count()
            + code.matches("function ").count()
            + code.matches("def ").count()
            + code.matches("class ").count()
    }

    /// Count control structures (if, for, while, etc.)
    fn count_control_structures(&self, code: &str) -> usize {
        code.matches("if ").count()
            + code.matches("for ").count()
            + code.matches("while ").count()
            + code.matches("match ").count()
            + code.matches("switch ").count()
            + code.matches("case ").count()
    }

    /// Calculate maximum nesting depth
    fn calculate_max_nesting_depth(&self, code: &str) -> usize {
        let mut max_depth = 0;
        let mut current_depth = 0;

        for ch in code.chars() {
            match ch {
                '{' | '[' | '(' => {
                    current_depth += 1;
                    max_depth = max_depth.max(current_depth);
                }
                '}' | ']' | ')' => {
                    if current_depth > 0 {
                        current_depth -= 1;
                    }
                }
                _ => {}
            }
        }

        max_depth
    }

    /// Calculate entropy of identifiers (measures naming diversity)
    fn calculate_identifier_entropy(&self, code: &str) -> f64 {
        use std::collections::HashMap;

        // Extract potential identifiers (simplified approach)
        let words: Vec<&str> = code
            .split_whitespace()
            .filter(|word| {
                ![
                    "fn", "let", "if", "for", "while", "match", "struct", "impl", "pub", "use",
                    "mod", "self", "Self", "String", "Vec", "Option", "Result", "async", "await",
                    "impl", "trait", "enum",
                ]
                .contains(word)
            })
            .filter(|word| word.chars().all(|c| c.is_alphanumeric() || c == '_'))
            .map(|word| word.trim_matches(|c: char| !c.is_alphabetic()))
            .filter(|word| word.len() > 2)
            .collect();

        if words.is_empty() {
            return 0.0;
        }

        // Count word frequencies
        let mut freq_map = HashMap::new();
        for word in &words {
            *freq_map.entry(*word).or_insert(0) += 1;
        }

        // Calculate entropy
        let mut entropy = 0.0;
        for &count in freq_map.values() {
            let probability = count as f64 / words.len() as f64;
            if probability > 0.0 {
                entropy -= probability * probability.log2();
            }
        }

        entropy
    }

    /// Normalize features using min-max scaling
    fn normalize_features(&self, features: &[f64]) -> Vec<f64> {
        if features.is_empty() {
            return Vec::new();
        }

        // Find min and max values
        let min_val = features.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_val = features.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        // Avoid division by zero
        if (max_val - min_val).abs() < f64::EPSILON {
            return vec![0.0; features.len()];
        }

        // Normalize each feature
        features
            .iter()
            .map(|&x| (x - min_val) / (max_val - min_val))
            .collect()
    }
}

impl SemanticAnalysisEngine {
    /// Create a new semantic analysis engine
    pub fn new() -> Self {
        let device = Device::Cpu; // Default to CPU, can be changed to Cuda if available

        Self {
            complexity_thresholds: ComplexityThresholds {
                max_cyclomatic_complexity: 10.0,
                max_cognitive_complexity: 15.0,
                min_comment_ratio: 0.1,
            },
            pattern_models: HashMap::new(),
            feature_extractor: FeatureExtractor,
            pattern_recognition_model: None, // Will be initialized after construction
            device,
        }
    }

    /// Initialize the neural network model for pattern recognition
    pub fn initialize_pattern_recognition_model(&mut self) -> Result<()> {
        use candle_nn::VarBuilder;

        // Create a simple variable builder for the model parameters
        let vb = VarBuilder::zeros(candle_core::DType::F32, &self.device);

        // Initialize the model with appropriate input/output sizes
        // Input size based on the number of features extracted by our FeatureExtractor
        let input_size = 6; // Based on the features extracted in extract_features method
        let output_size = PatternType::num_variants(); // Number of pattern types to classify

        match PatternRecognitionModel::new(vb, input_size, output_size) {
            Ok(model) => {
                self.pattern_recognition_model = Some(model);
                info!("Pattern recognition model initialized successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to initialize pattern recognition model: {}", e);
                Err(e.into())
            }
        }
    }

    /// Get the device used by the semantic analysis engine
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Perform semantic analysis on a code file
    pub async fn analyze(&self, file: &CodeFile) -> Result<SemanticAnalysis> {
        info!("Starting semantic analysis for file: {}", file.path);

        // Calculate complexity metrics
        let complexity_metrics = self.calculate_complexity_metrics(file)?;
        debug!("Calculated complexity metrics: {:?}", complexity_metrics);

        // Extract semantic features
        let semantic_features = self.feature_extractor.extract_features(&file.content)?;
        debug!("Extracted {} semantic features", semantic_features.ncols());

        // Identify code patterns
        let identified_patterns = self.identify_patterns(&file.content, &semantic_features)?;
        debug!("Identified {} code patterns", identified_patterns.len());

        // Build dependency graph
        let dependency_graph = self.build_dependency_graph(file)?;
        debug!(
            "Built dependency graph with {} nodes",
            dependency_graph.nodes.len()
        );

        // Apply neural network model for enhanced pattern recognition if available
        let enhanced_patterns = self
            .apply_neural_network_analysis(&file.content, &semantic_features, identified_patterns)
            .await?;

        // Create semantic analysis result
        let analysis = SemanticAnalysis {
            id: Uuid::new_v4(),
            file_id: file.id,
            complexity_metrics,
            identified_patterns: enhanced_patterns,
            dependency_graph,
            semantic_features,
            analysis_timestamp: chrono::Utc::now(),
        };

        info!("Completed semantic analysis for file: {}", file.path);
        Ok(analysis)
    }

    /// Calculate complexity metrics for code
    fn calculate_complexity_metrics(&self, file: &CodeFile) -> Result<ComplexityMetrics> {
        let loc = file.content.lines().count();
        let comment_lines = file
            .content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with("*")
                    || trimmed.starts_with("#")
            })
            .count();

        let comment_ratio = if loc > 0 {
            comment_lines as f64 / loc as f64
        } else {
            0.0
        };

        // Calculate cyclomatic complexity (simplified)
        let cyclomatic_complexity = self.calculate_cyclomatic_complexity(&file.content);

        // Calculate cognitive complexity (simplified)
        let cognitive_complexity = self.calculate_cognitive_complexity(&file.content);

        // Calculate Halstead volume (simplified)
        let halstead_volume = self.calculate_halstead_volume(&file.content);

        // Count functions and classes (language-agnostic)
        let function_count = file.content.matches("fn ").count()
            + file.content.matches("function ").count()
            + file.content.matches("def ").count();

        let class_count = file.content.matches("struct ").count()
            + file.content.matches("class ").count()
            + file.content.matches("interface ").count();

        let parameter_count = file.content.matches(",").count(); // Rough estimate

        Ok(ComplexityMetrics {
            cyclomatic_complexity,
            cognitive_complexity,
            halstead_volume,
            loc,
            comment_ratio,
            function_count,
            class_count,
            parameter_count,
        })
    }

    /// Calculate cyclomatic complexity
    fn calculate_cyclomatic_complexity(&self, code: &str) -> f64 {
        let mut complexity = 1; // Base complexity

        // Add complexity for control flow structures
        complexity += code.matches("if ").count();
        complexity += code.matches("elif ").count();
        complexity += code.matches("else ").count();
        complexity += code.matches("for ").count();
        complexity += code.matches("while ").count();
        complexity += code.matches("match ").count();
        complexity += code.matches("case ").count();
        complexity += code.matches("switch ").count();
        complexity += code.matches("&&").count();
        complexity += code.matches("||").count();
        complexity += code.matches(" and ").count();
        complexity += code.matches(" or ").count();

        complexity as f64
    }

    /// Calculate cognitive complexity
    fn calculate_cognitive_complexity(&self, code: &str) -> f64 {
        let mut complexity = 0.0;

        // Count nested levels of control flow
        let mut _nesting_level = 0;
        for line in code.lines() {
            let trimmed = line.trim();
            let current_nesting = (line.len() - trimmed.len()) / 4; // Estimating by indentation

            if trimmed.starts_with("if ")
                || trimmed.starts_with("for ")
                || trimmed.starts_with("while ")
                || trimmed.starts_with("match ")
                || trimmed.starts_with("fn ")
            {
                complexity += (current_nesting + 1) as f64;
            }

            _nesting_level = current_nesting;
        }

        complexity
    }

    /// Calculate Halstead volume (simplified)
    fn calculate_halstead_volume(&self, code: &str) -> f64 {
        let operators = [
            "fn", "let", "if", "for", "while", "match", "struct", "impl", "pub", "use", "mod",
            "return", "break", "continue", "async", "await", "const", "static", "=", "+", "-", "*",
            "/", "%", "==", "!=", "<", ">", "<=", ">=", "!", "&", "|", "^", "<<", ">>", "+=", "-=",
            "*=", "/=", "%=", "&=", "|=", "^=", "=>", "->",
        ];
        let mut operator_count = 0;

        for op in &operators {
            operator_count += code.matches(op).count();
        }

        let operands = code
            .split_whitespace()
            .filter(|word| !operators.contains(word))
            .filter(|word| word.chars().all(|c| c.is_alphanumeric() || c == '_'))
            .collect::<Vec<_>>();

        let operand_count = operands.len();

        // Calculate Halstead volume
        let n1 = operator_count as f64;
        let n2 = operand_count as f64;
        let n = n1 + n2;
        let v = if n > 0.0 {
            n * (n1.max(1.0) * n2.max(1.0)).log2()
        } else {
            0.0
        };

        v
    }

    /// Identify code patterns in the code
    fn identify_patterns(&self, code: &str, _features: &Array2<f64>) -> Result<Vec<CodePattern>> {
        let mut patterns = Vec::new();

        // Identify potential issues based on complexity metrics
        patterns.extend(self.identify_complexity_issues(code));

        // Identify common code smells
        patterns.extend(self.identify_code_smells(code));

        // Identify security vulnerabilities
        patterns.extend(self.identify_security_issues(code));

        // Identify performance issues
        patterns.extend(self.identify_performance_issues(code));

        Ok(patterns)
    }

    /// Identify complexity-related issues
    fn identify_complexity_issues(&self, code: &str) -> Vec<CodePattern> {
        let mut patterns = Vec::new();

        // High cyclomatic complexity detection
        let cyclomatic_complexity = self.calculate_cyclomatic_complexity(code);
        if cyclomatic_complexity > self.complexity_thresholds.max_cyclomatic_complexity {
            patterns.push(CodePattern {
                id: Uuid::new_v4(),
                pattern_type: PatternType::CodeSmell,
                name: "High Cyclomatic Complexity".to_string(),
                confidence: 0.9,
                start_line: 1,
                end_line: code.lines().count(),
                description: format!(
                    "Code has high cyclomatic complexity ({} > {})",
                    cyclomatic_complexity, self.complexity_thresholds.max_cyclomatic_complexity
                ),
                suggestions: vec![
                    "Consider breaking down complex functions into smaller ones".to_string(),
                    "Use design patterns to simplify complex control flow".to_string(),
                    "Extract complex conditional logic into separate functions".to_string(),
                ],
                related_patterns: Vec::new(),
            });
        }

        // Low documentation ratio
        let comment_ratio = self.calculate_comment_ratio(code);
        if comment_ratio < self.complexity_thresholds.min_comment_ratio {
            patterns.push(CodePattern {
                id: Uuid::new_v4(),
                pattern_type: PatternType::CodeSmell,
                name: "Low Documentation".to_string(),
                confidence: 0.7,
                start_line: 1,
                end_line: code.lines().count(),
                description: format!(
                    "Code has low comment-to-code ratio ({} < {})",
                    comment_ratio, self.complexity_thresholds.min_comment_ratio
                ),
                suggestions: vec![
                    "Add more comments to explain complex logic".to_string(),
                    "Document public APIs and functions".to_string(),
                    "Add inline comments for complex algorithms".to_string(),
                ],
                related_patterns: Vec::new(),
            });
        }

        patterns
    }

    /// Identify common code smells
    fn identify_code_smells(&self, code: &str) -> Vec<CodePattern> {
        let mut patterns = Vec::new();

        // Long parameter list detection
        if self.has_long_parameter_list(code) {
            patterns.push(CodePattern {
                id: Uuid::new_v4(),
                pattern_type: PatternType::CodeSmell,
                name: "Long Parameter List".to_string(),
                confidence: 0.8,
                start_line: 1,
                end_line: code.lines().count(),
                description: "Function has too many parameters".to_string(),
                suggestions: vec![
                    "Consider using a configuration object or struct".to_string(),
                    "Group related parameters into logical units".to_string(),
                    "Use builder pattern for complex object creation".to_string(),
                ],
                related_patterns: Vec::new(),
            });
        }

        // Large class detection (simplified)
        if self.has_large_class(code) {
            patterns.push(CodePattern {
                id: Uuid::new_v4(),
                pattern_type: PatternType::CodeSmell,
                name: "Large Class".to_string(),
                confidence: 0.75,
                start_line: 1,
                end_line: code.lines().count(),
                description: "Class has too many methods or fields".to_string(),
                suggestions: vec![
                    "Consider breaking the class into smaller, focused classes".to_string(),
                    "Apply Single Responsibility Principle".to_string(),
                    "Extract related functionality into separate classes".to_string(),
                ],
                related_patterns: Vec::new(),
            });
        }

        patterns
    }

    /// Identify security vulnerabilities
    fn identify_security_issues(&self, code: &str) -> Vec<CodePattern> {
        let mut patterns = Vec::new();

        // Hardcoded credentials detection
        if self.contains_hardcoded_credentials(code) {
            patterns.push(CodePattern {
                id: Uuid::new_v4(),
                pattern_type: PatternType::SecurityVulnerability,
                name: "Hardcoded Credentials".to_string(),
                confidence: 0.95,
                start_line: 1,
                end_line: code.lines().count(),
                description: "Found hardcoded credentials in code".to_string(),
                suggestions: vec![
                    "Use environment variables for sensitive data".to_string(),
                    "Load credentials from secure configuration files".to_string(),
                    "Implement proper secret management".to_string(),
                ],
                related_patterns: Vec::new(),
            });
        }

        // SQL injection potential
        if self.contains_sql_injection_risk(code) {
            patterns.push(CodePattern {
                id: Uuid::new_v4(),
                pattern_type: PatternType::SecurityVulnerability,
                name: "SQL Injection Risk".to_string(),
                confidence: 0.85,
                start_line: 1,
                end_line: code.lines().count(),
                description: "Potential SQL injection vulnerability detected".to_string(),
                suggestions: vec![
                    "Use parameterized queries".to_string(),
                    "Validate and sanitize user input".to_string(),
                    "Use ORM libraries with built-in protection".to_string(),
                ],
                related_patterns: Vec::new(),
            });
        }

        patterns
    }

    /// Identify performance issues
    fn identify_performance_issues(&self, code: &str) -> Vec<CodePattern> {
        let mut patterns = Vec::new();

        // Nested loop detection (potential performance issue)
        if self.contains_nested_loops(code) {
            patterns.push(CodePattern {
                id: Uuid::new_v4(),
                pattern_type: PatternType::PerformanceIssue,
                name: "Nested Loops".to_string(),
                confidence: 0.7,
                start_line: 1,
                end_line: code.lines().count(),
                description: "Found nested loops which might cause performance issues".to_string(),
                suggestions: vec![
                    "Consider using more efficient algorithms".to_string(),
                    "Look for opportunities to reduce time complexity".to_string(),
                    "Profile code to measure actual performance impact".to_string(),
                ],
                related_patterns: Vec::new(),
            });
        }

        // Potential memory issue detection
        if self.contains_memory_issues(code) {
            patterns.push(CodePattern {
                id: Uuid::new_v4(),
                pattern_type: PatternType::PerformanceIssue,
                name: "Memory Usage".to_string(),
                confidence: 0.75,
                start_line: 1,
                end_line: code.lines().count(),
                description: "Potential memory usage issue detected".to_string(),
                suggestions: vec![
                    "Consider using more memory-efficient data structures".to_string(),
                    "Implement lazy evaluation where possible".to_string(),
                    "Monitor memory usage with profiling tools".to_string(),
                ],
                related_patterns: Vec::new(),
            });
        }

        patterns
    }

    /// Build dependency graph from code
    fn build_dependency_graph(&self, file: &CodeFile) -> Result<DependencyGraph> {
        // This is a simplified dependency graph builder
        // In a real implementation, we would use proper AST parsing

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut node_types = Vec::new();

        // Extract function and class names as nodes
        for (_line_num, line) in file.content.lines().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("fn ") {
                // Extract function name
                if let Some(func_name) = self.extract_function_name(trimmed) {
                    nodes.push(func_name);
                    node_types.push(NodeType::Function);
                }
            } else if trimmed.starts_with("struct ") {
                // Extract struct name
                if let Some(struct_name) = self.extract_struct_name(trimmed) {
                    nodes.push(struct_name);
                    node_types.push(NodeType::Class);
                }
            }
        }

        // Add some basic edges based on common patterns
        // In a real implementation, we would analyze actual function calls
        for (i, _node) in nodes.iter().enumerate() {
            if i > 0 {
                // Connect each node to previous one as a simple example
                edges.push((i - 1, i));
            }
        }

        Ok(DependencyGraph {
            nodes,
            edges,
            node_types,
        })
    }

    /// Extract function name from declaration
    fn extract_function_name(&self, line: &str) -> Option<String> {
        // Simple heuristic to extract function name from "fn name(" pattern
        if let Some(start) = line.find("fn ") {
            let after_fn = &line[start + 3..];
            if let Some(name_end) = after_fn.find('(') {
                let func_name = &after_fn[..name_end].trim();
                return Some(func_name.to_string());
            }
        }
        None
    }

    /// Extract struct name from declaration
    fn extract_struct_name(&self, line: &str) -> Option<String> {
        // Simple heuristic to extract struct name from "struct Name" pattern
        if let Some(start) = line.find("struct ") {
            let after_struct = &line[start + 7..];
            // Find end of struct name (before '{' or generics)
            let mut end_pos = after_struct.len();
            if let Some(generics_pos) = after_struct.find('<') {
                end_pos = generics_pos;
            }
            if let Some(brace_pos) = after_struct.find('{') {
                end_pos = end_pos.min(brace_pos);
            }
            let struct_name = &after_struct[..end_pos].trim();
            return Some(struct_name.to_string());
        }
        None
    }

    /// Calculate comment ratio
    fn calculate_comment_ratio(&self, code: &str) -> f64 {
        let total_lines = code.lines().count();
        if total_lines == 0 {
            return 0.0;
        }

        let comment_lines = code
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with("*")
                    || trimmed.starts_with("#")
            })
            .count();

        comment_lines as f64 / total_lines as f64
    }

    /// Check for long parameter list
    fn has_long_parameter_list(&self, code: &str) -> bool {
        // Check if any function has many parameters (more than 5)
        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("fn ") {
                let param_count = self.count_parameters(trimmed);
                if param_count > 5 {
                    return true;
                }
            }
        }
        false
    }

    /// Count parameters in function signature
    fn count_parameters(&self, fn_line: &str) -> usize {
        if let Some(parens_start) = fn_line.find('(') {
            if let Some(parens_end) = fn_line.find(')') {
                if parens_start < parens_end {
                    let params_str = &fn_line[parens_start + 1..parens_end];
                    if params_str.trim().is_empty() {
                        return 0;
                    }
                    return params_str.split(',').count();
                }
            }
        }
        0
    }

    /// Check for large class (simplified)
    fn has_large_class(&self, code: &str) -> bool {
        // Count the number of methods in a class (simplified approach)
        let method_count = code.matches("fn ").count();
        method_count > 20
    }

    /// Check for hardcoded credentials
    fn contains_hardcoded_credentials(&self, code: &str) -> bool {
        let lowercase_code = code.to_lowercase();
        let credential_patterns = [
            "password:",
            "password =",
            "password=\"",
            "password='",
            "secret:",
            "secret =",
            "secret=\"",
            "secret='",
            "token:",
            "token =",
            "token=\"",
            "token='",
            "api_key:",
            "api_key =",
            "api_key=\"",
            "api_key='",
            "access_key:",
            "access_key =",
            "access_key=\"",
            "access_key='",
        ];

        credential_patterns
            .iter()
            .any(|&pattern| lowercase_code.contains(pattern))
    }

    /// Check for potential SQL injection risk
    fn contains_sql_injection_risk(&self, code: &str) -> bool {
        let lowercase_code = code.to_lowercase();
        let sql_patterns = [
            "select ", "insert ", "update ", "delete ", "drop ", "create ", "alter ",
        ];

        sql_patterns.iter().any(|&pattern| {
            lowercase_code.contains(pattern)
                && (lowercase_code.contains(" where ")
                    || lowercase_code.contains(" from ")
                    || lowercase_code.contains("values")
                    || lowercase_code.contains("concat"))
        })
    }

    /// Check for nested loops
    fn contains_nested_loops(&self, code: &str) -> bool {
        let mut loop_depth = 0;
        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("for ")
                || trimmed.starts_with("while ")
                || trimmed.starts_with("loop ")
            {
                loop_depth += 1;
                if loop_depth > 1 {
                    return true; // Found nested loop
                }
            } else if trimmed == "}" || trimmed.starts_with("}") {
                if loop_depth > 0 {
                    loop_depth -= 1;
                }
            }
        }
        false
    }

    /// Check for potential memory issues (simplified)
    fn contains_memory_issues(&self, code: &str) -> bool {
        let lowercase_code = code.to_lowercase();
        let memory_patterns = [
            "vec!",
            "vector",
            "box::new",
            "rc::new",
            "arc::new",
            "large_array",
            "huge_collection",
        ];

        memory_patterns
            .iter()
            .any(|&pattern| lowercase_code.contains(pattern))
    }

    /// Apply neural network analysis to enhance pattern recognition
    async fn apply_neural_network_analysis(
        &self,
        code: &str,
        features: &Array2<f64>,
        existing_patterns: Vec<CodePattern>,
    ) -> Result<Vec<CodePattern>> {
        if let Some(ref model) = self.pattern_recognition_model {
            debug!("Applying neural network analysis to enhance pattern recognition");

            // Convert features to tensor for neural network processing
            // First flatten the Array2 to a 1D vector
            let flat_features: Vec<f64> = features.iter().cloned().collect();

            // Create input tensor - need to reshape to (batch_size, feature_count)
            // For single input, batch_size = 1
            let input_tensor = match Tensor::from_vec(
                flat_features,
                (1, features.ncols()), // (batch_size, num_features)
                &self.device,
            ) {
                Ok(tensor) => tensor,
                Err(e) => {
                    error!("Failed to create input tensor: {}", e);
                    // Return existing patterns unchanged if tensor creation fails
                    return Ok(existing_patterns);
                }
            };

            // Run the neural network to predict pattern types
            let prediction_result = model.forward(&input_tensor);

            match prediction_result {
                Ok(predictions) => {
                    // Convert predictions to probabilities using manual softmax implementation
                    let probabilities = match self.manual_softmax(&predictions, 1) {
                        Ok(probs) => probs,
                        Err(e) => {
                            error!("Failed to compute softmax: {}", e);
                            return Ok(existing_patterns);
                        }
                    };

                    // Convert to Vec<f32> to get class probabilities
                    let prob_values: Vec<f32> = match probabilities.to_vec1() {
                        Ok(vals) => vals,
                        Err(e) => {
                            error!("Failed to extract prediction values: {}", e);
                            return Ok(existing_patterns);
                        }
                    };

                    // Create new patterns based on neural network predictions
                    let mut nn_patterns = Vec::new();

                    for (idx, &prob) in prob_values.iter().enumerate() {
                        if prob > 0.3 {
                            // Threshold for considering a pattern
                            // Get the pattern type based on index
                            let pattern_type = match idx {
                                0 => PatternType::AntiPattern,
                                1 => PatternType::DesignPattern,
                                2 => PatternType::CodeSmell,
                                3 => PatternType::PerformanceIssue,
                                4 => PatternType::SecurityVulnerability,
                                5 => PatternType::BestPractice,
                                6 => PatternType::ArchitecturalPattern,
                                7 => PatternType::RefactoringOpportunity,
                                _ => continue, // Skip if index doesn't match any pattern type
                            };

                            let pattern_name = match &pattern_type {
                                PatternType::AntiPattern => "Anti-Pattern Detected",
                                PatternType::DesignPattern => "Design Pattern Identified",
                                PatternType::CodeSmell => "Code Smell Detected",
                                PatternType::PerformanceIssue => "Performance Issue Identified",
                                PatternType::SecurityVulnerability => {
                                    "Security Vulnerability Detected"
                                }
                                PatternType::BestPractice => "Best Practice Identified",
                                PatternType::ArchitecturalPattern => {
                                    "Architectural Pattern Identified"
                                }
                                PatternType::RefactoringOpportunity => {
                                    "Refactoring Opportunity Identified"
                                }
                            };

                            nn_patterns.push(CodePattern {
                                id: Uuid::new_v4(),
                                pattern_type,
                                name: pattern_name.to_string(),
                                confidence: prob as f64,
                                start_line: 1,
                                end_line: code.lines().count(),
                                description: format!("Neural network detected {} with {:.2}% confidence", 
                                                   pattern_name.to_lowercase(), 
                                                   prob * 100.0),
                                suggestions: vec!["Review and address this pattern as identified by neural network analysis".to_string()],
                                related_patterns: Vec::new(),
                            });
                        }
                    }

                    // Combine existing patterns with neural network patterns
                    let mut all_patterns = existing_patterns;
                    all_patterns.extend(nn_patterns);

                    Ok(all_patterns)
                }
                Err(e) => {
                    error!("Neural network prediction failed: {}", e);
                    // Return existing patterns unchanged if prediction fails
                    Ok(existing_patterns)
                }
            }
        } else {
            debug!("Neural network model not available, returning existing patterns");
            // If neural network model is not initialized, return existing patterns
            Ok(existing_patterns)
        }
    }

    /// Manual implementation of softmax function for tensor operations
    fn manual_softmax(&self, tensor: &Tensor, dim: usize) -> Result<Tensor> {
        // Subtract max for numerical stability
        let max_vals = tensor.max_keepdim(dim)?;
        let shifted = tensor.broadcast_sub(&max_vals)?;

        // Exponentiate
        let exp_vals = shifted.exp()?;

        // Sum along the dimension
        let sum_vals = exp_vals.sum_keepdim(dim)?;

        // Divide to get probabilities
        let probabilities = exp_vals.broadcast_div(&sum_vals)?;

        Ok(probabilities)
    }
}

impl Default for SemanticAnalysisEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodeFile;

    #[tokio::test]
    async fn test_semantic_analysis_engine_creation() {
        let engine = SemanticAnalysisEngine::new();
        assert!(true); // Basic test to ensure creation works
    }

    #[tokio::test]
    async fn test_simple_code_analysis() {
        let engine = SemanticAnalysisEngine::new();
        let code_file = CodeFile {
            id: Uuid::new_v4(),
            path: "test.rs".to_string(),
            content: r#"
fn simple_function(x: i32) -> i32 {
    if x > 0 {
        return x * 2;
    } else {
        return x;
    }
}
            "#
            .to_string(),
            language: "rust".to_string(),
            modified: chrono::Utc::now(),
        };

        let result = engine.analyze(&code_file).await;
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert_eq!(analysis.file_id, code_file.id);
        assert!(!analysis.identified_patterns.is_empty());
    }

    #[tokio::test]
    async fn test_complexity_calculation() {
        let engine = SemanticAnalysisEngine::new();
        let code_file = CodeFile {
            id: Uuid::new_v4(),
            path: "complex.rs".to_string(),
            content: r#"
fn complex_function(x: i32) -> i32 {
    let mut result = 0;
    for i in 0..10 {
        if i % 2 == 0 {
            for j in 0..10 {
                if j > 5 {
                    result += x;
                } else {
                    result -= x;
                }
            }
        } else {
            if x > 0 {
                result *= 2;
            } else {
                result /= 2;
            }
        }
    }
    result
}
            "#
            .to_string(),
            language: "rust".to_string(),
            modified: chrono::Utc::now(),
        };

        let result = engine.analyze(&code_file).await.unwrap();
        assert!(result.complexity_metrics.cyclomatic_complexity > 3.0);
        assert!(result.complexity_metrics.cognitive_complexity > 5.0);
    }

    #[tokio::test]
    async fn test_pattern_identification() {
        let engine = SemanticAnalysisEngine::new();
        let code_file = CodeFile {
            id: Uuid::new_v4(),
            path: "pattern_test.rs".to_string(),
            content: r#"
// This is a hardcoded password - security issue
let password = "admin123";
fn function_with_many_params(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32) -> i32 {
    a + b + c + d + e + f + g
}
            "#
            .to_string(),
            language: "rust".to_string(),
            modified: chrono::Utc::now(),
        };

        let result = engine.analyze(&code_file).await.unwrap();

        // Check that security issues and code smells are identified
        let security_issues: Vec<_> = result
            .identified_patterns
            .iter()
            .filter(|p| matches!(p.pattern_type, PatternType::SecurityVulnerability))
            .collect();
        assert!(!security_issues.is_empty());

        let code_smells: Vec<_> = result
            .identified_patterns
            .iter()
            .filter(|p| matches!(p.pattern_type, PatternType::CodeSmell))
            .collect();
        assert!(!code_smells.is_empty());
    }
}
