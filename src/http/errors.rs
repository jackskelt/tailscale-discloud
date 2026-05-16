use std::collections::HashMap;

use serde::Serialize;

/// A structured message with an i18n key and interpolation parameters.
/// The frontend resolves the `id` via its i18n module and substitutes
/// the `params` placeholders.
#[derive(Debug, Clone, Serialize)]
pub struct ApiMessage {
    pub id: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, serde_json::Value>,
}

impl ApiMessage {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            params: HashMap::new(),
        }
    }

    pub fn with_params(id: impl Into<String>, params: HashMap<String, serde_json::Value>) -> Self {
        Self {
            id: id.into(),
            params,
        }
    }
}

/// Structured API error returned to the frontend.
/// `error.id` is an i18n key; `error.params` carries any dynamic values
/// the frontend template needs for interpolation.
#[derive(Debug, Clone, Serialize)]
pub struct ApiErrorResponse {
    pub error: ApiMessage,
}
