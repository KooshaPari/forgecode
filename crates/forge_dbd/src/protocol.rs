use forge_domain::{Conversation, ConversationId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    UpsertConversation { conversation: Conversation },
    UpsertConversationRef { conversation: Conversation },
    UpdateParentId {
        conversation_id: ConversationId,
        new_parent_id: Option<ConversationId>,
    },
    DeleteConversation { conversation_id: ConversationId },
    OptimizeFts,
    RefreshFts,
    CheckpointWal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Ack,
    Error { message: String },
}
