
use serde::{Deserialize, Serialize};
use alloc::string::String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EchoRequest {
    Echo { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EchoResponse {
    EchoReply { message: String },
    Error(String),
}

impl EchoResponse {
    /// Creates an [`EchoResponse::EchoReply`] from an incoming request payload.
    pub fn from_request(request: EchoRequest) -> Self {
        match request {
            EchoRequest::Echo { message } => Self::EchoReply { message },
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{EchoRequest, EchoResponse};

    #[test]
    fn from_request_returns_echo_reply() {
        let request = EchoRequest::Echo {
            message: "hello, nexus".into(),
        };

        let response = EchoResponse::from_request(request);

        assert_eq!(
            response,
            EchoResponse::EchoReply {
                message: "hello, nexus".into(),
            }
        );
    }

    #[test]
    fn request_round_trip_serialization_works() {
        let request = EchoRequest::Echo {
            message: "ping".into(),
        };

        let encoded = postcard::to_allocvec(&request).expect("request should serialize");
        let decoded: EchoRequest = postcard::from_bytes(&encoded).expect("request should deserialize");

        assert_eq!(decoded, request);
    }

    #[test]
    fn response_round_trip_serialization_works() {
        let response = EchoResponse::EchoReply {
            message: "pong".into(),
        };

        let encoded = postcard::to_allocvec(&response).expect("response should serialize");
        let decoded: EchoResponse = postcard::from_bytes(&encoded).expect("response should deserialize");

        assert_eq!(decoded, response);
    }
}
