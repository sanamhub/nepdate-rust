//! Single instance (WR-09 AC1, W3-06). The first instance listens on a local socket. A second
//! instance connects, sends `open\n` and exits 0. The listener runs on a background thread and
//! wakes the UI thread with `slint::invoke_from_event_loop`, because a Slint component is not
//! `Send`. W5-02 replaced the 250 ms polling timer with this wake, so an idle app has no
//! four-times-a-second UI wake.
//!
//! The name is `mitibar-<username-hash>.sock` in the OS namespace when available (Windows,
//! Linux). macOS has no namespace, so the socket is a file in the data-local directory. A stale
//! file is removed and the listener retried.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;

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

/// Starts the listener thread. Each `open\n` calls `wake`, which the composition root turns into
/// `slint::invoke_from_event_loop(run_open_request)`. The callback must be `Send`; it captures no
/// Slint type, only a function pointer or a sendable channel.
pub fn start(listener: Listener, wake: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        for connection in listener.incoming() {
            let Ok(stream) = connection else {
                break;
            };
            // `open` plus a newline is 5 bytes; a longer message is not ours and is not buffered.
            let mut line = String::new();
            let mut reader = BufReader::new(stream.take(MESSAGE_LIMIT));
            if reader.read_line(&mut line).is_ok() && line.trim() == "open" {
                wake();
            }
        }
    });
}

/// The most bytes read from one connection.
const MESSAGE_LIMIT: u64 = 16;

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
    fnv1a(user.as_bytes())
}

/// 64-bit FNV-1a. The socket name must be the same in every build of the app, so an upgraded
/// binary still finds the running one; `DefaultHasher` makes no such promise across Rust
/// releases.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
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
    fn socket_name_hash_is_fixed_across_builds() {
        // Published FNV-1a 64 test vectors.
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn wr09_ac1_second_instance_signals_the_first() {
        let (address, _dir) = test_address("signal");
        let Claim::Primary(listener) = claim_at(&address).expect("primary") else {
            panic!("first claim must be primary");
        };
        let (wake, woken) = std::sync::mpsc::channel();
        start(listener, move || {
            let _ = wake.send(());
        });
        signal_at(&address).expect("signal");
        woken
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("the listener woke the UI callback");
        // The thread owns the listener for the rest of this process; the test process ends here.
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
