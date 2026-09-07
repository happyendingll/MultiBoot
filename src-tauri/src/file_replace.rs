use std::{io, path::Path};

/// Replaces `destination` with `source`, including when the destination already
/// exists. `std::fs::rename` has that behavior on Unix, while Windows requires
/// the replace flag explicitly.
pub fn replace(source: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        replace_windows(source, destination)
    }

    #[cfg(not(windows))]
    {
        std::fs::rename(source, destination)
    }
}

#[cfg(windows)]
fn replace_windows(source: &Path, destination: &Path) -> io::Result<()> {
    use std::{iter, os::windows::ffi::OsStrExt};

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new_name: *const u16, flags: u32) -> i32;
    }

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();
    // Both pointers remain valid for the call and are NUL-terminated UTF-16.
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn replaces_an_existing_file() {
        let directory =
            std::env::temp_dir().join(format!("multiboot-replace-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("source.tmp");
        let destination = directory.join("destination.json");
        fs::write(&source, "new").unwrap();
        fs::write(&destination, "old").unwrap();

        replace(&source, &destination).unwrap();

        assert_eq!(fs::read_to_string(&destination).unwrap(), "new");
        assert!(!source.exists());
        fs::remove_dir_all(directory).unwrap();
    }
}
