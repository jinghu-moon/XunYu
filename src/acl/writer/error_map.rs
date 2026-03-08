use anyhow::{Context, Result};

use crate::acl::error::AclError;

pub(super) fn check_win32(
    status: windows::Win32::Foundation::WIN32_ERROR,
    context: impl Into<String>,
) -> Result<()> {
    if status.0 != 0 {
        let err = anyhow::Error::new(AclError::from_win32(status.0));
        return Err(err).context(context.into());
    }
    Ok(())
}
