use std::path::Path;
use std::process::Command;

pub const CONFIDENCE_THRESHOLD: f32 = 60.0;
const MIN_HIGH_CONFIDENCE_WORDS: usize = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct OcrWord {
    pub text: String,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct OcrResult {
    pub text: String,
    pub low_confidence: bool,
    pub dimensions: (u32, u32),
}

#[derive(Debug)]
pub enum OcrError {
    TesseractUnavailable(String),
    Failed(String),
}

impl std::fmt::Display for OcrError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TesseractUnavailable(message) | Self::Failed(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for OcrError {}

pub fn check_available() -> Result<(), OcrError> {
    if which_tesseract().is_none() {
        return Err(OcrError::TesseractUnavailable(
            "Tesseract was not found. Install it with `brew install tesseract` and ensure it is on PATH."
                .to_string(),
        ));
    }

    tesseract::Tesseract::new(None, Some("eng"))
        .map(|_| ())
        .map_err(|error| {
            OcrError::TesseractUnavailable(format!(
                "Tesseract's native library or English language data could not be initialized: {error}. Install with `brew install tesseract`."
            ))
        })
}

pub fn extract(path: &Path) -> Result<OcrResult, OcrError> {
    let dimensions = image_dimensions(path)?;
    let is_heic = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("heic"))
        .unwrap_or(false);

    if !is_heic {
        image::ImageReader::open(path)
            .map_err(|error| {
                OcrError::Failed(format!("could not read image {}: {error}", path.display()))
            })?
            .with_guessed_format()
            .map_err(|error| {
                OcrError::Failed(format!(
                    "could not identify image {}: {error}",
                    path.display()
                ))
            })?
            .decode()
            .map_err(|error| {
                OcrError::Failed(format!(
                    "could not decode image {}: {error}",
                    path.display()
                ))
            })?;
    }

    let path = path.to_str().ok_or_else(|| {
        OcrError::Failed(format!("Image path is not valid UTF-8: {}", path.display()))
    })?;

    let mut engine = tesseract::Tesseract::new(None, Some("eng"))
        .map_err(|error| OcrError::Failed(error.to_string()))?
        .set_image(path)
        .map_err(|error| OcrError::Failed(error.to_string()))?
        .recognize()
        .map_err(|error| OcrError::Failed(error.to_string()))?;
    let tsv = engine
        .get_tsv_text(0)
        .map_err(|error| OcrError::Failed(error.to_string()))?;
    let words = parse_tsv_words(&tsv);
    let text = words
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    Ok(OcrResult {
        low_confidence: is_low_confidence(&words),
        text,
        dimensions,
    })
}

pub fn is_low_confidence(words: &[OcrWord]) -> bool {
    if words.is_empty() {
        return true;
    }

    let high_confidence_count = words
        .iter()
        .filter(|word| word.confidence > CONFIDENCE_THRESHOLD)
        .count();
    let average_confidence =
        words.iter().map(|word| word.confidence).sum::<f32>() / words.len() as f32;

    high_confidence_count < MIN_HIGH_CONFIDENCE_WORDS || average_confidence < CONFIDENCE_THRESHOLD
}

fn parse_tsv_words(tsv: &str) -> Vec<OcrWord> {
    tsv.lines()
        .skip(1)
        .filter_map(|line| {
            let columns = line.split('\t').collect::<Vec<_>>();
            if columns.len() < 12 || columns[0] != "5" {
                return None;
            }
            let text = columns[11].trim();
            if text.is_empty() {
                return None;
            }
            let confidence = columns[10].parse::<f32>().ok()?;
            if confidence < 0.0 {
                return None;
            }
            Some(OcrWord {
                text: text.to_string(),
                confidence,
            })
        })
        .collect()
}

fn image_dimensions(path: &Path) -> Result<(u32, u32), OcrError> {
    match image::image_dimensions(path) {
        Ok(dimensions) => Ok(dimensions),
        Err(_image_error)
            if path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("heic"))
                .unwrap_or(false) =>
        {
            let output = Command::new("sips")
                .args(["-g", "pixelWidth", "-g", "pixelHeight"])
                .arg(path)
                .output()
                .map_err(|error| {
                    OcrError::Failed(format!(
                        "could not read HEIC dimensions for {} with sips: {error}",
                        path.display()
                    ))
                })?;
            if !output.status.success() {
                return Err(OcrError::Failed(format!(
                    "could not read HEIC dimensions for {}: {}",
                    path.display(),
                    String::from_utf8_lossy(&output.stderr).trim()
                )));
            }
            let output = String::from_utf8_lossy(&output.stdout);
            let width = parse_sips_dimension(&output, "pixelWidth")?;
            let height = parse_sips_dimension(&output, "pixelHeight")?;
            Ok((width, height))
        }
        Err(error) => Err(OcrError::Failed(format!(
            "could not read image dimensions for {}: {error}",
            path.display()
        ))),
    }
}

fn parse_sips_dimension(output: &str, key: &str) -> Result<u32, OcrError> {
    output
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            (name.trim() == key)
                .then(|| value.trim().parse::<u32>().ok())
                .flatten()
        })
        .ok_or_else(|| OcrError::Failed(format!("sips did not report {key}")))
}

fn which_tesseract() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join("tesseract"))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::{OcrWord, is_low_confidence};

    fn words(confidences: &[f32]) -> Vec<OcrWord> {
        confidences
            .iter()
            .enumerate()
            .map(|(index, confidence)| OcrWord {
                text: format!("word{index}"),
                confidence: *confidence,
            })
            .collect()
    }

    #[test]
    fn rejects_fewer_than_three_high_confidence_words() {
        assert!(is_low_confidence(&words(&[95.0, 90.0, 59.9, 10.0])));
    }

    #[test]
    fn accepts_three_high_confidence_words_with_good_average() {
        assert!(!is_low_confidence(&words(&[80.0, 75.0, 90.0])));
    }

    #[test]
    fn rejects_low_average_even_with_three_high_confidence_words() {
        assert!(is_low_confidence(&words(&[61.0, 61.0, 61.0, 10.0])));
    }

    #[test]
    fn confidence_at_threshold_is_not_above_threshold() {
        assert!(is_low_confidence(&words(&[60.0, 60.0, 60.0])));
    }
}
