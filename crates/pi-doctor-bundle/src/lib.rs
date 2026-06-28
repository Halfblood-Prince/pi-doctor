mod error;

pub use error::BundleError;

use pi_doctor_core::Report;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct BundleInput {
    pub report: Report,
    pub extra_files: BTreeMap<String, String>,
    pub privacy_mode: BundlePrivacyMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundlePrivacyMode {
    Sanitized,
    Sensitive,
}

impl BundlePrivacyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sanitized => "sanitized",
            Self::Sensitive => "sensitive",
        }
    }

    fn redaction_enabled(self) -> bool {
        matches!(self, Self::Sanitized)
    }
}

#[derive(Debug, Clone)]
pub struct BundleResult {
    pub bundle_dir: PathBuf,
    pub files: Vec<String>,
    pub manifest: Vec<ManifestEntry>,
    pub privacy_mode: BundlePrivacyMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub path: String,
    pub sha256: String,
    pub bytes: usize,
}

pub fn write_bundle(
    output_root: impl AsRef<Path>,
    bundle_name: &str,
    input: &BundleInput,
) -> Result<BundleResult, BundleError> {
    let output_root = output_root.as_ref();
    fs::create_dir_all(output_root).map_err(|source| BundleError::CreateDir {
        path: output_root.to_path_buf(),
        source,
    })?;

    let final_dir = collision_safe_bundle_dir(output_root, bundle_name);
    let staging_dir = create_staging_dir(output_root, bundle_name)?;
    set_secure_dir_permissions(&staging_dir)?;

    let result = write_bundle_contents(&staging_dir, bundle_name, input).and_then(
        |(mut files, manifest)| {
            fs::rename(&staging_dir, &final_dir).map_err(|source| BundleError::Rename {
                from: staging_dir.clone(),
                to: final_dir.clone(),
                source,
            })?;
            set_secure_dir_permissions(&final_dir)?;
            files.sort();
            Ok(BundleResult {
                bundle_dir: final_dir,
                files,
                manifest,
                privacy_mode: input.privacy_mode,
            })
        },
    );

    if result.is_err() {
        let _ = fs::remove_dir_all(&staging_dir);
    }

    result
}

fn write_bundle_contents(
    bundle_dir: &Path,
    bundle_name: &str,
    input: &BundleInput,
) -> Result<(Vec<String>, Vec<ManifestEntry>), BundleError> {
    let mut contents = BTreeMap::new();
    contents.insert(
        "report.json".to_owned(),
        privacy_filter(
            &pi_doctor_report::json::render(&input.report)?,
            input.privacy_mode,
        ),
    );
    contents.insert(
        "report.txt".to_owned(),
        privacy_filter(
            &pi_doctor_report::human::render(
                &input.report,
                pi_doctor_report::human::RenderOptions {
                    verbosity: pi_doctor_report::human::Verbosity::Verbose,
                    color: false,
                },
            ),
            input.privacy_mode,
        ),
    );
    contents.insert("privacy.txt".to_owned(), privacy_notice(input.privacy_mode));

    for (path, content) in &input.extra_files {
        contents.insert(path.clone(), privacy_filter(content, input.privacy_mode));
    }

    let mut files = Vec::new();
    let mut manifest = Vec::new();
    for (relative, content) in &contents {
        validate_relative_path(relative)?;
        write_atomic_text(bundle_dir, relative, content)?;
        files.push(relative.clone());
        manifest.push(ManifestEntry {
            path: relative.clone(),
            sha256: sha256_hex(content.as_bytes()),
            bytes: content.len(),
        });
    }

    let manifest_text = manifest_text(bundle_name, input.privacy_mode, &manifest);
    write_atomic_text(bundle_dir, "manifest.txt", &manifest_text)?;
    files.push("manifest.txt".to_owned());

    Ok((files, manifest))
}

fn privacy_filter(input: &str, mode: BundlePrivacyMode) -> String {
    if mode.redaction_enabled() {
        redact(input)
    } else {
        input.to_owned()
    }
}

fn privacy_notice(mode: BundlePrivacyMode) -> String {
    match mode {
        BundlePrivacyMode::Sanitized => [
            "privacy_mode=sanitized",
            "redaction=enabled",
            "note=Bundle content is sanitized before it is written.",
            "note=Review every file before sharing it with another party.",
            "",
        ]
        .join("\n"),
        BundlePrivacyMode::Sensitive => [
            "privacy_mode=sensitive",
            "redaction=disabled",
            "note=This bundle may contain hostnames, paths, serials, URLs, tokens, and raw command output.",
            "note=Share only with trusted support contacts.",
            "",
        ]
        .join("\n"),
    }
}

fn manifest_text(
    bundle_name: &str,
    privacy_mode: BundlePrivacyMode,
    entries: &[ManifestEntry],
) -> String {
    let mut lines = vec![
        format!("bundle={bundle_name}"),
        format!("privacy_mode={}", privacy_mode.as_str()),
        "format=sha256  bytes  path".to_owned(),
    ];
    for entry in entries {
        lines.push(format!("{}  {}  {}", entry.sha256, entry.bytes, entry.path));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn write_atomic_text(bundle_dir: &Path, relative: &str, content: &str) -> Result<(), BundleError> {
    let relative_path = validate_relative_path(relative)?;
    let path = bundle_dir.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| BundleError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
        set_secure_dir_permissions(parent)?;
    }

    let tmp_path = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("pi-doctor")
    ));
    fs::write(&tmp_path, content).map_err(|source| BundleError::WriteFile {
        path: tmp_path.clone(),
        source,
    })?;
    set_secure_file_permissions(&tmp_path)?;
    fs::rename(&tmp_path, &path).map_err(|source| BundleError::Rename {
        from: tmp_path,
        to: path,
        source,
    })?;
    Ok(())
}

fn validate_relative_path(relative: &str) -> Result<&Path, BundleError> {
    let path = Path::new(relative);
    if relative.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        })
    {
        return Err(BundleError::UnsafePath {
            path: relative.to_owned(),
        });
    }
    Ok(path)
}

fn collision_safe_bundle_dir(output_root: &Path, bundle_name: &str) -> PathBuf {
    let candidate = output_root.join(bundle_name);
    if !candidate.exists() {
        return candidate;
    }

    for index in 1.. {
        let candidate = output_root.join(format!("{bundle_name}-{index}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("unbounded suffix search should return a path")
}

fn create_staging_dir(output_root: &Path, bundle_name: &str) -> Result<PathBuf, BundleError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    for index in 0..100 {
        let candidate = output_root.join(format!(
            ".{bundle_name}.{}.{}.{index}.tmp",
            std::process::id(),
            now
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(BundleError::CreateDir {
                    path: candidate,
                    source,
                });
            }
        }
    }

    Err(BundleError::CreateDir {
        path: output_root.join(format!(".{bundle_name}.tmp")),
        source: std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not create unique staging directory",
        ),
    })
}

#[cfg(unix)]
fn set_secure_dir_permissions(path: &Path) -> Result<(), BundleError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|source| {
        BundleError::CreateDir {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn set_secure_dir_permissions(_path: &Path) -> Result<(), BundleError> {
    Ok(())
}

#[cfg(unix)]
fn set_secure_file_permissions(path: &Path) -> Result<(), BundleError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|source| {
        BundleError::WriteFile {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn set_secure_file_permissions(_path: &Path) -> Result<(), BundleError> {
    Ok(())
}

pub fn redact(input: &str) -> String {
    let mut output = input.to_owned();
    output = redact_private_key_blocks(&output);
    output = redact_home_paths(&output);
    output = redact_labeled_values(&output);
    output = redact_urls(&output);
    output = redact_ipv4(&output);
    output = redact_ipv6(&output);
    output = redact_mac_like(&output);
    redact_secret_tokens(&output)
}

fn redact_private_key_blocks(input: &str) -> String {
    let mut output = Vec::new();
    let mut in_private_key = false;
    for line in input.lines() {
        let upper = line.to_ascii_uppercase();
        if upper.starts_with("-----BEGIN ") && upper.contains("PRIVATE KEY-----") {
            output.push("<redacted-private-key>");
            in_private_key = true;
            continue;
        }
        if in_private_key {
            if upper.starts_with("-----END ") && upper.contains("PRIVATE KEY-----") {
                in_private_key = false;
            }
            continue;
        }
        output.push(line);
    }

    let mut redacted = output.join("\n");
    if input.ends_with('\n') {
        redacted.push('\n');
    }
    redacted
}

fn redact_home_paths(input: &str) -> String {
    let mut output = String::new();
    let chars = input.char_indices().collect::<Vec<_>>();
    let mut index = 0;

    while index < chars.len() {
        let byte_index = chars[index].0;
        let rest = &input[byte_index..];
        if let Some(prefix_len) = home_path_prefix_len(rest) {
            output.push_str(&rest[..prefix_len]);
            let consumed = consume_path_component(&rest[prefix_len..]);
            output.push_str("<redacted-user>");
            index = char_index_after_byte(&chars, byte_index + prefix_len + consumed);
        } else {
            output.push(chars[index].1);
            index += 1;
        }
    }

    output
}

fn home_path_prefix_len(rest: &str) -> Option<usize> {
    if rest.starts_with("/home/") {
        Some("/home/".len())
    } else if rest.starts_with("/Users/") {
        Some("/Users/".len())
    } else if rest.len() >= 9
        && rest.as_bytes()[1] == b':'
        && rest.as_bytes()[2] == b'\\'
        && rest[3..].to_ascii_lowercase().starts_with("users\\")
        && rest.as_bytes()[0].is_ascii_alphabetic()
    {
        Some("C:\\Users\\".len())
    } else {
        None
    }
}

fn consume_path_component(input: &str) -> usize {
    input
        .char_indices()
        .find(|(_, ch)| matches!(ch, '/' | '\\' | '\n' | '\r' | '"' | '\''))
        .map(|(index, _)| index)
        .unwrap_or(input.len())
}

fn char_index_after_byte(chars: &[(usize, char)], byte: usize) -> usize {
    chars
        .iter()
        .position(|(index, _)| *index >= byte)
        .unwrap_or(chars.len())
}

fn redact_labeled_values(input: &str) -> String {
    const LABELS: &[&str] = &[
        "authorization",
        "bearer",
        "board_serial",
        "credential",
        "credentials",
        "device id",
        "device-id",
        "device_id",
        "hostname",
        "host",
        "machine id",
        "machine-id",
        "machine_id",
        "password",
        "passwd",
        "psk",
        "secret",
        "serial",
        "serial number",
        "serial-number",
        "serial_number",
        "ssid",
        "token",
        "url",
        "user",
        "username",
        "wifi ssid",
        "wifi_ssid",
        "wi-fi ssid",
    ];

    let mut output = String::new();
    for segment in input.split_inclusive('\n') {
        let (line, ending) = split_line_ending(segment);
        if let Some(redacted) = redact_labeled_line(line, LABELS) {
            output.push_str(&redacted);
        } else {
            output.push_str(line);
        }
        output.push_str(ending);
    }
    output
}

fn split_line_ending(segment: &str) -> (&str, &str) {
    if let Some(line) = segment.strip_suffix("\r\n") {
        (line, "\r\n")
    } else if let Some(line) = segment.strip_suffix('\n') {
        (line, "\n")
    } else if let Some(line) = segment.strip_suffix('\r') {
        (line, "\r")
    } else {
        (segment, "")
    }
}

fn redact_labeled_line(line: &str, labels: &[&str]) -> Option<String> {
    let trimmed = line.trim_start();
    let leading = &line[..line.len() - trimmed.len()];
    let lower = trimmed.to_ascii_lowercase();

    for label in labels {
        if !lower.starts_with(label) {
            continue;
        }
        let after_label = &trimmed[label.len()..];
        let after_spaces = after_label.trim_start();
        let removed_spaces = after_label.len() - after_spaces.len();
        if let Some(separator) = after_spaces.chars().next()
            && matches!(separator, ':' | '=')
        {
            let value_start = label.len() + removed_spaces + separator.len_utf8();
            return Some(format!(
                "{}{}{} <redacted>",
                leading,
                &trimmed[..value_start],
                if after_spaces[separator.len_utf8()..].starts_with(' ') {
                    ""
                } else {
                    " "
                }
            ));
        }
    }

    None
}

fn redact_urls(input: &str) -> String {
    replace_matching_tokens(input, is_url, "<redacted-url>")
}

fn redact_ipv4(input: &str) -> String {
    replace_matching_tokens(input, is_ipv4, "<redacted-ip>")
}

fn redact_ipv6(input: &str) -> String {
    replace_matching_tokens(input, is_ipv6, "<redacted-ipv6>")
}

fn redact_mac_like(input: &str) -> String {
    replace_matching_tokens(input, is_mac_like, "<redacted-mac>")
}

fn redact_secret_tokens(input: &str) -> String {
    replace_matching_tokens(input, is_secret_like_token, "<redacted-token>")
}

fn is_url(token: &str) -> bool {
    token.starts_with("http://") || token.starts_with("https://")
}

fn is_ipv4(token: &str) -> bool {
    let parts = token.split('.').collect::<Vec<_>>();
    parts.len() == 4
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.parse::<u8>().is_ok())
}

fn is_ipv6(token: &str) -> bool {
    let token = token
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split('%')
        .next()
        .unwrap_or(token);
    token.contains(':') && token.parse::<std::net::Ipv6Addr>().is_ok()
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

fn is_secret_like_token(token: &str) -> bool {
    if token.starts_with("ghp_")
        || token.starts_with("gho_")
        || token.starts_with("github_pat_")
        || token.starts_with("sk-")
        || token.starts_with("xoxb-")
        || token.starts_with("AKIA")
    {
        return true;
    }

    let jwt_parts = token.split('.').collect::<Vec<_>>();
    if jwt_parts.len() == 3
        && jwt_parts
            .iter()
            .all(|part| part.len() >= 8 && part.chars().all(is_base64_url_char))
    {
        return true;
    }

    token.len() >= 32
        && token.chars().any(|ch| ch.is_ascii_alphabetic())
        && token.chars().any(|ch| ch.is_ascii_digit())
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}

fn is_base64_url_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '=')
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

    let trimmed = token.trim_matches(|ch: char| {
        ".,;()[]{}<>\"'`".contains(ch) || matches!(ch, '\u{201c}' | '\u{201d}')
    });
    if predicate(trimmed) {
        token.replace(trimmed, replacement)
    } else {
        token.to_owned()
    }
}

fn sha256_hex(input: &[u8]) -> String {
    const H0: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h = H0;
    let bit_len = (input.len() as u64) * 8;
    let mut message = input.to_vec();
    message.push(0x80);
    while (message.len() % 64) != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks(64) {
        let mut w = [0_u32; 64];
        for (index, word) in w.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    h.iter()
        .map(|word| format!("{word:08x}"))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::{BundleInput, BundlePrivacyMode, redact, sha256_hex, write_bundle};
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
    fn redacts_realistic_secret_corpus() {
        let input = [
            "HOSTNAME=Pi-Lab-01",
            "Serial          : 10000000abcdef12",
            "SSID: Home Lab Wifi",
            "url=https://user:pass@example.test/path?token=abc",
            "Authorization: Bearer ghp_abcdefghijklmnopqrstuvwxyz123456",
            "token=sk-test1234567890abcdefghijklmnop",
            "machine-id: 87c4bc1848a84471997203ee530d2fda",
            "device_id=usb-FT232R-ABC123456",
            "fd00:1234:5678::cafe",
            "-----BEGIN OPENSSH PRIVATE KEY-----",
            "not a real key",
            "-----END OPENSSH PRIVATE KEY-----",
        ]
        .join("\n");
        let redacted = redact(&input);

        for leaked in [
            "Pi-Lab-01",
            "10000000abcdef12",
            "Home Lab Wifi",
            "https://user:pass@example.test",
            "ghp_abcdefghijklmnopqrstuvwxyz123456",
            "sk-test1234567890abcdefghijklmnop",
            "87c4bc1848a84471997203ee530d2fda",
            "usb-FT232R-ABC123456",
            "fd00:1234:5678::cafe",
            "not a real key",
        ] {
            assert!(!redacted.contains(leaked), "leaked {leaked}");
        }
        assert!(redacted.contains("<redacted-private-key>"));
        assert!(redacted.contains("<redacted-ipv6>"));
    }

    #[test]
    fn sensitive_mode_keeps_raw_content() {
        let root = test_root("sensitive");
        let _ = std::fs::remove_dir_all(&root);
        let mut extra = BTreeMap::new();
        extra.insert("raw/system.txt".to_owned(), "hostname: pi5".to_owned());

        let result = write_bundle(
            &root,
            "pi-doctor-bundle-test",
            &BundleInput {
                report: empty_report(),
                extra_files: extra,
                privacy_mode: BundlePrivacyMode::Sensitive,
            },
        )
        .expect("bundle should be written");

        let raw = std::fs::read_to_string(result.bundle_dir.join("raw/system.txt"))
            .expect("raw file should exist");
        assert!(raw.contains("hostname: pi5"));
        assert_eq!(result.privacy_mode, BundlePrivacyMode::Sensitive);
    }

    #[test]
    fn writes_reproducible_bundle_layout_with_manifest_hashes() {
        let root = test_root("layout");
        let _ = std::fs::remove_dir_all(&root);

        let mut extra = BTreeMap::new();
        extra.insert("raw/system.txt".to_owned(), "hostname: pi5".to_owned());

        let result = write_bundle(
            &root,
            "pi-doctor-bundle-test",
            &BundleInput {
                report: empty_report(),
                extra_files: extra,
                privacy_mode: BundlePrivacyMode::Sanitized,
            },
        )
        .expect("bundle should be written");

        assert_eq!(
            result.files,
            vec![
                "manifest.txt".to_owned(),
                "privacy.txt".to_owned(),
                "raw/system.txt".to_owned(),
                "report.json".to_owned(),
                "report.txt".to_owned()
            ]
        );
        let manifest = std::fs::read_to_string(result.bundle_dir.join("manifest.txt"))
            .expect("manifest should exist");
        assert!(manifest.contains("privacy_mode=sanitized"));
        assert!(manifest.contains("raw/system.txt"));
        assert!(manifest.contains("<redacted>") || manifest.contains("sha256"));
        assert_eq!(result.manifest.len(), 4);
    }

    #[test]
    fn avoids_bundle_name_collisions() {
        let root = test_root("collision");
        let _ = std::fs::remove_dir_all(&root);
        let input = BundleInput {
            report: empty_report(),
            extra_files: BTreeMap::new(),
            privacy_mode: BundlePrivacyMode::Sanitized,
        };

        let first = write_bundle(&root, "pi-doctor-bundle-test", &input)
            .expect("first bundle should be written");
        let second = write_bundle(&root, "pi-doctor-bundle-test", &input)
            .expect("second bundle should be written");

        assert_ne!(first.bundle_dir, second.bundle_dir);
        assert!(
            second
                .bundle_dir
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("-1"))
        );
    }

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    fn empty_report() -> Report {
        Report {
            metadata: ReportMetadata::new("check"),
            schema_version: "1.0.0",
            overall_status: OverallStatus::Healthy,
            probe_health: Vec::new(),
            system: None,
            config: None,
            camera: None,
            python: None,
            groups: Vec::new(),
            findings: Vec::new(),
        }
    }

    fn test_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("bundle-tests")
            .join(name)
    }
}
