/// Run `cmd` through the system shell, capturing its standard output.
///
/// Returns the exit status (0 on success) and the captured stdout. Mirrors
/// `pdal::Utils::run_shell_command`, which runs via `popen(cmd, "r")` and so
/// captures stdout only.
pub fn run_shell_command(cmd: &str) -> (i32, String) {
    #[cfg(unix)]
    {
        run_shell_command_unix(cmd)
    }
    #[cfg(windows)]
    {
        run_shell_command_windows(cmd)
    }
}

#[cfg(unix)]
fn run_shell_command_unix(cmd: &str) -> (i32, String) {
    use std::ffi::{c_char, c_int, c_void, CString};

    unsafe extern "C" {
        fn popen(command: *const c_char, mode: *const c_char) -> *mut c_void;
        fn pclose(stream: *mut c_void) -> c_int;
        fn fread(ptr: *mut c_void, size: usize, nmemb: usize, stream: *mut c_void) -> usize;
    }

    let command = match CString::new(cmd) {
        Ok(command) => command,
        Err(_) => return (1, String::new()),
    };
    let mode = c"r";
    let stream = unsafe { popen(command.as_ptr(), mode.as_ptr()) };
    if stream.is_null() {
        return (1, String::new());
    }

    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let read = unsafe {
            fread(
                buffer.as_mut_ptr().cast::<c_void>(),
                1,
                buffer.len(),
                stream,
            )
        };
        if read == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..read]);
    }

    let status = unsafe { pclose(stream) };
    let code = if status == -1 {
        1
    } else if status & 0x7f == 0 {
        (status >> 8) & 0xff
    } else {
        1
    };
    (code, String::from_utf8_lossy(&output).into_owned())
}

#[cfg(windows)]
fn run_shell_command_windows(cmd: &str) -> (i32, String) {
    use std::ffi::{c_char, c_int, c_void, CString};

    unsafe extern "C" {
        fn _popen(command: *const c_char, mode: *const c_char) -> *mut c_void;
        fn _pclose(stream: *mut c_void) -> c_int;
        fn fread(ptr: *mut c_void, size: usize, nmemb: usize, stream: *mut c_void) -> usize;
    }

    let command = match CString::new(windows_popen_command(cmd)) {
        Ok(command) => command,
        Err(_) => return (1, String::new()),
    };
    let mode = c"r";
    let stream = unsafe { _popen(command.as_ptr(), mode.as_ptr()) };
    if stream.is_null() {
        return (1, String::new());
    }

    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let read = unsafe {
            fread(
                buffer.as_mut_ptr().cast::<c_void>(),
                1,
                buffer.len(),
                stream,
            )
        };
        if read == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..read]);
    }

    let status = unsafe { _pclose(stream) };
    let code = if status == -1 { 1 } else { status };
    (code, String::from_utf8_lossy(&output).into_owned())
}

#[cfg(windows)]
fn windows_popen_command(cmd: &str) -> String {
    if cmd.trim_start().starts_with('"') {
        format!("\"{cmd}\"")
    } else {
        cmd.to_string()
    }
}
