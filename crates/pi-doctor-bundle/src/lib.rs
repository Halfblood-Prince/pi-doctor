mod error;

pub use error::BundleError;

use pi_doctor_core::Report;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BundleInput {
    pub report: Report,
    pub extra_files: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct BundleResult {
    pub bundle_dir: PathBuf,
    pub files: Vec<String>,
}

pub fn write_bundle(
    output_root: impl AsRef<Path>,
    bundle_name: &str,
    input: &BundleInput,
) -> Result<BundleResult, BundleError> {
    let bundle_dir = output_root.as_ref().join(bundle_name);
    fs::create_dir_all(&bundle_dir).map_err(|source| BundleError::CreateDir {
        path: bundle_dir.clone(),
        source,
    })?;

    let mut files = Vec::new();
    let mut contents = BTreeMap::new();
    contents.insert(
        "report.json".to_owned(),
        redact(&pi_doctor_report::json::render(&input.report)?),
    );
    contents.insert(
        "report.txt".to_owned(),
        redact(&pi_doctor_report::human::render(
            &input.report,
            pi_doctor_report::human::RenderOptions {
                verbosity: pi_doctor_report::human::Verbosity::Verbose,
                color: false,
            },
        )),
    );

    for (path, content) in &input.extra_files {
        contents.insert(path.clone(), redact(content));
    }

    let manifest = manifest_text(bundle_name, &contents);
    contents.insert("manifest.txt".to_owned(), manifest);

    for (relative, content) in contents {
        let path = bundle_dir.join(&relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| BundleError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        fs::write(&path, content).map_err(|source| BundleError::WriteFile {
            path: path.clone(),
            source,
        })?;
        files.push(relative);
    }

    files.sort();
    Ok(BundleResult { bundle_dir, files })
}

fn manifest_text(bundle_name: &str, contents: &BTreeMap<String, String>) -> String {
    let mut lines = vec![format!("bundle={bundle_name}")];
    for path in contents.keys() {
        lines.push(path.clone());
    }
    format!("{}\n", lines.join("\n"))
}

pub fn redact(input: &str) -> String {
    let mut output = input.to_owned();
    output = redact_windows_user_paths(&output);
    output = redact_unix_user_paths(&output);
    output = redact_labeled_values(&output, &["hostname", "host", "user", "username"]);
    output = redact_ipv4(&output);
    redact_mac_like(&output)
}

fn redact_windows_user_paths(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        output.push(ch);
        if output.ends_with("C:\\Users\\") || output.ends_with("c:\\Users\\") {
            while let Some(next) = chars.peek() {
                if *next == '\\' || *next == '\n' || *next == '\r' {
                    break;
                }
                chars.next();
            }
            output.push_str("<redacted-user>");
        }
    }

    output
}

fn redact_unix_user_paths(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        output.push(ch);
        if output.ends_with("/home/") || output.ends_with("/Users/") {
            while let Some(next) = chars.peek() {
                if *next == '/' || *next == '\n' || *next == '\r' {
                    break;
                }
                chars.next();
            }
            output.push_str("<redacted-user>");
        }
    }

    output
}

fn redact_labeled_values(input: &str, labels: &[&str]) -> String {
    let mut output = input.to_owned();
    for label in labels {
        for separator in [":", "="] {
            let needle = format!("{label}{separator}");
            let mut start = 0;
            while let Some(index) = output[start..].find(&needle) {
                let absolute = start + index + needle.len();
                let rest = &output[absolute..];
                let end = rest
                    .find(['\n', '\r'])
                    .map(|offset| absolute + offset)
                    .unwrap_or(output.len());
                output.replace_range(absolute..end, " <redacted>");
                start = absolute + " <redacted>".len();
            }
        }
    }
    output
}

fn redact_ipv4(input: &str) -> String {
    replace_matching_tokens(input, is_ipv4, "<redacted-ip>")
}

fn redact_mac_like(input: &str) -> String {
    replace_matching_tokens(input, is_mac_like, "<redacted-mac>")
}

fn is_ipv4(token: &str) -> bool {
    let parts = token.split('.').collect::<Vec<_>>();
    parts.len() == 4
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.parse::<u8>().is_ok())
}

fn is_mac_like(token: &str) -> bool {
    for separator in [':', '-'] {
        let parts = token.split(separator).collect::<Vec<_>>();
        if parts.len() == 6
            && parts
                .iter()
                .all(|part| part.len() == 2 && part.chars().all(|ch| ch.is_ascii_hexdigit()))
        {
            return true;
        }
    }
    false
}

fn replace_matching_tokens(input: &str, predicate: fn(&str) -> bool, replacement: &str) -> String {
    let mut output = String::new();
    let mut token = String::new();

    for ch in input.chars() {
        if ch.is_whitespace() {
            output.push_str(&replace_token_if_needed(&token, predicate, replacement));
            token.clear();
            output.push(ch);
        } else {
            token.push(ch);
        }
    }

    output.push_str(&replace_token_if_needed(&token, predicate, replacement));
    output
}

fn replace_token_if_needed(token: &str, predicate: fn(&str) -> bool, replacement: &str) -> String {
    if token.is_empty() {
        return String::new();
    }

    let trimmed = token.trim_matches(|ch: char| ",;()[]{}".contains(ch));
    if predicate(trimmed) {
        token.replace(trimmed, replacement)
    } else {
        token.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{BundleInput, redact, write_bundle};
    use pi_doctor_core::{OverallStatus, Report, ReportMetadata};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn redacts_user_host_ip_and_mac_patterns() {
        let input = "hostname: pi5\nuser=alice\nC:\\Users\\alice\\file\n/home/alice/project\n192.168.1.44 aa:bb:cc:dd:ee:ff";
        let redacted = redact(input);

        assert!(!redacted.contains("alice"));
        assert!(!redacted.contains("192.168.1.44"));
        assert!(!redacted.contains("aa:bb:cc:dd:ee:ff"));
        assert!(redacted.contains("<redacted-user>"));
        assert!(redacted.contains("<redacted-ip>"));
        assert!(redacted.contains("<redacted-mac>"));
    }

    #[test]
    fn writes_reproducible_bundle_layout() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("bundle-tests");
        let _ = std::fs::remove_dir_all(&root);

        let report = Report {
            metadata: ReportMetadata {
                command: "check".to_owned(),
            },
            schema_version: "1.0.0",
            overall_status: OverallStatus::Healthy,
            probe_health: Vec::new(),
            system: None,
            config: None,
            camera: None,
            python: None,
            groups: Vec::new(),
            findings: Vec::new(),
        };
        let mut extra = BTreeMap::new();
        extra.insert("raw/system.txt".to_owned(), "hostname: pi5".to_owned());

        let result = write_bundle(
            &root,
            "pi-doctor-bundle-test",
            &BundleInput {
                report,
                extra_files: extra,
            },
        )
        .expect("bundle should be written");

        assert_eq!(
            result.files,
            vec![
                "manifest.txt".to_owned(),
                "raw/system.txt".to_owned(),
                "report.json".to_owned(),
                "report.txt".to_owned()
            ]
        );
    }
}
