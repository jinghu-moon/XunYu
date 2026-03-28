pub(crate) fn checked_len_to_usize(len: u64, context: &'static str) -> Result<usize, String> {
    usize::try_from(len).map_err(|_| format!("{context} too large: {len} bytes"))
}

pub(crate) fn allocate_zeroed_buffer(len: u64, context: &'static str) -> Result<Vec<u8>, String> {
    let len = checked_len_to_usize(len, context)?;
    let mut buf = Vec::new();
    buf.try_reserve_exact(len)
        .map_err(|err| format!("failed to allocate {context} buffer ({len} bytes): {err:?}"))?;
    buf.resize(len, 0);
    Ok(buf)
}

pub(crate) fn reserve_buffer_capacity(
    buf: &mut Vec<u8>,
    len: u64,
    context: &'static str,
) -> Result<(), String> {
    let len = checked_len_to_usize(len, context)?;
    buf.try_reserve_exact(len)
        .map_err(|err| format!("failed to reserve {context} buffer ({len} bytes): {err:?}"))
}
