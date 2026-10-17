//! How agent commands find the running board. On start the board writes its
//! address and a random token to `<state dir>/board-server` (readable only
//! by the user on Unix). Agent API requests must carry that token. Web pages
//! can't read the file, so other sites can't call the agent API.

use std::collections::hash_map::RandomState;
use std::fs::{self, OpenOptions};
use std::hash::{BuildHasher, Hasher};
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const FILE: &str = "board-server";

pub struct ServerInfo {
    pub addr: SocketAddr,
    pub token: String,
}

impl ServerInfo {
    pub fn new(addr: SocketAddr) -> Self {
        ServerInfo {
            addr,
            token: new_token(),
        }
    }

    pub fn save(&self, state_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(state_dir)
            .map_err(|error| format!("cannot create {}: {error}", state_dir.display()))?;
        let path = path(state_dir);
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options
            .open(&path)
            .and_then(|mut file| write!(file, "addr={}\ntoken={}\n", self.addr, self.token))
            .map_err(|error| format!("cannot write {}: {error}", path.display()))
    }

    pub fn load(state_dir: &Path) -> Option<Self> {
        let text = fs::read_to_string(path(state_dir)).ok()?;
        let field = |name: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
                .map(str::trim)
        };
        Some(ServerInfo {
            addr: field("addr")?.parse().ok()?,
            token: field("token")?.to_string(),
        })
    }
}

fn path(state_dir: &Path) -> PathBuf {
    state_dir.join(FILE)
}

/// 128 bits from std's per-process random SipHash keys (seeded by the OS).
fn new_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos()) as u64;
    (0..2u64)
        .map(|i| {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u64(nanos ^ i);
            format!("{:016x}", hasher.finish())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempRepo;

    #[test]
    fn saves_and_loads_server_info() {
        let repo = TempRepo::new();
        let info = ServerInfo::new("127.0.0.1:4567".parse().unwrap());
        assert_eq!(info.token.len(), 32);
        assert_ne!(info.token, ServerInfo::new(info.addr).token);
        info.save(&repo.workspace.state_dir).unwrap();
        let loaded = ServerInfo::load(&repo.workspace.state_dir).unwrap();
        assert_eq!((loaded.addr, loaded.token), (info.addr, info.token));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(path(&repo.workspace.state_dir))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
}
