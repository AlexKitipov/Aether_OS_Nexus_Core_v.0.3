
# Model Runtime V-Node (svc://model-runtime)

## Overview

The `model-runtime` V-Node is a dedicated service within AetherOS designed to host and execute machine learning models for inference. It provides a secure, isolated, and resource-managed environment for performing AI tasks like image classification, text generation, and more. Client V-Nodes can interact with it via a well-defined IPC interface, abstracting away the complexities of model loading, execution, and resource management.

## IPC Protocol

Communication with the `model-runtime` V-Node occurs via IPC, using the `InferRequest` and `InferResponse` enums defined in `src/ipc/model_runtime_ipc.rs`.

### InferRequest Enum (Client -> model-runtime)

Client V-Nodes send these requests to `svc://model-runtime` to perform machine learning inferences.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub enum InferRequest {
    /// Request for image classification.
    ImageClassification { model_id: String, image_data: Vec<u8> },
    /// Request for text generation.
    TextGeneration { model_id: String, prompt: String, max_tokens: u32 },
    // Add more inference types as needed (e.g., ObjectDetection, SpeechToText)
}
```

**Parameters:**

*   `model_id`: A `String` identifying the specific model to be used for inference (e.g., "image_classifier_v1", "gpt-nano").
*   `image_data`: A `Vec<u8>` containing the raw bytes of an image for classification.
*   `prompt`: A `String` containing the input text for text generation.
*   `max_tokens`: A `u32` specifying the maximum number of tokens to generate for text tasks.

### InferResponse Enum (model-runtime -> Client)

`svc://model-runtime` sends these responses back to the client V-Node after processing an `InferRequest`.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub enum InferResponse {
    /// Result for image classification.
    ImageClassificationResult { class_labels: Vec<String>, probabilities: Vec<f32> },
    /// Result for text generation.
    TextGenerationResult { generated_text: String },
    /// Indicates an error occurred during inference.
    Error { message: String },
}
```

**Return Values:**

*   `ImageClassificationResult { class_labels: Vec<String>, probabilities: Vec<f32> }`: Returns a list of predicted class labels and their corresponding probabilities for image classification.
*   `TextGenerationResult { generated_text: String }`: Returns the generated text for text generation tasks.
*   `Error { message: String }`: An error occurred during the inference process, with a descriptive message.

## Functionality

The `model-runtime` V-Node performs the following key functions:

1.  **IPC Interface**: Exposes a clear IPC interface for other V-Nodes to request inference services.
2.  **Model Loading & Management**: Loads machine learning models from designated storage paths (e.g., `/models` from `svc://vfs`) into memory. It manages multiple loaded models identified by `model_id`.
3.  **Inference Execution**: Executes inference using the loaded models and provided input data. (Conceptual: This would involve specialized ML runtime libraries and potentially GPU interaction via `svc://gpu-driver`).
4.  **Resource Management**: Adheres to its configured `required_mem_mb` and `max_cpu_share`, dynamically managing memory and CPU resources for efficient inference execution.
5.  **Error Handling**: Catches and reports errors during model loading, data processing, or inference execution.
6.  **Observability**: Exposes metrics like `inference_requests_total`, `inference_latency_avg_ms`, and `gpu_utilization_percent` for monitoring performance.

## Usage Examples

### Example 1: Requesting Image Classification

```rust
// Pseudocode for client V-Node sending an image for classification

let mut model_runtime_chan = VNodeChannel::new(11); // IPC Channel to svc://model-runtime

// Dummy image data
let image_data = vec![0; 1024]; // Replace with actual image data

let request = InferRequest::ImageClassification {
    model_id: String::from("image_classifier_v1"),
    image_data,
};
match model_runtime_chan.send_and_recv::<InferRequest, InferResponse>(&request) {
    Ok(InferResponse::ImageClassificationResult { class_labels, probabilities }) => {
        log!("Image Classification Result:");
        for (label, prob) in class_labels.iter().zip(probabilities.iter()) {
            log!("- {}: {:.2}%", label, prob * 100.0);
        }
    },
    Ok(InferResponse::Error { message }) => {
        log!("Image classification error: {}", message);
    },
    _ => log!("Unexpected response from Model Runtime"),
}
```

### Example 2: Requesting Text Generation

```rust
// Pseudocode for client V-Node sending a prompt for text generation

let mut model_runtime_chan = VNodeChannel::new(11);

let request = InferRequest::TextGeneration {
    model_id: String::from("gpt-nano"),
    prompt: String::from("The quick brown fox "),
    max_tokens: 50,
};
match model_runtime_chan.send_and_recv::<InferRequest, InferResponse>(&request) {
    Ok(InferResponse::TextGenerationResult { generated_text }) => {
        log!("Generated Text: {}", generated_text);
    },
    Ok(InferResponse::Error { message }) => {
        log!("Text generation error: {}", message);
    },
    _ => log!("Unexpected response from Model Runtime"),
}
```

This documentation highlights the `model-runtime` V-Node's role as a powerful AI backend, enabling secure and efficient machine learning inference for applications within AetherOS.
