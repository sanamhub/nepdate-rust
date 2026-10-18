//! Single instance (WR-09 AC1, W3-06). The first instance listens on a local socket. A second
//! instance connects, sends `open\n` and exits 0. The listener runs on a background thread and
//! sets a flag that a small UI timer turns into "open the popup", because a Slint component is
//! not `Send`.
//!
//! The name is `mitibar-<username-hash>.sock` in the OS namespace when available (Windows,
//! Linux). macOS has no namespace, so the socket is a file in the data-local directory. A stale
//! file is removed and the listener retried.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, Listener, ListenerOptions, Stream,
};

/// What this process is.
pub enum Claim {
    /// The first instance; keep the listener.
    Primary(Listener),
    /// Another instance is running; it was told to open.
    Secondary,
}

/// The socket address, kept as data so the `Name` can borrow it locally.
enum Address {
    Namespaced(String),
    File(PathBuf),
}

/// Claims the app's socket, or signals the running instance.
///
/// # Errors
///
/// Returns an error when the socket cannot be created or an existing one cannot be checked.
pub fn claim() -> anyhow::Result<Claim> {
    let address = address()?;
    claim_at(&address)
}

/// Starts the listener thread. Each `open\n` sets the flag read by [`take_request`].
pub fn start(listener: Listener) {
    std::thread::spawn(move || {
        for connection in listener.incoming() {
            let Ok(stream) = connection else {
                break;
            };
            let mut line = String::new();
            if BufReader::new(stream).read_line(&mut line).is_ok() && line.trim() == "open" {
                OPEN_REQUESTED.store(true, Ordering::SeqCst);
            }
        }
    });
}

/// Takes the pending "open" request, if any.
#[must_use]
pub fn take_request() -> bool {
    OPEN_REQUESTED.swap(false, Ordering::SeqCst)
}

static OPEN_REQUESTED: AtomicBool = AtomicBool::new(false);

fn claim_at(address: &Address) -> anyhow::Result<Claim> {
    if let Ok(listener) = create(address) {
        return Ok(Claim::Primary(listener));
    }
    if signal_at(address).is_ok() {
        return Ok(Claim::Secondary);
    }
    remove_stale(address);
    let listener = create(address)?;
    Ok(Claim::Primary(listener))
}

fn address() -> anyhow::Result<Address> {
    let name = format!("mitibar-{:016x}.sock", user_hash());
    if GenericNamespaced::is_supported() {
        return Ok(Address::Namespaced(name));
    }
    let dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("no data-local directory for the socket"))?
        .join("mitibar");
    std::fs::create_dir_all(&dir)?;
    Ok(Address::File(dir.join(name)))
}

fn user_hash() -> u64 {
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    user.hash(&mut hasher);
    hasher.finish()
}

fn create(address: &Address) -> anyhow::Result<Listener> {
    let options = match address {
        Address::Namespaced(name) => {
            let name = name.as_str().to_ns_name::<GenericNamespaced>()?;
            ListenerOptions::new().name(name)
        }
        Address::File(path) => {
            let name = path.as_path().to_fs_name::<GenericFilePath>()?;
            ListenerOptions::new().name(name)
        }
    };
    Ok(options.create_sync()?)
}

fn signal_at(address: &Address) -> anyhow::Result<()> {
    let mut stream = match address {
        Address::Namespaced(name) => {
            let name = name.as_str().to_ns_name::<GenericNamespaced>()?;
            Stream::connect(name)?
        }
        Address::File(path) => {
            let name = path.as_path().to_fs_name::<GenericFilePath>()?;
            Stream::connect(name)?
        }
    };
    stream.write_all(b"open\n")?;
    stream.flush()?;
    Ok(())
}

fn remove_stale(address: &Address) {
    if let Address::File(path) = address {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique test socket. Windows uses a named pipe; Unix uses a file in a tempdir, which the
    /// caller removes.
    #[cfg(windows)]
    fn test_address(tag: &str) -> (Address, Option<tempfile::TempDir>) {
        (
            Address::Namespaced(format!("mitibar-test-{tag}-{}", std::process::id())),
            None,
        )
    }

    #[cfg(not(windows))]
    fn test_address(tag: &str) -> (Address, Option<tempfile::TempDir>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(format!("mitibar-test-{tag}.sock"));
        (Address::File(path), Some(dir))
    }

    #[test]
    fn wr09_ac1_second_instance_signals_the_first() {
        let (address, _dir) = test_address("signal");
        let Claim::Primary(listener) = claim_at(&address).expect("primary") else {
            panic!("first claim must be primary");
        };
        signal_at(&address).expect("signal");
        let mut line = String::new();
        {
            let mut incoming = listener.incoming();
            let stream = incoming.next().expect("one connection").expect("stream");
            BufReader::new(stream).read_line(&mut line).expect("read");
        }
        assert_eq!(line, "open\n");
        drop(listener);
        // The tempdir (Unix socket file) and the named pipe (Windows) both go away here.
        if let Address::File(path) = &address {
            let _ = std::fs::remove_file(path);
            assert!(!path.exists(), "test socket cleaned up");
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn wr09_ac1_stale_socket_is_taken_over() {
        let (address, _dir) = test_address("stale");
        if let Address::File(path) = &address {
            std::fs::write(path, b"stale").expect("stale file");
        }
        let Claim::Primary(listener) = claim_at(&address).expect("take over") else {
            panic!("a stale file must not count as a running instance");
        };
        drop(listener);
        if let Address::File(path) = &address {
            let _ = std::fs::remove_file(path);
            assert!(!path.exists(), "test socket cleaned up");
        }
    }
}
