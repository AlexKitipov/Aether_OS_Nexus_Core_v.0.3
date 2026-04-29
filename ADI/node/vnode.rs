pub struct VirtualNode {
    pub id: u64,
}

pub fn create_vnode(id: u64) -> VirtualNode {
    VirtualNode { id }
}
