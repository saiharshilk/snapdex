use chrono::{DateTime, Local};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "in", "is", "it", "of",
    "on", "or", "that", "the", "this", "to", "was", "were", "with", "you",
];

#[derive(Debug, Clone)]
pub struct RenamePlan {
    pub old_path: PathBuf,
    pub new_path: PathBuf,
}

pub fn keywords(text: &str) -> Vec<String> {
    let mut counts = HashMap::<String, usize>::new();
    for word in text.split_whitespace() {
        let clean = word
            .chars()
            .filter(|character| character.is_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        if clean.len() >= 2 && !STOPWORDS.contains(&clean.as_str()) {
            *counts.entry(clean).or_default() += 1;
        }
    }

    let mut words = counts.into_iter().collect::<Vec<_>>();
    words.sort_by(|(left_word, left_count), (right_word, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| right_word.len().cmp(&left_word.len()))
            .then_with(|| left_word.cmp(right_word))
    });
    words.into_iter().take(3).map(|(word, _)| word).collect()
}

pub fn build_plan(
    folder: &Path,
    old_path: PathBuf,
    text: &str,
    reserved: &mut HashSet<String>,
) -> io::Result<RenamePlan> {
    let extension = old_path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("png")
        .to_string();
    let date = earlier_file_date(&old_path)?;
    let date = DateTime::<Local>::from(date).format("%Y-%m-%d").to_string();
    let mut words = keywords(text);
    if words.is_empty() {
        words.extend(["untitled".to_string(), "image".to_string()]);
    } else if words.len() == 1 {
        words.push("image".to_string());
    }
    let topic = &words[0];
    let suffix = words.iter().skip(1).take(2).cloned().collect::<Vec<_>>();
    let base = if suffix.is_empty() {
        format!("{date}_{topic}")
    } else {
        format!("{date}_{topic}_{}", suffix.join("-"))
    };

    let original_name = old_path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Image filename is not valid UTF-8",
            )
        })?;
    let mut index = 1;
    loop {
        let candidate_name = if index == 1 {
            format!("{base}.{extension}")
        } else {
            format!("{base}-{index}.{extension}")
        };
        let candidate = folder.join(&candidate_name);
        if candidate_name != original_name
            && (candidate.exists() || reserved.contains(&candidate_name))
        {
            index += 1;
            continue;
        }
        reserved.insert(candidate_name);
        return Ok(RenamePlan {
            old_path,
            new_path: candidate,
        });
    }
}

fn earlier_file_date(path: &Path) -> io::Result<SystemTime> {
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?;
    Ok(metadata
        .created()
        .map(|created| created.min(modified))
        .unwrap_or(modified))
}
