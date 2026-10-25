//! Diagnostics: problems found in the content, reported together instead of one at a time.

use std::fmt;

use super::DocId;

/// A problem found in the content, and the document it concerns, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// The document concerned.
    pub source: Option<DocId>,
    /// What is wrong, as a sentence.
    pub message: String,
}

impl Diagnostic {
    /// A problem in one document.
    pub fn in_doc(source: &DocId, message: impl fmt::Display) -> Self {
        Self {
            source: Some(source.clone()),
            message: message.to_string(),
        }
    }

    /// A problem that concerns no single document.
    pub fn site(message: impl fmt::Display) -> Self {
        Self {
            source: None,
            message: message.to_string(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(source) => write!(f, "{source}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}
