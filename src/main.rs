mod history;
mod naming;
mod ocr;
mod scanner;
mod tui;

use naming::RenamePlan;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = run() {
        eprintln!("snapdex: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let undo = args.next().as_deref() == Some(std::ffi::OsStr::new("--undo"));
    let folder = expand_home(&folder_from_args_or_prompt(args.next())?)?;

    if !folder.is_dir() {
        return Err(format!("not a folder: {}", folder.display()).into());
    }
    if undo {
        let count = history::undo_latest_batch(&folder)?;
        if count == 0 {
            println!("No rename batch is available to undo.");
        } else {
            println!("Undid {count} rename(s) in {}.", folder.display());
        }
        return Ok(());
    }

    ocr::check_available()?;
    let images = scanner::scan_folder(&folder)?;
    if images.is_empty() {
        println!("No unprocessed images found in {}.", folder.display());
        return Ok(());
    }

    let mut reserved = HashSet::new();
    let mut plans = Vec::with_capacity(images.len());
    for image in images {
        let text = ocr::extract_text(&image)?;
        plans.push(naming::build_plan(&folder, image, &text, &mut reserved)?);
    }

    if !tui::confirm(&plans)? {
        println!("Cancelled; no files were renamed.");
        return Ok(());
    }

    rename_plans(&plans)?;
    let records = plans
        .iter()
        .map(|plan| (file_name(&plan.old_path), file_name(&plan.new_path)))
        .collect::<Result<Vec<_>, _>>()?;
    history::append_batch(&folder, &records)?;
    println!("Renamed {} image(s).", plans.len());
    Ok(())
}

fn folder_from_args_or_prompt(argument: Option<std::ffi::OsString>) -> Result<PathBuf, io::Error> {
    if let Some(argument) = argument {
        return Ok(PathBuf::from(argument));
    }
    print!("Folder to scan: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    if input.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a folder path is required",
        ));
    }
    Ok(PathBuf::from(input))
}

fn expand_home(path: &Path) -> Result<PathBuf, io::Error> {
    let value = path.to_string_lossy();
    if value == "~" {
        return env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"));
    }
    if let Some(rest) = value.strip_prefix("~/") {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
        return Ok(home.join(rest));
    }
    Ok(path.to_path_buf())
}

fn rename_plans(plans: &[RenamePlan]) -> Result<(), io::Error> {
    let mut temporary = Vec::with_capacity(plans.len());

    for (index, plan) in plans.iter().enumerate() {
        let mut attempt = 0;
        let temp = loop {
            let candidate = plan.old_path.with_file_name(format!(
                ".snapdex-tmp-{index}-{}-{attempt}",
                std::process::id()
            ));
            if !candidate.exists() {
                break candidate;
            }
            attempt += 1;
        };

        if let Err(error) = fs::rename(&plan.old_path, &temp) {
            for (created_temp, old_path, _) in temporary.iter().rev() {
                let _ = fs::rename(created_temp, old_path);
            }
            return Err(error);
        }
        temporary.push((temp, plan.old_path.clone(), plan.new_path.clone()));
    }

    for index in 0..temporary.len() {
        let (temp, old_path, destination) = &temporary[index];
        if destination.exists() {
            rollback_renames(&temporary, index);
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("destination already exists: {}", destination.display()),
            ));
        }
        if let Err(error) = fs::rename(temp, destination) {
            rollback_renames(&temporary, index);
            return Err(io::Error::new(
                error.kind(),
                format!(
                    "could not rename {} to {}: {error}",
                    old_path.display(),
                    destination.display()
                ),
            ));
        }
    }
    Ok(())
}

fn rollback_renames(temporary: &[(PathBuf, PathBuf, PathBuf)], completed: usize) {
    for (index, (temp, old_path, destination)) in temporary.iter().enumerate().rev() {
        if index < completed {
            let _ = fs::rename(destination, old_path);
        } else if temp.exists() {
            let _ = fs::rename(temp, old_path);
        }
    }
}

fn file_name(path: &Path) -> Result<String, io::Error> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "filename is not valid UTF-8"))
}
