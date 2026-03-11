pub struct NodeId(pub [u8; 32]);

pub enum DhtValue {
    Manifest(super::examples::Manifest),
}

pub struct InMemoryDht;

impl InMemoryDht {
    pub fn new() -> Self { InMemoryDht }
    pub fn store(&self, _key: [u8; 32], _value: DhtValue) {}
}
