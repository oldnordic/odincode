//! Data models for the Simple LTMC system
//!
//! Contains the core data structures for tasks, patterns, and their relationships
//! using SQLite JSON for graph-like functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a development task in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for the task
    pub id: Uuid,
    /// Optional parent task ID for hierarchical structure
    pub parent_task_id: Option<Uuid>,
    /// Optional PRD ID this task belongs to
    pub prd_id: Option<Uuid>,
    /// Title of the task
    pub title: String,
    /// Detailed description of the task
    pub description: String,
    /// Current status of the task
    pub status: TaskStatus,
    /// Priority level
    pub priority: TaskPriority,
    /// Estimated time required (in minutes)
    pub estimated_time: Option<u32>,
    /// Actual time taken (in minutes)
    pub actual_time: Option<u32>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Completion timestamp (if completed)
    pub completed_at: Option<DateTime<Utc>>,
    /// Dependencies - other tasks that must be completed before this one
    pub dependencies: Vec<Uuid>,
    /// Related files that this task affects
    pub related_files: Vec<String>,
    /// Additional metadata stored as key-value pairs
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Status of a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    /// Task has been created but not yet started
    Todo,
    /// Task is currently in progress
    InProgress,
    /// Task is blocked by other tasks or external factors
    Blocked,
    /// Task has been completed successfully
    Completed,
    /// Task has been cancelled
    Cancelled,
}

/// Priority level of a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskPriority {
    /// Low priority task
    Low,
    /// Normal priority task
    Normal,
    /// High priority task
    High,
    /// Critical priority task
    Critical,
}

/// Represents a learning pattern in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Unique identifier for the pattern
    pub id: Uuid,
    /// Type of pattern
    pub pattern_type: PatternType,
    /// Content of the pattern (text description, code snippet, etc.)
    pub content: String,
    /// Title of the pattern
    pub title: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last access timestamp
    pub last_accessed: DateTime<Utc>,
    /// Number of times the pattern has been accessed
    pub access_count: u32,
    /// Confidence level in the pattern's validity/relevance
    pub confidence: f32,
    /// Embedding vector for semantic similarity search
    pub embedding: Vec<f32>,
    /// Context in which the pattern was learned
    pub context: HashMap<String, serde_json::Value>,
    /// Related patterns based on similarity or usage context
    pub related_patterns: Vec<RelatedPattern>,
}

/// Type of learning pattern
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PatternType {
    /// Architectural decision pattern
    ArchitecturalDecision,
    /// Code pattern or idiom
    CodePattern,
    /// Research finding or discovery
    ResearchFinding,
    /// Performance-related pattern
    PerformanceData,
    /// Solution to an error or problem
    ErrorSolution,
    /// User interaction pattern
    UserInteraction,
    /// Sequential thinking session data
    SequentialThinking,
    /// Model training pattern
    ModelTraining,
    /// Best practice pattern
    BestPractice,
    /// Anti-pattern
    AntiPattern,
    /// Security-related pattern
    SecurityPattern,
}

/// Represents a relationship between tasks or patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Unique identifier for the relationship
    pub id: Uuid,
    /// ID of the source entity (task or pattern)
    pub from_id: Uuid,
    /// Type of the source entity
    pub from_type: EntityType,
    /// ID of the target entity (task or pattern)
    pub to_id: Uuid,
    /// Type of the target entity
    pub to_type: EntityType,
    /// Type of relationship (dependency, related_to, blocks, etc.)
    pub relationship_type: RelationshipType,
    /// Additional metadata for the relationship
    pub metadata: HashMap<String, serde_json::Value>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Type of entity in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityType {
    /// Task entity
    Task,
    /// Pattern entity
    Pattern,
}

/// Type of relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    /// Dependency: the 'from' entity depends on the 'to' entity
    DependsOn,
    /// The entities are related but not dependent
    RelatedTo,
    /// The 'from' entity blocks the 'to' entity
    Blocks,
    /// The 'from' entity is a prerequisite for the 'to' entity
    Prerequisite,
    /// The 'from' entity is a subtask of the 'to' entity
    Subtask,
    /// The 'from' entity is a parent of the 'to' entity
    Parent,
    /// The entities are similar in some way
    Similar,
    /// The 'from' entity refines or implements the 'to' entity
    Refines,
    /// The 'from' entity provides context for the 'to' entity
    ProvidesContext,
}

/// Represents a relationship to another pattern with additional context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedPattern {
    /// ID of the related pattern
    pub pattern_id: Uuid,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Similarity score (for semantic similarity)
    pub similarity_score: f32,
    /// Additional context for the relationship
    pub context: String,
}

/// Represents a Product Requirement Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRequirement {
    /// Unique identifier for the PRD
    pub id: Uuid,
    /// Title of the PRD
    pub title: String,
    /// Brief overview of the feature
    pub overview: String,
    /// Goals of the feature
    pub goals: Vec<String>,
    /// User stories for the feature
    pub user_stories: Vec<UserStory>,
    /// Functional requirements
    pub functional_requirements: Vec<FunctionalRequirement>,
    /// Non-goals or out-of-scope items
    pub non_goals: Vec<String>,
    /// Design considerations
    pub design_considerations: Vec<String>,
    /// Technical considerations
    pub technical_considerations: Vec<String>,
    /// Success metrics
    pub success_metrics: Vec<String>,
    /// Open questions
    pub open_questions: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Current status of the PRD
    pub status: PRDStatus,
}

/// Represents a user story
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStory {
    /// Unique identifier for the user story
    pub id: Uuid,
    /// The role of the user
    pub role: String,
    /// What the user wants to do
    pub want: String,
    /// Why the user wants this
    pub benefit: String,
}

/// Represents a functional requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalRequirement {
    /// Unique identifier for the requirement
    pub id: Uuid,
    /// Requirement description
    pub description: String,
    /// Priority of the requirement
    pub priority: TaskPriority,
    /// Acceptance criteria
    pub acceptance_criteria: Vec<String>,
}

/// Status of a PRD
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PRDStatus {
    /// PRD is being drafted
    Draft,
    /// PRD is under review
    UnderReview,
    /// PRD has been approved
    Approved,
    /// PRD has been rejected
    Rejected,
    /// Feature implementation is in progress
    InProgress,
    /// Feature implementation is complete
    Completed,
}

impl Task {
    /// Create a new task with default values
    pub fn new(title: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_task_id: None,
            prd_id: None,
            title,
            description,
            status: TaskStatus::Todo,
            priority: TaskPriority::Normal,
            estimated_time: None,
            actual_time: None,
            created_at: Utc::now(),
            completed_at: None,
            dependencies: Vec::new(),
            related_files: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Mark the task as completed
    pub fn mark_completed(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark the task as in progress
    pub fn mark_in_progress(&mut self) {
        self.status = TaskStatus::InProgress;
    }

    /// Add a dependency to the task
    pub fn add_dependency(&mut self, dependency_id: Uuid) {
        if !self.dependencies.contains(&dependency_id) {
            self.dependencies.push(dependency_id);
        }
    }

    /// Add a related file to the task
    pub fn add_related_file(&mut self, file_path: String) {
        if !self.related_files.contains(&file_path) {
            self.related_files.push(file_path);
        }
    }
}

impl Pattern {
    /// Create a new pattern with default values
    pub fn new(title: String, content: String, pattern_type: PatternType) -> Self {
        Self {
            id: Uuid::new_v4(),
            pattern_type,
            content,
            title,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
            confidence: 1.0,       // Default to high confidence for new patterns
            embedding: Vec::new(), // Will be populated by the search module
            context: HashMap::new(),
            related_patterns: Vec::new(),
        }
    }

    /// Update the last accessed time and increment access count
    pub fn access(&mut self) {
        self.last_accessed = Utc::now();
        self.access_count += 1;
    }
}

impl Relationship {
    /// Create a new relationship
    pub fn new(
        from_id: Uuid,
        from_type: EntityType,
        to_id: Uuid,
        to_type: EntityType,
        relationship_type: RelationshipType,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_id,
            from_type,
            to_id,
            to_type,
            relationship_type,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }
}

impl ProductRequirement {
    /// Create a new PRD with default values
    pub fn new(title: String, overview: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            overview,
            goals: Vec::new(),
            user_stories: Vec::new(),
            functional_requirements: Vec::new(),
            non_goals: Vec::new(),
            design_considerations: Vec::new(),
            technical_considerations: Vec::new(),
            success_metrics: Vec::new(),
            open_questions: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: PRDStatus::Draft,
        }
    }

    /// Add a goal to the PRD
    pub fn add_goal(&mut self, goal: String) {
        self.goals.push(goal);
    }

    /// Add a user story to the PRD
    pub fn add_user_story(&mut self, user_story: UserStory) {
        self.user_stories.push(user_story);
    }

    /// Add a functional requirement to the PRD
    pub fn add_functional_requirement(&mut self, requirement: FunctionalRequirement) {
        self.functional_requirements.push(requirement);
    }

    /// Update the PRD status
    pub fn update_status(&mut self, status: PRDStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }
}
