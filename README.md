# snapdex

`snapdex` is a macOS-only, fully local terminal app that OCRs screenshots and renames them in place into searchable filenames. It makes no network or API calls, and it never writes OCR text or image content to its history log.

## Install

Install the native OCR dependency with Homebrew:

```sh
brew install tesseract
```

You also need a Rust toolchain and a C/C++ build environment (Xcode Command Line Tools are sufficient for typical macOS setups). If Cargo cannot find the Homebrew libraries, install `pkg-config` and make sure Homebrew's `bin` and `lib` directories are on your shell's `PATH`/library search path.

Build snapdex from this repository:

```sh
cargo build --release
```

## Usage

Pass a folder directly:

```sh
cargo run -- ~/Desktop/screenshots
# or, after installing/copying the binary:
snapdex ~/Desktop/screenshots
```

With no folder argument, snapdex prompts for one:

```sh
cargo run
```

The app scans only that folder (not subfolders) for `.png`, `.jpg`, `.jpeg`, and `.heic` files. It skips names already matching the snapdex pattern, such as `2026-08-11_receipt-total.png`.

A preview screen shows every `old name → new name` before anything changes:

- `y` or `Enter` confirms
- `q` or `Esc` cancels
- `↑`/`↓` or `j`/`k` scrolls

Example generated names look like:

```text
2026-08-11_invoice-total-payment.png
```

The date uses the earlier of the file's creation and modification times. Renames stay in the same directory. Collisions receive `-2`, `-3`, and so on.

## Undo

Undo the most recent confirmed batch:

```sh
cargo run -- --undo ~/Desktop/screenshots
```

The history is stored at `<folder>/.snapdex_history.json` and contains only timestamps and old/new filenames. It is intended solely to support undo.

OCR and keyword extraction are deliberately simple in this first version: Tesseract runs locally with English data, and keywords are selected offline from frequent non-stopwords. Sorting screenshots into subfolders and background watching are out of scope.
