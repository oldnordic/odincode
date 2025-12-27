//! Session state for LLM interactions
//!
//! Tracks plans and authorization within a session.

use crate::llm::types::{Plan, PlanAuthorization, SessionContext};

/// LLM session state
///
/// Tracks plans and authorization within a session.
/// No background processing, no autonomous actions.
pub struct LlmSession {
    context: SessionContext,
    plans: Vec<Plan>,
    current_authorization: Option<PlanAuthorization>,
}

/// Create a new session from context
pub fn create_session(context: SessionContext) -> LlmSession {
    LlmSession {
        context,
        plans: Vec::new(),
        current_authorization: None,
    }
}

impl LlmSession {
    /// Get session context
    pub fn context(&self) -> &SessionContext {
        &self.context
    }

    /// Get all plans in session
    pub fn plans(&self) -> &[Plan] {
        &self.plans
    }

    /// Set current plan for authorization
    pub fn set_plan_for_authorization(&mut self, plan: Plan) {
        let auth = PlanAuthorization::new(plan.plan_id.clone());
        self.current_authorization = Some(auth);
        self.plans.push(plan);
    }

    /// Get current authorization
    pub fn authorization(&self) -> Option<&PlanAuthorization> {
        self.current_authorization.as_ref()
    }

    /// Approve current plan
    pub fn approve(&mut self) {
        if let Some(ref mut auth) = self.current_authorization {
            auth.approve();
        }
    }

    /// Reject current plan
    pub fn reject(&mut self) {
        if let Some(ref mut auth) = self.current_authorization {
            auth.reject();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::Intent;
    use std::path::PathBuf;

    #[test]
    fn test_create_session() {
        let context = SessionContext {
            user_intent: "test".to_string(),
            selected_file: None,
            current_diagnostic: None,
            db_root: PathBuf::from("."),
        };

        let session = create_session(context);
        assert!(session.plans().is_empty());
        assert!(session.authorization().is_none());
    }

    #[test]
    fn test_session_context() {
        let context = SessionContext {
            user_intent: "test intent".to_string(),
            selected_file: Some("src/lib.rs".to_string()),
            current_diagnostic: Some("error: test".to_string()),
            db_root: PathBuf::from("."),
        };

        let session = create_session(context);
        assert_eq!(session.context().user_intent, "test intent");
        assert_eq!(session.context().selected_file, Some("src/lib.rs".to_string()));
        assert_eq!(session.context().current_diagnostic, Some("error: test".to_string()));
    }

    #[test]
    fn test_set_plan_for_authorization() {
        let context = SessionContext {
            user_intent: "test".to_string(),
            selected_file: None,
            current_diagnostic: None,
            db_root: PathBuf::from("."),
        };

        let mut session = create_session(context);

        let plan = Plan {
            plan_id: "test_plan".to_string(),
            intent: Intent::Read,
            steps: vec![],
            evidence_referenced: vec![],
        };

        session.set_plan_for_authorization(plan);

        // Plan should be added to session
        assert_eq!(session.plans().len(), 1);
        assert_eq!(session.plans()[0].plan_id, "test_plan");

        // Authorization should be set
        assert!(session.authorization().is_some());
        assert_eq!(session.authorization().unwrap().plan_id(), "test_plan");
    }

    #[test]
    fn test_authorization_approve_reject() {
        let context = SessionContext {
            user_intent: "test".to_string(),
            selected_file: None,
            current_diagnostic: None,
            db_root: PathBuf::from("."),
        };

        let mut session = create_session(context);

        let plan = Plan {
            plan_id: "test_auth_plan".to_string(),
            intent: Intent::Read,
            steps: vec![],
            evidence_referenced: vec![],
        };

        session.set_plan_for_authorization(plan);

        // Initially not approved
        assert!(!session.authorization().unwrap().is_approved());

        // Approve
        session.approve();
        assert!(session.authorization().unwrap().is_approved());
    }

    #[test]
    fn test_authorization_reject() {
        let context = SessionContext {
            user_intent: "test".to_string(),
            selected_file: None,
            current_diagnostic: None,
            db_root: PathBuf::from("."),
        };

        let mut session = create_session(context);

        let plan = Plan {
            plan_id: "test_auth_plan".to_string(),
            intent: Intent::Read,
            steps: vec![],
            evidence_referenced: vec![],
        };

        session.set_plan_for_authorization(plan);

        // Reject
        session.reject();
        assert!(!session.authorization().unwrap().is_approved());
    }

    #[test]
    fn test_multiple_plans_in_session() {
        let context = SessionContext {
            user_intent: "test".to_string(),
            selected_file: None,
            current_diagnostic: None,
            db_root: PathBuf::from("."),
        };

        let mut session = create_session(context);

        // Add multiple plans
        for i in 0..3 {
            let plan = Plan {
                plan_id: format!("plan_{}", i),
                intent: Intent::Read,
                steps: vec![],
                evidence_referenced: vec![],
            };
            session.set_plan_for_authorization(plan);
        }

        assert_eq!(session.plans().len(), 3);
        assert_eq!(session.plans()[0].plan_id, "plan_0");
        assert_eq!(session.plans()[1].plan_id, "plan_1");
        assert_eq!(session.plans()[2].plan_id, "plan_2");
    }
}
