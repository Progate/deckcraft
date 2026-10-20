//! Reading and writing the files a command names by path.
//!
//! Every such read and write goes through [`read_file`] and [`write_file`], so the automation
//! roots ([`crate::file_access`]) are checked in one place. A command module that called
//! `std::fs` itself would bypass them; a test in `file_access` keeps that from happening.

/// The bytes of the file at `path`, inside the automation read root when there is one.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn read_file(path: &str) -> Result<Vec<u8>, String> {
    crate::file_access::check_read(path)?;
    std::fs::read(path).map_err(|e| format!("{path}: {e}"))
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub fn read_file(path: &str) -> Result<Vec<u8>, String> {
    Err(format!("{path}: files are opened through the browser on the web"))
}

/// Write `bytes` to the file at `path`, inside the automation write root when there is one.
///
/// The bytes go to a temporary sibling first and are renamed over `path`, so a failed save never
/// truncates the old file. The sibling is in the same folder, so it is inside the root whenever
/// `path` is.
///
/// A value that distinguishes one process's temp files from another's. WASI has no pids, so a
/// random per-process id (RandomState's seed is drawn per process) stands in for `std::process::id`.
pub(crate) fn process_tag() -> u32 {
    #[cfg(not(target_os = "wasi"))]
    {
        std::process::id()
    }
    #[cfg(target_os = "wasi")]
    {
        use std::hash::{BuildHasher, Hasher};
        static TAG: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
        *TAG.get_or_init(|| std::collections::hash_map::RandomState::new().build_hasher().finish() as u32)
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn write_file(path: &str, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    crate::file_access::check_write(path)?;
    let err = |e: std::io::Error| format!("{path}: {e}");
    // A uniquely named temporary sibling, created exclusively (never following or reusing an
    // existing file or symlink), is renamed over the destination: concurrent saves never share
    // temporary bytes.
    let mut attempts = 0;
    let (tmp, mut file) = loop {
        let tmp = format!("{path}.{}.{}.saving", process_tag(), COUNTER.fetch_add(1, Ordering::Relaxed));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp) {
            Ok(f) => break (tmp, f),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && attempts < 16 => attempts += 1,
            Err(e) => return Err(err(e)),
        }
    };
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let result = written.and_then(|()| std::fs::rename(&tmp, path));
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(err(e));
    }
    Ok(())
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub fn write_file(path: &str, _bytes: &[u8]) -> Result<(), String> {
    Err(format!("{path}: saving to a path is not available in the browser"))
}

/// The size in bytes of the file at `path`; `None` when it can't be read (or is outside the
/// automation read root, so File › Info doesn't reveal files the agent may not read).
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn file_len(path: &str) -> Option<u64> {
    crate::file_access::check_read(path).ok()?;
    std::fs::metadata(path).ok().map(|m| m.len())
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub fn file_len(_path: &str) -> Option<u64> {
    None
}
