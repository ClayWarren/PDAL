/// Run `cmd` through the system shell, capturing its standard output.
///
/// Returns the exit status (0 on success) and the captured stdout. Mirrors
/// `pdal::Utils::run_shell_command`, which runs via `popen(cmd, "r")` and so
/// captures stdout only.
pub fn run_shell_command(cmd: &str) -> (i32, String) {
    use std::process::Command;
    let (shell, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    match Command::new(shell).arg(flag).arg(cmd).output() {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout).into_owned();
            (output.status.code().unwrap_or(1), text)
        }
        Err(_) => (1, String::new()),
    }
}
