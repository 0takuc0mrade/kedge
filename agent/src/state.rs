use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Deserialize, Serialize)]
struct PersistedState {
    processed_claim_ids: HashSet<String>,
}

pub struct StateStore {
    path: PathBuf,
    state: PersistedState,
}

impl StateStore {
    pub fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let state = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).wrap_err("Invalid Kedge state file")?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => PersistedState::default(),
            Err(error) => return Err(error).wrap_err("Failed to read Kedge state file"),
        };
        Ok(Self { path, state })
    }

    pub fn contains(&self, claim_id: &[u8; 32]) -> bool {
        self.state
            .processed_claim_ids
            .contains(&hex::encode(claim_id))
    }

    pub fn mark_processed(&mut self, claim_id: &[u8; 32]) -> Result<()> {
        if !self.state.processed_claim_ids.insert(hex::encode(claim_id)) {
            return Ok(());
        }

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).wrap_err("Failed to create Kedge state directory")?;
        }
        let temporary_path = temporary_path(&self.path);
        fs::write(&temporary_path, serde_json::to_vec_pretty(&self.state)?)
            .wrap_err("Failed to write Kedge state")?;
        fs::rename(&temporary_path, &self.path).wrap_err("Failed to commit Kedge state")?;
        Ok(())
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(".tmp");
    PathBuf::from(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_processed_claims() {
        let path = std::env::temp_dir().join(format!("kedge-state-{}.json", std::process::id()));
        let claim_id = [0xAB; 32];

        let mut store = StateStore::load(&path).unwrap();
        assert!(!store.contains(&claim_id));
        store.mark_processed(&claim_id).unwrap();

        let reloaded = StateStore::load(&path).unwrap();
        assert!(reloaded.contains(&claim_id));
        let _ = fs::remove_file(path);
    }
}
