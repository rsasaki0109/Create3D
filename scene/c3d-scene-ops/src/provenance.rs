use serde::{Deserialize, Serialize};

/// Audit metadata attached to AI-generated transactions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionProvenance {
    /// Agent or subsystem that produced the transaction.
    pub agent: String,
    /// Original user prompt.
    pub user_prompt: String,
    /// Model identifier used for planning.
    pub model_id: String,
    /// Tool names invoked while building the plan.
    pub tool_names: Vec<String>,
}
