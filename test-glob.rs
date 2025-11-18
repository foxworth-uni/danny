// Quick test for glob patterns
use std::path::Path;

fn main() {
    let root = Path::new("/Users/fox/src/nine-gen/danny/test-files/nextjs-app");

    let patterns = [
        "**/app/**/page.{ts,tsx,js,jsx}",
        "pages/**/*.{ts,tsx,js,jsx}",
    ];

    for pattern_str in &patterns {
        let absolute_pattern = root.join(pattern_str);
        let pattern_str = absolute_pattern.to_string_lossy().to_string();

        println!("\nTesting pattern: {}", pattern_str);

        match glob::glob(&pattern_str) {
            Ok(paths) => {
                let mut count = 0;
                for path_result in paths {
                    match path_result {
                        Ok(path) => {
                            if path.is_file() {
                                println!("  Found: {}", path.display());
                                count += 1;
                            }
                        }
                        Err(e) => println!("  Error: {}", e),
                    }
                }
                println!("  Total files found: {}", count);
            }
            Err(e) => println!("  Glob error: {}", e),
        }
    }
}
