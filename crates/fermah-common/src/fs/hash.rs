use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use tokio::{fs, task::JoinSet};

use crate::{fs::error::Error, hash::Hasher};

const KB_128: u64 = 128 * 1000;

pub async fn hash_path<H: Hasher + Clone + Send + 'static>(path: &Path) -> Result<H::Hash, Error> {
    let meta = fs::metadata(&path).await?;
    let hasher = Arc::new(Mutex::new(H::new()));
    let mut filepaths = BTreeSet::<PathBuf>::new();
    let mut tasks: JoinSet<()> = JoinSet::new();

    if meta.is_dir() {
        let mut read_dir = fs::read_dir(&path).await?;

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            if !entry.path().is_dir() {
                filepaths.insert(entry.path());
            }
        }

        for path in filepaths {
            let h = Arc::clone(&hasher);
            tasks.spawn(async move {
                let meta = fs::metadata(&path).await.unwrap();
                if meta.len() > KB_128 {
                    h.lock().unwrap().update_mmap_rayon(&path).unwrap();
                } else {
                    h.lock().unwrap().update_mmap(&path).unwrap();
                }
            });
        }
    } else if meta.len() > KB_128 {
        hasher.lock().unwrap().update_mmap_rayon(path)?;
    } else {
        hasher.lock().unwrap().update_mmap(path)?;
    }

    while tasks.join_next().await.is_some() {}

    let hash = hasher.lock().unwrap().clone().finalize();

    Ok(hash)
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write};

    use const_hex::ToHexExt;

    use crate::{fs::hash::hash_path, hash::blake3::Blake3Hasher};

    #[tokio::test]
    async fn test_hash_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        let mut f = File::create(&file).unwrap();

        f.write_all(b"test").unwrap();

        let hash = hash_path::<Blake3Hasher>(dir.path()).await.unwrap();
        let hex = "4878ca0425c739fa427f7eda20fe845f6b2e46ba5fe2a14df5b1e32f50603215";

        assert_eq!(hash.encode_hex().as_str(), hex);
    }
}
