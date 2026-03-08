#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub pid: u32,
    pub name: String,
    pub exe_path: String,
    pub cmdline: String,
    pub cwd: String,
    pub protocol: Protocol,
}

pub(super) type RawPortEntry = (u16, u32, Protocol);
