// vnode/model-runtime/src/main.rs

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};

use common::ipc::vnode::VNodeChannel;
use common::syscall::{syscall3, SYS_LOG, SUCCESS, SYS_TIME};
use common::ipc::model_runtime_ipc::{InferRequest, InferResponse};
use common::ipc::vfs_ipc::{VfsRequest, VfsResponse, Fd, VfsMetadata}; // For loading models

// Temporary log function for V-Nodes
fn log(msg: &str) {
    unsafe {
        let res = syscall3(
            SYS_LOG,
            msg.as_ptr() as u64,
            msg.len() as u64,
            0 // arg3 is unused for SYS_LOG
        );
        if res != SUCCESS { /* Handle log error, maybe panic or fall back */ }
    }
}

// Placeholder for a loaded ML model
struct LoadedModel {
    model_id: String,
    data: Vec<u8>, // Raw model bytes
    // Add more metadata, e.g., type of model, input/output shapes
}

struct ModelRuntimeService {
    client_chan: VNodeChannel, // Channel for client V-Nodes sending inference requests
    vfs_chan: VNodeChannel,    // Channel to svc://vfs for loading models

    loaded_models: BTreeMap<String, LoadedModel>, // model_id -> LoadedModel
}

impl ModelRuntimeService {
    fn new(client_chan_id: u32, vfs_chan_id: u32) -> Self {
        let client_chan = VNodeChannel::new(client_chan_id);
        let vfs_chan = VNodeChannel::new(vfs_chan_id);

        log("Model Runtime Service: Initializing...");

        Self {
            client_chan,
            vfs_chan,
            loaded_models: BTreeMap::new(),
        }
    }

    // Conceptual: Load a model from VFS
    fn load_model(&mut self, model_id: &str, path: &str) -> Result<&LoadedModel, String> {
        if let Some(model) = self.loaded_models.get(model_id) {
            log(&alloc::format!("Model Runtime: Model '{}' already loaded.", model_id));
            return Ok(model);
        }

        log(&alloc::format!("Model Runtime: Loading model '{}' from VFS path '{}'.", model_id, path));
        
        // Simulate opening the model file
        let open_req = VfsRequest::Open { path: path.to_string(), flags: 0 }; // 0 for O_RDONLY
        let fd: Fd = match self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&open_req) {
            Ok(VfsResponse::Success(file_fd)) => file_fd as Fd,
            Ok(VfsResponse::Error { message, .. }) => return Err(alloc::format!("Failed to open model file: {}.", message)),
            _ => return Err(String::from("Unexpected VFS response during model open.")),
        };

        // Simulate reading the model data
        let read_req = VfsRequest::Read { fd, len: 1_000_000, offset: 0 }; // Assume max model size 1MB
        let model_data: Vec<u8> = match self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&read_req) {
            Ok(VfsResponse::Data(data)) => data,
            Ok(VfsResponse::Error { message, .. }) => {
                let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd });
                return Err(alloc::format!("Failed to read model data: {}.", message));
            },
            _ => {
                let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd });
                return Err(String::from("Unexpected VFS response during model read.")),
            },
        };

        // Close the model file
        let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd });

        if model_data.is_empty() {
            return Err(String::from("Model file is empty."));
        }

        let loaded_model = LoadedModel { model_id: model_id.to_string(), data: model_data };
        self.loaded_models.insert(model_id.to_string(), loaded_model);
        Ok(self.loaded_models.get(model_id).unwrap())
    }

    fn handle_request(&mut self, request: InferRequest) -> InferResponse {
        match request {
            InferRequest::ImageClassification { model_id, image_data } => {
                log(&alloc::format!("Model Runtime: Image classification request for model '{}'.", model_id));
                
                // Attempt to load the model (or retrieve from cache)
                let model = match self.load_model(&model_id, &alloc::format!("/models/{}/image_classifier.bin", model_id)) {
                    Ok(m) => m,
                    Err(e) => return InferResponse::Error(alloc::format!("Failed to load model: {}.", e)),
                };

                // Simulate inference
                log(&alloc::format!("Model Runtime: Performing image classification on {} bytes of image data using model '{}'.", image_data.len(), model.model_id));
                InferResponse::ImageClassificationResult {
                    class_labels: vec!["cat".to_string(), "dog".to_string()],
                    probabilities: vec![0.9, 0.1],
                }
            },
            InferRequest::TextGeneration { model_id, prompt, max_tokens } => {
                log(&alloc::format!("Model Runtime: Text generation request for model '{}' with prompt: '{}'.", model_id, prompt));
                
                // Attempt to load the model (or retrieve from cache)
                let model = match self.load_model(&model_id, &alloc::format!("/models/{}/text_generator.bin", model_id)) {
                    Ok(m) => m,
                    Err(e) => return InferResponse::Error(alloc::format!("Failed to load model: {}.", e)),
                };

                // Simulate inference
                log(&alloc::format!("Model Runtime: Generating {} tokens for prompt: '{}' using model '{}'.", max_tokens, prompt, model.model_id));
                InferResponse::TextGenerationResult { generated_text: alloc::format!("This is a generated text based on the prompt: '{}'. It is generated by model {}.", prompt, model.model_id) }
            },
        }
    }

    fn run_loop(&mut self) -> ! {
        log("Model Runtime Service: Entering main event loop.");
        loop {
            // Process incoming requests from client V-Nodes
            if let Ok(Some(req_data)) = self.client_chan.recv_non_blocking() {
                if let Ok(request) = postcard::from_bytes::<InferRequest>(&req_data) {
                    log(&alloc::format!("Model Runtime Service: Received InferRequest: {:?}.", request));
                    let response = self.handle_request(request);
                    self.client_chan.send(&response).unwrap_or_else(|_| log("Model Runtime Service: Failed to send response to client."));
                } else {
                    log("Model Runtime Service: Failed to deserialize InferRequest.");
                }
            }

            // Yield to other V-Nodes to prevent busy-waiting
            unsafe { syscall3(SYS_TIME, 0, 0, 0); } // This will cause a context switch
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Assuming channel IDs:
    // 11 for Model Runtime Service client requests
    // 7 for VFS Service
    let mut model_runtime_service = ModelRuntimeService::new(11, 7);
    model_runtime_service.run_loop();
}

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo) -> ! {
    log(&alloc::format!("Model Runtime V-Node panicked! Info: {:?}.", info));
    // In a production system, this might trigger a system-wide error handler or reboot.
    // For now, it enters an infinite loop to prevent further execution.
    loop {}
}