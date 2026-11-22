//! Windows on-demand UAC elevation fallback (for non-elevated dev builds).

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn to_wide_null(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

/// Re-launch the current executable with the `runas` verb to trigger a UAC prompt.
pub fn relaunch_as_admin() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("无法获取当前程序路径: {e}"))?;
    let verb = to_wide_null(OsStr::new("runas"));
    let file = to_wide_null(exe.as_os_str());

    let args: Vec<String> = std::env::args().skip(1).collect();
    let params_joined = args.join(" ");
    let params_wide = to_wide_null(OsStr::new(&params_joined));
    let params_ptr = if params_joined.is_empty() {
        std::ptr::null()
    } else {
        params_wide.as_ptr()
    };

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params_ptr,
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    // Per Win32 docs, ShellExecuteW returns a value > 32 on success
    if (result as isize) <= 32 {
        return Err("已取消或未能启动管理员权限窗口。".to_string());
    }

    Ok(())
}
