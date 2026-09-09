use base64::Engine;
use std::collections::HashSet;

/// 16 OS-random bytes, URL-safe base64 without padding.
pub fn new_id() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    fill_os_random(&mut bytes)?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

pub fn new_id_avoiding(occupied: &HashSet<String>) -> Result<String, String> {
    for _ in 0..32 {
        let id = new_id()?;
        if !occupied.contains(&id) {
            return Ok(id);
        }
    }
    Err("could not allocate a unique id".into())
}

fn fill_os_random(buf: &mut [u8]) -> Result<(), String> {
    #[cfg(windows)]
    {
        if try_rtl_gen_random(buf) {
            return Ok(());
        }
        if try_bcrypt_gen_random(buf) {
            return Ok(());
        }
        return Err("OS RNG failed".into());
    }
    #[cfg(unix)]
    {
        use std::io::Read;
        std::fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(buf))
            .map_err(|e| format!("OS RNG failed: {e}"))
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = buf;
        Err("OS RNG failed".into())
    }
}

#[cfg(windows)]
#[link(name = "advapi32")]
extern "system" {
    fn SystemFunction036(random_buffer: *mut u8, random_buffer_length: u32) -> u8;
}

#[cfg(windows)]
fn try_rtl_gen_random(buf: &mut [u8]) -> bool {
    if buf.is_empty() {
        return true;
    }
    unsafe { SystemFunction036(buf.as_mut_ptr(), buf.len() as u32) != 0 }
}

#[cfg(windows)]
#[link(name = "bcrypt")]
extern "system" {
    fn BCryptGenRandom(
        h_algorithm: *mut core::ffi::c_void,
        pb_buffer: *mut u8,
        cb_buffer: u32,
        dw_flags: u32,
    ) -> i32;
}

#[cfg(windows)]
fn try_bcrypt_gen_random(buf: &mut [u8]) -> bool {
    const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;
    if buf.is_empty() {
        return true;
    }
    unsafe {
        BCryptGenRandom(
            std::ptr::null_mut(),
            buf.as_mut_ptr(),
            buf.len() as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        ) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::{new_id, new_id_avoiding};
    use std::collections::HashSet;

    #[test]
    fn ids_are_url_safe_unpadded_and_unique() {
        let a = new_id().unwrap();
        let b = new_id().unwrap();
        assert_ne!(a, b);
        for id in [&a, &b] {
            assert_eq!(id.len(), 22);
            assert!(!id.contains('='));
            assert!(id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        }
    }

    #[test]
    fn avoiding_retries_until_free() {
        let id = new_id().unwrap();
        let occupied = HashSet::from([id.clone()]);
        let next = new_id_avoiding(&occupied).unwrap();
        assert_ne!(next, id);
    }
}
