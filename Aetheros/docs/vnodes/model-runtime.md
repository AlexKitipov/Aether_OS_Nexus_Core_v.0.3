# Model Runtime V-Node

## Overview

The `model-runtime` V-Node is a specialized system service designed for efficient and secure execution of machine learning inference tasks within AetherOS. It provides an IPC interface for other V-Nodes to offload computationally intensive inference requests, leveraging underlying hardware (e.g., GPU) and managing model lifecycle.

## Core Responsibilities

*   **Model Loading and Management**: Loads ML models (e.g., pre-trained neural networks) from persistent storage (via `vfs`) into memory, caching them for subsequent inferences. It can handle various model formats.
*   **Inference Execution**: Executes inference requests (e.g., image classification, text generation) using the loaded models. This involves preprocessing input data, running the model, and post-processing results.
*   **IPC Interface for Inference**: Exposes an IPC API (`InferRequest`, `InferResponse`) that allows client V-Nodes to send inference queries and receive structured results.
*   **Resource Optimization**: Manages memory and CPU/GPU resources dedicated to model inference, ensuring efficient utilization and adherence to declared quotas.
*   **Hardware Acceleration (Conceptual)**: Interacts with conceptual `gpu-driver` V-Node for accelerated inference on specialized hardware.

## Capabilities and Dependencies

To perform its functions, the `model-runtime` V-Node requires specific capabilities:

*   `CAP_IPC_ACCEPT`: To accept inference requests from client V-Nodes (e.g., `webview`, `aetherterminal`, user applications).
*   `CAP_IPC_CONNECT: "svc://vfs"`: To load ML model binaries, configuration files, and potentially save/load model checkpoints from the filesystem.
*   `CAP_IPC_CONNECT: "svc://gpu-driver"`: (Conceptual) To interact with a dedicated GPU driver for hardware-accelerated inference, utilizing shared memory for efficient data transfer.
*   `CAP_LOG_WRITE`: For logging inference requests, performance metrics (e.g., latency), model loading events, and errors.
*   `CAP_TIME_READ`: For measuring inference latency, managing timeouts for long-running inferences, or scheduling periodic maintenance tasks.

## Operational Flow (High-Level)

1.  **Initialization**: Establishes IPC channels with client V-Nodes and `vfs` (and conceptually `gpu-driver`). Initializes an empty cache for loaded models.
2.  **Request Handling (`InferRequest`)**: 
    *   Receives `InferRequest` messages from client V-Nodes.
    *   For a given `model_id`, it first checks if the model is already loaded in its internal cache.
    *   If not cached, it attempts to load the model binary from `vfs` using a predefined path (e.g., `/models/<model_id>/<model_file>`).
    *   Once the model is ready, it simulates (or actually performs) the inference using the provided input data.
    *   Returns an `InferResponse` (e.g., `ImageClassificationResult`, `TextGenerationResult`) or an `Error` if the model cannot be loaded or inference fails.
3.  **Model Loading**: The `load_model` function uses `vfs_chan` to open, read, and close model files, ensuring proper access control and error handling.
4.  **Event Loop**: Continuously polls its client IPC channel for new inference requests and processes them. Uses `SYS_TIME` to yield control to the kernel, preventing busy-waiting.

## Example `vnode.yml` Configuration

```yaml
# vnode/model-runtime/vnode.yml
vnode:
  name: "model-runtime"
  version: "0.1.0"
  maintainer: "aetheros-core-team@aetheros.org"
  mode: strict # A critical system service for machine learning inference

runtime:
  entrypoint: "bin/model-runtime.vnode"
  required_mem_mb: 256 # For loading ML models and processing large inputs/outputs
  max_cpu_share: 0.50 # Can be very CPU/GPU intensive during inference

capabilities:
  - CAP_IPC_ACCEPT # To accept inference requests from client V-Nodes
  - CAP_IPC_CONNECT: "svc://vfs" # To load models from disk, save/load checkpoints
  - CAP_IPC_CONNECT: "svc://gpu-driver" # Conceptual: To interact with GPU for accelerated inference
  - CAP_LOG_WRITE # For logging inference requests, performance, and errors
  - CAP_TIME_READ # For measuring inference latency and managing timeouts

storage:
  mounts:
    - path: "/models"
      source: "aetherfs://system-models"
      options: [ "ro" ] # Read-only access to pre-trained system models
    - path: "/home/<AID>/models"
      source: "aetherfs://user/<AID>/models"
      options: [ "rw" ] # Read/write access for user-specific models or checkpoints
    - path: "/tmp"
      source: "volatile://ramdisk"
      size: "64MB" # For temporary data, intermediate inference results

observability:
  metrics: ["inference_requests_total", "inference_success_total", "inference_errors_total", "inference_latency_avg_ms", "model_loads_total", "gpu_utilization_percent"]
```
