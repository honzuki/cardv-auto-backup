use windows::Win32::Storage::FileSystem::GetLogicalDrives;

/// Lists all the logical drives that are currently mounted
pub fn list_all_logical_drives() -> Vec<char> {
    // Safety: !
    let mut mask = unsafe { GetLogicalDrives() };

    let mut drives = vec![];
    for ch in 'A'..='Z' {
        if mask & 1 != 0 {
            drives.push(ch);
        }
        mask >>= 1;
    }

    drives
}

#[cfg(test)]
mod tests {
    use crate::list_all_logical_drives;

    #[test]
    fn check_for_c_drive() {
        assert!(list_all_logical_drives().contains(&'C'))
    }
}
