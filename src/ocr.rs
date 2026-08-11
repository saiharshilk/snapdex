use std::path::Path;

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

pub fn extract_text(path: &Path) -> Result<String, OcrError> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("heic") {
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
    engine
        .get_text()
        .map_err(|error| OcrError::Failed(error.to_string()))
}

fn which_tesseract() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join("tesseract"))
        .find(|candidate| candidate.is_file())
}
