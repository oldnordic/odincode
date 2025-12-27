//! Approval state management (Phase 9.2)
//!
//! Provides:
//! - ApprovalScope: Once vs SessionAllGated
//! - PendingApproval: Tool awaiting user approval
//! - ApprovalState: Session-scoped approval tracking

use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

/// Scope of tool approval
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalScope {
    /// Approve this specific tool invocation only
    Once { tool: String },
    /// Approve all GATED tools for current session
    SessionAllGated,
}

impl ApprovalScope {
    /// Check if a given tool is approved under this scope
    pub fn covers(&self, tool: &str) -> bool {
        match self {
            ApprovalScope::Once { tool: t } => t == tool,
            ApprovalScope::SessionAllGated => true,
        }
    }

    /// Display text for UI
    pub fn display_text(&self) -> &str {
        match self {
            ApprovalScope::Once { .. } => "once (this tool)",
            ApprovalScope::SessionAllGated => "session (all gated)",
        }
    }
}

/// Pending approval awaiting user response
#[derive(Debug, Clone)]
pub struct PendingApproval {
    /// Chat session ID
    pub session_id: String,
    /// Tool being approved
    pub tool: String,
    /// Tool arguments (for display)
    pub args: HashMap<String, String>,
    /// Step number when tool was requested
    pub step: usize,
    /// Affected file path (if any)
    pub affected_path: Option<String>,
    /// Timestamp of request
    pub requested_at: SystemTime,
}

impl PendingApproval {
    /// Create new pending approval
    pub fn new(
        session_id: String,
        tool: String,
        args: HashMap<String, String>,
        step: usize,
        affected_path: Option<String>,
    ) -> Self {
        Self {
            session_id,
            tool,
            args,
            step,
            affected_path,
            requested_at: SystemTime::now(),
        }
    }

    /// Format approval prompt for UI display
    pub fn format_prompt(&self) -> String {
        let mut prompt = format!("GATED Tool: {}\n", self.tool);
        if let Some(ref path) = self.affected_path {
            prompt.push_str(&format!("  File: {}\n", path));
        }
        prompt.push_str("  [y=once, a=session, n=deny, q=quit]");
        prompt
    }
}

/// Session-scoped approval state
#[derive(Debug, Clone, Default)]
pub struct ApprovalState {
    /// Approve all GATED tools for this session
    pub approved_all_gated: bool,
    /// Tools approved for single use (tool name)
    pub approved_once: HashSet<String>,
    /// Current pending approval (if any)
    pub pending: Option<PendingApproval>,
}

impl ApprovalState {
    /// Create new approval state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a tool is approved
    pub fn is_approved(&self, tool: &str) -> bool {
        self.approved_all_gated || self.approved_once.contains(tool)
    }

    /// Grant approval for a scope
    pub fn grant(&mut self, scope: ApprovalScope) {
        match scope {
            ApprovalScope::Once { tool } => {
                self.approved_once.insert(tool);
            }
            ApprovalScope::SessionAllGated => {
                self.approved_all_gated = true;
            }
        }
    }

    /// Set pending approval
    pub fn set_pending(&mut self, pending: PendingApproval) {
        self.pending = Some(pending);
    }

    /// Clear pending approval
    pub fn clear_pending(&mut self) {
        self.pending = None;
    }

    /// Get pending approval reference
    pub fn pending_approval(&self) -> Option<&PendingApproval> {
        self.pending.as_ref()
    }

    /// Reset state (call on new chat session)
    pub fn reset(&mut self) {
        self.approved_all_gated = false;
        self.approved_once.clear();
        self.pending = None;
    }
}

/// Approval response from UI back to chat loop
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalResponse {
    /// Approve this tool once
    ApproveOnce(String),
    /// Approve all GATED tools for session
    ApproveSessionAllGated,
    /// Deny this tool
    Deny(String),
    /// Quit immediately
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_scope_once_covers_only_its_tool() {
        let scope = ApprovalScope::Once {
            tool: "file_write".to_string(),
        };

        assert!(scope.covers("file_write"));
        assert!(!scope.covers("file_create"));
    }

    #[test]
    fn test_approval_scope_session_all_covers_all() {
        let scope = ApprovalScope::SessionAllGated;

        assert!(scope.covers("file_write"));
        assert!(scope.covers("file_create"));
    }

    #[test]
    fn test_approval_state_empty_initially() {
        let state = ApprovalState::new();

        assert!(!state.approved_all_gated);
        assert!(state.approved_once.is_empty());
        assert!(state.pending.is_none());
        assert!(!state.is_approved("file_write"));
    }

    #[test]
    fn test_approval_state_grant_once() {
        let mut state = ApprovalState::new();

        state.grant(ApprovalScope::Once {
            tool: "file_write".to_string(),
        });

        assert!(state.is_approved("file_write"));
        assert!(!state.is_approved("file_create"));
    }

    #[test]
    fn test_approval_state_grant_session_all() {
        let mut state = ApprovalState::new();

        state.grant(ApprovalScope::SessionAllGated);

        assert!(state.approved_all_gated);
        assert!(state.is_approved("file_write"));
        assert!(state.is_approved("file_create"));
    }

    #[test]
    fn test_approval_state_reset() {
        let mut state = ApprovalState::new();

        state.grant(ApprovalScope::SessionAllGated);
        state.set_pending(PendingApproval::new(
            "session".to_string(),
            "file_write".to_string(),
            HashMap::new(),
            1,
            None,
        ));

        assert!(state.approved_all_gated);
        assert!(state.pending.is_some());

        state.reset();

        assert!(!state.approved_all_gated);
        assert!(state.pending.is_none());
        assert!(!state.is_approved("file_write"));
    }

    #[test]
    fn test_pending_approval_format_prompt() {
        let pending = PendingApproval::new(
            "session-123".to_string(),
            "file_write".to_string(),
            HashMap::new(),
            1,
            Some("/path/to/file.txt".to_string()),
        );

        let prompt = pending.format_prompt();

        assert!(prompt.contains("GATED Tool"));
        assert!(prompt.contains("file_write"));
        assert!(prompt.contains("/path/to/file.txt"));
        assert!(prompt.contains("[y=once, a=session, n=deny, q=quit]"));
    }

    #[test]
    fn test_pending_approval_format_prompt_no_path() {
        let pending = PendingApproval::new(
            "session-123".to_string(),
            "file_create".to_string(),
            HashMap::new(),
            1,
            None,
        );

        let prompt = pending.format_prompt();

        assert!(prompt.contains("GATED Tool"));
        assert!(prompt.contains("file_create"));
        assert!(!prompt.contains("File:"));
    }

    #[test]
    fn test_approval_response_equality() {
        use ApprovalResponse::*;

        assert_eq!(
            ApproveOnce("file_write".to_string()),
            ApproveOnce("file_write".to_string())
        );
        assert_eq!(ApproveSessionAllGated, ApproveSessionAllGated);
        assert_eq!(
            Deny("file_write".to_string()),
            Deny("file_write".to_string())
        );
        assert_eq!(Quit, Quit);

        assert_ne!(
            ApproveOnce("file_write".to_string()),
            ApproveOnce("file_create".to_string())
        );
        assert_ne!(ApproveSessionAllGated, Quit);
    }
}
