use std::ptr;

use super::guards::{HandleGuard, SidGuard};
use super::path::get_attrs;
use super::utils::to_wide;
use super::{
    AddAccessAllowedAce, AllocateAndInitializeSid, DACL_SECURITY_INFORMATION, DWORD,
    FILE_ATTRIBUTE_DIRECTORY, GENERIC_ALL, OWNER_SECURITY_INFORMATION, OpenProcessToken, PACL,
    PSID, SE_FILE_OBJECT, SetNamedSecurityInfoW, SidIdentifierAuthority, TOKEN_OWNER_CLASS,
    TOKEN_QUERY, TokenOwner,
};

pub(crate) fn take_ownership_and_grant(path: &str) -> bool {
    let attrs = get_attrs(path);
    if attrs == 0xFFFF_FFFF || (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0 {
        return false;
    }

    let p = std::path::Path::new(path);
    let parent = match p.parent() {
        Some(x) => x,
        None => return false,
    };
    if let Some(root) = p.ancestors().last()
        && parent == root
    {
        return false;
    }

    unsafe {
        let mut token: super::HANDLE = 0;
        if OpenProcessToken(super::GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let _guard = HandleGuard(token);

        let mut needed: DWORD = 0;
        super::GetTokenInformation(token, TOKEN_OWNER_CLASS, ptr::null_mut(), 0, &mut needed);
        let mut buf = vec![0u8; needed as usize];
        if super::GetTokenInformation(
            token,
            TOKEN_OWNER_CLASS,
            buf.as_mut_ptr() as _,
            needed,
            &mut needed,
        ) == 0
        {
            return false;
        }
        let owner_sid = (*(buf.as_ptr() as *const TokenOwner)).owner;

        let world_auth = SidIdentifierAuthority {
            value: [0, 0, 0, 0, 0, 1],
        };
        let mut everyone: PSID = ptr::null_mut();
        if AllocateAndInitializeSid(&world_auth, 1, 0, 0, 0, 0, 0, 0, 0, 0, &mut everyone) == 0 {
            return false;
        }
        let _sid_guard = SidGuard(everyone);

        let mut acl_buf = [0u8; 512];
        let pacl = acl_buf.as_mut_ptr() as PACL;
        super::InitializeAcl(pacl, 512, super::ACL_REVISION);
        AddAccessAllowedAce(pacl, super::ACL_REVISION, GENERIC_ALL, everyone);

        let wide = to_wide(path);
        let err = SetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            owner_sid,
            ptr::null_mut(),
            pacl,
            ptr::null_mut(),
        );
        err == 0
    }
}
