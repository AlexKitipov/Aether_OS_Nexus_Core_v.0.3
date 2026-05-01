use crate::codec::{decode_message, encode_response};
use crate::kernel_bridge::KernelBridge;
use crate::protocol::{GatewayMessage, GatewayResponse};

use reqwest::blocking::Client;
use serde_json::json;

pub fn run_gateway_loop<B>(bridge: &mut B) -> Result<(), Box<dyn std::error::Error>>
where
    B: KernelBridge,
    B::Error: std::error::Error + 'static,
{
    loop {
        let kernel_message = bridge.recv_kernel_message()?;
        let encoded = serde_json::to_vec(&kernel_message)?;
        let message = decode_message(&encoded).map_err(std::io::Error::other)?;
        let response = route_to_agent(message);
        bridge.send_kernel_message(GatewayMessage::AiAction {
            action_type: response.status,
            payload: response.payload,
        })?;
    }
}

fn route_to_agent(msg: GatewayMessage) -> GatewayResponse {
    let (port, endpoint, event_type, payload) = match msg {
        GatewayMessage::KernelEvent { event_type, payload } => {
            if event_type.starts_with("driver.") {
                (5151, "/process", event_type, payload)
            } else if event_type.starts_with("runtime.") {
                (5152, "/analyze", event_type, payload)
            } else if event_type.starts_with("package.") {
                (5153, "/generate", event_type, payload)
            } else {
                return GatewayResponse {
                    status: "error".into(),
                    payload: b"unknown intent".to_vec(),
                };
            }
        }
        _ => {
            return GatewayResponse {
                status: "error".into(),
                payload: b"invalid message".to_vec(),
            };
        }
    };

    let url = format!("http://127.0.0.1:{port}{endpoint}");
    let client = Client::new();
    let request_payload = json!({
        "event_type": event_type,
        "payload": payload,
    });

    match client.post(url).json(&request_payload).send() {
        Ok(response) => {
            if !response.status().is_success() {
                return GatewayResponse {
                    status: "error".into(),
                    payload: format!("http status {}", response.status()).into_bytes(),
                };
            }

            match response.bytes() {
                Ok(bytes) => {
                    let ndjson_payload = bytes.to_vec();
                    let normalized = encode_response(&GatewayResponse {
                        status: "ok".into(),
                        payload: ndjson_payload,
                    })
                    .unwrap_or_else(|e| {
                        format!("{{\"status\":\"error\",\"payload\":\"{}\"}}", e).into_bytes()
                    });

                    GatewayResponse {
                        status: "ok".into(),
                        payload: normalized,
                    }
                }
                Err(e) => GatewayResponse {
                    status: "error".into(),
                    payload: e.to_string().into_bytes(),
                },
            }
        }
        Err(e) => GatewayResponse {
            status: "error".into(),
            payload: e.to_string().into_bytes(),
        },
    }
}
