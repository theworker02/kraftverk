//! Local IPC transport: named pipe (Windows) / Unix socket (Linux).

use std::io::{Read, Write};

use kraftverk_core::error::{Error, Result};

use crate::protocol::FRAME_MAGIC;

pub fn default_endpoint() -> String {
    #[cfg(windows)]
    {
        r"\\.\pipe\kraftverk-agent".to_string()
    }
    #[cfg(unix)]
    {
        crate::auth::agent_data_dir()
            .map(|d| d.join("agent.sock").to_string_lossy().into_owned())
            .unwrap_or_else(|_| "/tmp/kraftverk-agent.sock".into())
    }
    #[cfg(not(any(windows, unix)))]
    {
        "unsupported".into()
    }
}

pub fn write_frame(w: &mut impl Write, payload: &[u8]) -> Result<()> {
    if payload.len() > 16 * 1024 * 1024 {
        return Err(Error::Platform("IPC frame too large".into()));
    }
    w.write_all(FRAME_MAGIC)?;
    w.write_all(&(payload.len() as u32).to_le_bytes())?;
    w.write_all(payload)?;
    w.flush()?;
    Ok(())
}

pub fn read_frame(r: &mut impl Read) -> Result<Vec<u8>> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)?;
    if &magic != FRAME_MAGIC {
        return Err(Error::Platform(
            "IPC magic mismatch (not kraftverk-agent)".into(),
        ));
    }
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > 16 * 1024 * 1024 {
        return Err(Error::Platform("IPC frame length invalid".into()));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn write_json<T: serde::Serialize>(w: &mut impl Write, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    write_frame(w, &bytes)
}

pub fn read_json<T: serde::de::DeserializeOwned>(r: &mut impl Read) -> Result<T> {
    let bytes = read_frame(r)?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(unix)]
pub mod unix_sock {
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::Path;

    use kraftverk_core::error::{Error, Result};

    pub fn listen(path: &str) -> Result<UnixListener> {
        let p = Path::new(path);
        if p.exists() {
            let _ = std::fs::remove_file(p);
        }
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let listener =
            UnixListener::bind(p).map_err(|e| Error::Platform(format!("unix bind {path}: {e}")))?;
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600));
        }
        Ok(listener)
    }

    pub fn accept(listener: &UnixListener) -> Result<UnixStream> {
        listener
            .accept()
            .map(|(s, _)| s)
            .map_err(|e| Error::Platform(format!("unix accept: {e}")))
    }

    pub fn connect(path: &str) -> Result<UnixStream> {
        UnixStream::connect(path).map_err(|e| Error::Platform(format!("unix connect: {e}")))
    }
}

#[cfg(windows)]
pub mod win_pipe {
    use std::ffi::OsStr;
    use std::io::{Read, Write};
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    use kraftverk_core::error::{Error, Result};
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, BOOL, ERROR_PIPE_CONNECTED, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Pipes::{
        CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
        PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };

    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const PIPE_ACCESS_DUPLEX: u32 = 0x0000_0003;

    #[link(name = "kernel32")]
    extern "system" {
        fn ConnectNamedPipe(handle: HANDLE, overlapped: *mut core::ffi::c_void) -> BOOL;
        fn ReadFile(
            handle: HANDLE,
            buffer: *mut u8,
            number_of_bytes_to_read: u32,
            number_of_bytes_read: *mut u32,
            overlapped: *mut core::ffi::c_void,
        ) -> BOOL;
        fn WriteFile(
            handle: HANDLE,
            buffer: *const u8,
            number_of_bytes_to_write: u32,
            number_of_bytes_written: *mut u32,
            overlapped: *mut core::ffi::c_void,
        ) -> BOOL;
        fn FlushFileBuffers(handle: HANDLE) -> BOOL;
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    pub struct PipeStream {
        handle: HANDLE,
        server: bool,
    }

    impl PipeStream {
        pub fn listen_and_accept(name: &str) -> Result<Self> {
            let w = wide(name);
            let handle = unsafe {
                CreateNamedPipeW(
                    w.as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                    PIPE_UNLIMITED_INSTANCES,
                    64 * 1024,
                    64 * 1024,
                    0,
                    ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                return Err(Error::Platform(format!(
                    "CreateNamedPipeW failed for {name}"
                )));
            }
            let ok = unsafe { ConnectNamedPipe(handle, ptr::null_mut()) };
            if ok == 0 {
                let err = unsafe { GetLastError() };
                if err != ERROR_PIPE_CONNECTED {
                    unsafe {
                        CloseHandle(handle);
                    }
                    return Err(Error::Platform(format!(
                        "ConnectNamedPipe failed (err={err})"
                    )));
                }
            }
            Ok(Self {
                handle,
                server: true,
            })
        }

        pub fn connect(name: &str) -> Result<Self> {
            let w = wide(name);
            let handle = unsafe {
                CreateFileW(
                    w.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    ptr::null(),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    core::ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                return Err(Error::Platform(format!(
                    "agent pipe not available at {name} (is `kraftverk agent serve` running elevated if needed?)"
                )));
            }
            Ok(Self {
                handle,
                server: false,
            })
        }
    }

    impl Read for PipeStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut read = 0u32;
            let ok = unsafe {
                ReadFile(
                    self.handle,
                    buf.as_mut_ptr() as *mut _,
                    buf.len() as u32,
                    &mut read,
                    ptr::null_mut(),
                )
            };
            if ok == 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(read as usize)
        }
    }

    impl Write for PipeStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut written = 0u32;
            let ok = unsafe {
                WriteFile(
                    self.handle,
                    buf.as_ptr() as *const _,
                    buf.len() as u32,
                    &mut written,
                    ptr::null_mut(),
                )
            };
            if ok == 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(written as usize)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            let ok = unsafe { FlushFileBuffers(self.handle) };
            if ok == 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    impl Drop for PipeStream {
        fn drop(&mut self) {
            if self.server {
                unsafe {
                    let _ = DisconnectNamedPipe(self.handle);
                }
            }
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }

    // send_recv helpers live on the client via write_json/read_json.
}
