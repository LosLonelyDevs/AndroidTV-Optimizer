use std::path::Path;

pub fn is_portable() -> bool {
    cfg!(target_os = "windows")
        && std::env::current_exe()
            .ok()
            .is_some_and(|exe| has_marker(&exe))
}

fn has_marker(exe: &Path) -> bool {
    exe.parent()
        .is_some_and(|dir| dir.join("portable.txt").is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_marker_belongs_to_the_executable_directory() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("atv-optimizer.exe");
        assert!(!has_marker(&exe));
        std::fs::create_dir(dir.path().join("portable.txt")).unwrap();
        assert!(!has_marker(&exe));
        std::fs::remove_dir(dir.path().join("portable.txt")).unwrap();
        std::fs::write(dir.path().join("portable.txt"), "Portable distribution").unwrap();
        assert!(has_marker(&exe));
        assert!(!has_marker(&dir.path().join("other/atv-optimizer.exe")));
    }
}
