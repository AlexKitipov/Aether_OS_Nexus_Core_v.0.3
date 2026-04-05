pub mod id {
    pub type ChannelId = u32;
}

/// Well-known service channel identifiers used by ABI v2 domain syscalls.
///
/// These IDs are intentionally stable so user-space V-Nodes can target
/// system services without out-of-band discovery during early boot.
pub mod well_known {
    use super::id::ChannelId;

    pub const UI_COMPOSITOR: ChannelId = 0x0101;
    pub const UI_WEBVIEW: ChannelId = 0x0102;
    pub const DEV_INTERFACE: ChannelId = 0x010F;

    pub const VFS_SERVICE: ChannelId = 0x0201;
    pub const MAIL_SERVICE: ChannelId = 0x0202;

    pub const MODEL_RUNTIME: ChannelId = 0x0301;
    pub const AI_GOVERNOR: ChannelId = 0x0302;

    pub const SWARM_NET_BRIDGE: ChannelId = 0x0401;

    #[must_use]
    pub const fn is_ui(channel_id: ChannelId) -> bool {
        matches!(channel_id, UI_COMPOSITOR | UI_WEBVIEW | DEV_INTERFACE)
    }

    #[must_use]
    pub const fn is_vfs(channel_id: ChannelId) -> bool {
        matches!(channel_id, VFS_SERVICE | MAIL_SERVICE)
    }

    #[must_use]
    pub const fn is_ai(channel_id: ChannelId) -> bool {
        matches!(channel_id, MODEL_RUNTIME | AI_GOVERNOR)
    }

    #[must_use]
    pub const fn is_swarm(channel_id: ChannelId) -> bool {
        matches!(channel_id, SWARM_NET_BRIDGE)
    }
}
