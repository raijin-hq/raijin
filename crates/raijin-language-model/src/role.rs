use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

impl Role {
    pub fn from_proto(role: i32) -> Role {
        match raijin_proto::LanguageModelRole::from_i32(role) {
            Some(raijin_proto::LanguageModelRole::LanguageModelUser) => Role::User,
            Some(raijin_proto::LanguageModelRole::LanguageModelAssistant) => Role::Assistant,
            Some(raijin_proto::LanguageModelRole::LanguageModelSystem) => Role::System,
            None => Role::User,
        }
    }

    pub fn to_proto(self) -> raijin_proto::LanguageModelRole {
        match self {
            Role::User => raijin_proto::LanguageModelRole::LanguageModelUser,
            Role::Assistant => raijin_proto::LanguageModelRole::LanguageModelAssistant,
            Role::System => raijin_proto::LanguageModelRole::LanguageModelSystem,
        }
    }

    pub fn cycle(self) -> Role {
        match self {
            Role::User => Role::Assistant,
            Role::Assistant => Role::System,
            Role::System => Role::User,
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
        }
    }
}
