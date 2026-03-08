use super::*;

pub(super) fn patch_subsystem_gui(bytes: &mut [u8]) -> Result<()> {
    if bytes.len() < 0x40 {
        anyhow::bail!("PE image too small");
    }
    let mz = u16::from_le_bytes([bytes[0], bytes[1]]);
    if mz != 0x5a4d {
        anyhow::bail!("invalid MZ signature");
    }
    let e_lfanew =
        u32::from_le_bytes([bytes[0x3c], bytes[0x3d], bytes[0x3e], bytes[0x3f]]) as usize;
    if bytes.len() < e_lfanew + 4 + 20 + 70 {
        anyhow::bail!("PE header out of range");
    }
    let pe = u32::from_le_bytes([
        bytes[e_lfanew],
        bytes[e_lfanew + 1],
        bytes[e_lfanew + 2],
        bytes[e_lfanew + 3],
    ]);
    if pe != 0x0000_4550 {
        anyhow::bail!("invalid PE signature");
    }
    let opt = e_lfanew + 4 + 20;
    let magic = u16::from_le_bytes([bytes[opt], bytes[opt + 1]]);
    if magic != 0x10b && magic != 0x20b {
        anyhow::bail!("unsupported optional header magic");
    }
    let subsystem_off = opt + 68;
    bytes[subsystem_off] = 2;
    bytes[subsystem_off + 1] = 0;
    Ok(())
}
