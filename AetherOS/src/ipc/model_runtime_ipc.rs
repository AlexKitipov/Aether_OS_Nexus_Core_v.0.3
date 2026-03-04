
// src/ipc/model_runtime_ipc.rs

#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;

use serde::{Deserialize, Serialize};

/// Represents requests from client V-Nodes to the Model Runtime V-Node for inference.
#[derive(Debug, Serialize, Deserialize)]
pub enum InferRequest {
    /// Request for image classification.
    ImageClassification { model_id: String, image_data: Vec<u8> },
    /// Request for text generation.
    TextGeneration { model_id: String, prompt: String, max_tokens: u32 },
    // Add more inference types as needed (e.g., ObjectDetection, SpeechToText)
}

/// Represents responses from the Model Runtime V-Node after inference.
#[derive(Debug, Serialize, Deserialize)]
pub enum InferResponse {
    /// Result for image classification.
    ImageClassificationResult { class_labels: Vec<String>, probabilities: Vec<f32> },
    /// Result for text generation.
    TextGenerationResult { generated_text: String },
    /// Indicates an error occurred during inference.
    Error { message: String },
}
