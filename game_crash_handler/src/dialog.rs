pub(super) fn dialog(text: &str) {
    #[cfg(unix)]
    unix::dialog(text);
}

#[cfg(unix)]
mod unix {
    use std::process::Command;

    pub fn dialog(text: &str) {
        let _ = Command::new("zenity")
            .args(["--error", &format!("--text={}", text)])
            .spawn();
    }
}
