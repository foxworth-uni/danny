mod detector;
mod package;
mod files;
mod security;

pub use detector::EntryPointDetector;
pub use package::detect_framework;
pub use files::find_nearest_package_json;
pub use security::{validate_entry_point, validate_files_for_analysis, MAX_FILES_IN_FILES_MODE};

