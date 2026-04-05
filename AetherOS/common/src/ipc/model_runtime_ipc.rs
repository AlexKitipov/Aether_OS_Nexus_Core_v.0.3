extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Requests sent to the Model Runtime V-Node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InferRequest {
    /// Request image classification for raw image bytes.
    ImageClassification {
        model_id: String,
        image_data: Vec<u8>,
    },
    /// Request text generation/autocomplete for a prompt.
    TextGeneration {
        model_id: String,
        prompt: String,
        max_tokens: u32,
    },
}

/// Responses returned from the Model Runtime V-Node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InferResponse {
    /// Classification labels and associated probabilities.
    ImageClassificationResult {
        class_labels: Vec<String>,
        probabilities: Vec<f32>,
    },
    /// Generated continuation text for a prompt.
    TextGenerationResult {
        generated_text: String,
    },
    /// Inference/runtime failure.
    Error {
        message: String,
    },
}
