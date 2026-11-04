// SPDX-License-Identifier: Apache-2.0
//! Single instance with argv forwarding.
//!
//! - Windows: a named mutex (`Local\<app id>`) elects the primary instance.
//!   The primary listens on a loopback TCP port; port and a random token are
//!   written to `<config_dir>/instance.lock` so secondaries can forward argv.
//! - Linux/macOS: a Unix domain socket in the runtime directory (or the
//!   temporary directory). A successful connect means a primary exists.
//!
//! Messages are one JSON array of strings per connection.

use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::{AppInfo, Result};

type Waker = Arc<Mutex<Option<Box<dyn Fn() + Send + 'static>>>>;

/// Outcome of [`acquire`].
pub enum Instance {
    /// This process is the primary instance.
    Primary(PrimaryInstance),
    /// Another instance is running and received this process's argv.
    /// The caller should exit.
    Forwarded,
}

/// Handle owned by the primary instance; dropping it releases the lock.
pub struct PrimaryInstance {
    rx: Receiver<Vec<String>>,
    waker: Waker,
    _guard: platform::Guard,
}

impl PrimaryInstance {
    /// Returns the next forwarded argv, if any.
    pub fn try_recv(&self) -> Option<Vec<String>> {
        self.rx.try_recv().ok()
    }

    /// Drains every pending forwarded argv.
    pub fn drain(&self) -> Vec<Vec<String>> {
        self.rx.try_iter().collect()
    }

    pub fn receiver(&self) -> &Receiver<Vec<String>> {
        &self.rx
    }

    /// Called from the listener thread after each forwarded message, e.g. to
    /// request a repaint or wake an event loop proxy.
    pub fn set_waker(&self, waker: impl Fn() + Send + 'static) {
        if let Ok(mut slot) = self.waker.lock() {
            *slot = Some(Box::new(waker));
        }
    }
}

/// Acquires the single-instance lock for `app`, forwarding `std::env::args()`
/// when another instance already owns it.
pub fn acquire(app: &AppInfo) -> Result<Instance> {
    acquire_with_args(app, std::env::args().collect())
}

/// Same as [`acquire`] with explicit arguments to forward.
pub fn acquire_with_args(app: &AppInfo, args: Vec<String>) -> Result<Instance> {
    let (tx, rx) = mpsc::channel();
    let waker: Waker = Arc::new(Mutex::new(None));
    match platform::acquire(app, &args, tx, waker.clone())? {
        Some(guard) => Ok(Instance::Primary(PrimaryInstance { rx, waker, _guard: guard })),
        None => Ok(Instance::Forwarded),
    }
}

fn encode(args: &[String]) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(args)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn serve_connection<S: std::io::Read>(stream: S, token: Option<&str>, tx: &Sender<Vec<String>>, waker: &Waker) {
    let mut reader = BufReader::new(stream);
    if let Some(token) = token {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim_end() != token {
            return;
        }
    }
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    if let Ok(args) = serde_json::from_str::<Vec<String>>(line.trim_end())
        && tx.send(args).is_ok()
        && let Ok(slot) = waker.lock()
        && let Some(wake) = slot.as_ref()
    {
        wake();
    }
}

#[allow(dead_code)]
fn send_all<W: Write>(mut stream: W, token: Option<&str>, args: &[String]) -> Result<()> {
    if let Some(token) = token {
        stream.write_all(token.as_bytes())?;
        stream.write_all(b"\n")?;
    }
    stream.write_all(&encode(args)?)?;
    stream.flush()?;
    Ok(())
}

#[cfg(unix)]
mod platform {
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::sync::mpsc::Sender;

    use super::{Waker, send_all, serve_connection};
    use crate::{AppInfo, Result};

    pub struct Guard {
        path: PathBuf,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn socket_path(app: &AppInfo) -> PathBuf {
        let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
        let name = format!("{}-{}.sock", app.slug(), user);
        app.project_dirs()
            .ok()
            .and_then(|dirs| dirs.runtime_dir().map(|d| d.to_path_buf()))
            .and_then(|dir| std::fs::create_dir_all(&dir).ok().map(|_| dir.join("instance.sock")))
            .unwrap_or_else(|| std::env::temp_dir().join(name))
    }

    pub fn acquire(app: &AppInfo, args: &[String], tx: Sender<Vec<String>>, waker: Waker) -> Result<Option<Guard>> {
        let path = socket_path(app);
        if let Ok(stream) = UnixStream::connect(&path) {
            send_all(stream, None, args)?;
            return Ok(None);
        }
        // Stale socket from a crashed primary.
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path)?;
        std::thread::Builder::new().name("yolo-single-instance".into()).spawn(move || {
            for stream in listener.incoming().flatten() {
                serve_connection(stream, None, &tx, &waker);
            }
        })?;
        Ok(Some(Guard { path }))
    }
}

#[cfg(windows)]
mod platform {
    use std::hash::{BuildHasher, Hasher};
    use std::net::{Ipv4Addr, TcpListener, TcpStream};
    use std::path::PathBuf;
    use std::sync::mpsc::Sender;
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    use super::{Waker, send_all, serve_connection};
    use crate::{AppInfo, Error, Result};

    pub struct Guard {
        mutex: usize,
        lock_file: PathBuf,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.lock_file);
            #[allow(unsafe_code)]
            // SAFETY: `mutex` is a handle returned by CreateMutexW and owned by this guard.
            unsafe {
                CloseHandle(self.mutex as HANDLE);
            }
        }
    }

    fn random_token() -> String {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        hasher.write_u32(std::process::id());
        let a = hasher.finish();
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u64(a);
        format!("{a:016x}{:016x}", hasher.finish())
    }

    pub fn acquire(app: &AppInfo, args: &[String], tx: Sender<Vec<String>>, waker: Waker) -> Result<Option<Guard>> {
        let name: Vec<u16> = format!("Local\\{}", app.slug()).encode_utf16().chain(Some(0)).collect();
        #[allow(unsafe_code)]
        // SAFETY: `name` is a NUL-terminated UTF-16 string that outlives the call.
        let (mutex, exists) = unsafe {
            let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
            (handle, GetLastError() == ERROR_ALREADY_EXISTS)
        };
        if mutex.is_null() {
            return Err(std::io::Error::last_os_error().into());
        }
        let lock_file = app.config_dir()?.join("instance.lock");

        if exists {
            #[allow(unsafe_code)]
            // SAFETY: the handle was just returned by CreateMutexW.
            unsafe {
                CloseHandle(mutex);
            }
            // The primary may still be starting: wait briefly for its lock file.
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                if let Ok(content) = std::fs::read_to_string(&lock_file)
                    && let Some((port, token)) = content.trim().split_once(' ')
                    && let Ok(port) = port.parse::<u16>()
                    && let Ok(stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
                {
                    send_all(stream, Some(token), args)?;
                    return Ok(None);
                }
                if Instant::now() >= deadline {
                    return Err(Error::Unsupported("primary instance did not publish its forwarding port"));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let token = random_token();
        std::fs::write(&lock_file, format!("{port} {token}"))?;
        std::thread::Builder::new().name("yolo-single-instance".into()).spawn(move || {
            for stream in listener.incoming().flatten() {
                serve_connection(stream, Some(&token), &tx, &waker);
            }
        })?;
        Ok(Some(Guard { mutex: mutex as usize, lock_file }))
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use std::sync::mpsc::Sender;

    use super::Waker;
    use crate::{AppInfo, Result};

    pub struct Guard;

    pub fn acquire(_app: &AppInfo, _args: &[String], _tx: Sender<Vec<String>>, _waker: Waker) -> Result<Option<Guard>> {
        Ok(Some(Guard))
    }
}
