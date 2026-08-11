use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameRecord {
    pub timestamp: String,
    pub old_name: String,
    pub new_name: String,
}

pub fn history_path(folder: &Path) -> PathBuf {
    folder.join(".snapdex_history.json")
}

pub fn append_batch(folder: &Path, records: &[(String, String)]) -> io::Result<()> {
    let path = history_path(folder);
    let mut history = load(&path)?;
    let timestamp = Utc::now().to_rfc3339();
    history.extend(records.iter().map(|(old_name, new_name)| RenameRecord {
        timestamp: timestamp.clone(),
        old_name: old_name.clone(),
        new_name: new_name.clone(),
    }));
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    serde_json::to_writer_pretty(file, &history).map_err(io::Error::other)
}

pub fn latest_batch(folder: &Path) -> io::Result<Vec<RenameRecord>> {
    let path = history_path(folder);
    let history = load(&path)?;
    let Some(latest) = history.last().map(|record| record.timestamp.clone()) else {
        return Ok(Vec::new());
    };
    Ok(history
        .into_iter()
        .filter(|record| record.timestamp == latest)
        .collect())
}

pub fn undo_latest_batch(folder: &Path) -> io::Result<usize> {
    let path = history_path(folder);
    let mut history = load(&path)?;
    let Some(latest_timestamp) = history.last().map(|record| record.timestamp.clone()) else {
        return Ok(0);
    };
    let start = history
        .iter()
        .rposition(|record| record.timestamp != latest_timestamp)
        .map(|index| index + 1)
        .unwrap_or(0);
    let records = history[start..].to_vec();

    for record in &records {
        let current = folder.join(&record.new_name);
        let original = folder.join(&record.old_name);
        if !current.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Cannot undo {}; file is missing", current.display()),
            ));
        }
        if original.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("Cannot undo; target already exists: {}", original.display()),
            ));
        }
    }

    for record in &records {
        fs::rename(folder.join(&record.new_name), folder.join(&record.old_name))?;
    }
    history.truncate(start);
    save(&path, &history)?;
    Ok(records.len())
}

fn save(path: &Path, history: &[RenameRecord]) -> io::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    serde_json::to_writer_pretty(file, history).map_err(io::Error::other)
}

fn load(path: &Path) -> io::Result<Vec<RenameRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)?;
    serde_json::from_reader(file).map_err(io::Error::other)
}
