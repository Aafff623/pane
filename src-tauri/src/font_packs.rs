//! Optional app-local fonts. Uses the skin pack ZIP/hash gate and generates
//! @font-face itself; downloaded packages never supply executable CSS.
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const MAX_BYTES: usize = 32 * 1024 * 1024;
// Serialize list/install/remove snapshots across app windows. The lock is
// acquired only after downloads finish, never held across network awaits.
static PACK_IO: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Clone, Serialize, Deserialize)]
pub struct FontFace {
    pub path: String,
    #[serde(default = "regular_weight")]
    pub weight: u16,
    #[serde(default = "normal_style")]
    pub style: String,
}
fn regular_weight() -> u16 {
    400
}
fn normal_style() -> String {
    "normal".into()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FontPackManifest {
    #[serde(flatten)]
    pub pack: crate::skin_packs::SkinPackManifest,
    /// App-local alias, deliberately distinct from installed system families.
    pub family: String,
    /// Relative path of the bundled redistribution license.
    pub license: String,
    pub faces: Vec<FontFace>,
}

#[derive(Serialize)]
pub struct InstalledFontPack {
    #[serde(flatten)]
    pub manifest: FontPackManifest,
    pub entry_css: String,
}

fn fonts_dir() -> PathBuf {
    crate::providers::config_dir().join("fonts")
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 32
        && !id.starts_with('-')
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        && ![
            "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
            "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
        ]
        .contains(&id)
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn font_format(path: &str, bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if path.ends_with(".woff2") && bytes.starts_with(b"wOF2") {
        Some(("woff2", "font/woff2"))
    } else if path.ends_with(".woff") && bytes.starts_with(b"wOFF") {
        Some(("woff", "font/woff"))
    } else if path.ends_with(".ttf")
        && (bytes.starts_with(&[0, 1, 0, 0]) || bytes.starts_with(b"true"))
    {
        Some(("truetype", "font/ttf"))
    } else if path.ends_with(".otf") && bytes.starts_with(b"OTTO") {
        Some(("opentype", "font/otf"))
    } else {
        None
    }
}

fn validate(m: &FontPackManifest, files: &BTreeMap<String, Vec<u8>>) -> Result<String, String> {
    if !valid_id(&m.pack.id) {
        return Err("invalid font pack id".into());
    }
    if !m.family.starts_with("Pane Pack ")
        || m.family.len() > 96
        || !m
            .family
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'-'))
    {
        return Err("font family must use the Pane Pack prefix and ASCII letters, digits, spaces or hyphens".into());
    }
    if m.faces.is_empty() || m.faces.len() > 16 || !m.faces.iter().any(|f| f.path == m.pack.entry) {
        return Err("font pack must list 1-16 faces including its entry".into());
    }
    let license = files
        .get(&m.license)
        .ok_or("font pack license file missing")?;
    if !m.pack.files.iter().any(|f| f.path == m.license)
        || license.is_empty()
        || std::str::from_utf8(license).is_err()
    {
        return Err("font pack license must be a listed nonempty UTF-8 file".into());
    }
    let mut css = String::new();
    let mut seen = std::collections::BTreeSet::new();
    for face in &m.faces {
        if !(1..=1000).contains(&face.weight)
            || !["normal", "italic", "oblique"].contains(&face.style.as_str())
            || !seen.insert((&face.path, face.weight, &face.style))
        {
            return Err("invalid or duplicate font face".into());
        }
        let bytes = files.get(&face.path).ok_or("font face missing")?;
        let (format, mime) =
            font_format(&face.path, bytes).ok_or("unsupported font file or signature")?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        css.push_str(&format!("@font-face{{font-family:\"{}\";font-style:{};font-weight:{};font-display:swap;src:url(\"data:{mime};base64,{encoded}\") format(\"{format}\");}}\n", m.family, face.style, face.weight));
    }
    Ok(css)
}

fn verify(bytes: &[u8]) -> Result<(FontPackManifest, BTreeMap<String, Vec<u8>>), String> {
    if bytes.len() > MAX_BYTES {
        return Err("font pack exceeds 32 MiB".into());
    }
    // Bound the total expansion before the shared verifier allocates buffers;
    // duplicate/case-colliding names are ambiguous on supported Windows hosts.
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut names = std::collections::BTreeSet::new();
    let mut expanded = 0u64;
    for i in 0..zip.len() {
        let f = zip.by_index(i).map_err(|e| e.to_string())?;
        for segment in f.name().trim_end_matches('/').split('/') {
            let stem = segment.split('.').next().unwrap_or("").to_ascii_lowercase();
            if segment.ends_with(['.', ' '])
                || [
                    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6",
                    "com7", "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7",
                    "lpt8", "lpt9",
                ]
                .contains(&stem.as_str())
            {
                return Err("font pack contains a Windows-ambiguous path".into());
            }
        }
        expanded = expanded
            .checked_add(f.size())
            .ok_or("expanded font pack too large")?;
        if expanded > MAX_BYTES as u64 {
            return Err("expanded font pack exceeds 32 MiB".into());
        }
        if !names.insert(f.name().to_ascii_lowercase()) {
            return Err("duplicate archive path".into());
        }
    }
    let (_, files) = crate::skin_packs::verify_pack(bytes)?;
    let m: FontPackManifest =
        serde_json::from_slice(files.get("manifest.json").ok_or("manifest missing")?)
            .map_err(|e| format!("font manifest: {e}"))?;
    validate(&m, &files)?;
    Ok((m, files))
}

/// Verify everything before touching disk. Stage beside the destination and
/// retain the prior install until the replacement rename succeeds.
fn install_into(root: &Path, bytes: &[u8]) -> Result<FontPackManifest, String> {
    let _guard = PACK_IO
        .lock()
        .map_err(|_| "font pack storage lock poisoned")?;
    let (m, files) = verify(bytes)?;
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let stage = root.join(format!(".stage-{}-{nonce}", std::process::id()));
    let backup = root.join(format!(".backup-{}-{nonce}", std::process::id()));
    let dest = root.join(&m.pack.id);
    let result = (|| {
        std::fs::create_dir(&stage).map_err(|e| e.to_string())?;
        for (path, bytes) in files {
            let target = stage.join(path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(target, bytes).map_err(|e| e.to_string())?;
        }
        let old = dest.exists();
        if old {
            std::fs::rename(&dest, &backup).map_err(|e| e.to_string())?;
        }
        if let Err(e) = std::fs::rename(&stage, &dest) {
            if old {
                std::fs::rename(&backup, &dest)
                    .map_err(|restore| format!("install failed: {e}; restore failed: {restore}"))?;
            }
            return Err(e.to_string());
        }
        if old {
            std::fs::remove_dir_all(&backup)
                .map_err(|e| format!("font installed; old backup cleanup failed: {e}"))?;
        }
        Ok(m)
    })();
    if stage.exists() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    result
}

fn read_installed(dir: &Path) -> Result<InstalledFontPack, String> {
    let raw = std::fs::read(dir.join("manifest.json")).map_err(|e| e.to_string())?;
    if raw.len() > 64 * 1024 {
        return Err("font manifest too large".into());
    }
    let m: FontPackManifest = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    if !valid_id(&m.pack.id) || dir.file_name().and_then(|p| p.to_str()) != Some(&m.pack.id) {
        return Err("font id mismatch".into());
    }
    crate::skin_packs::validate_manifest(&m.pack)?;
    // Verify local bytes too, so changed/deleted files cannot load silently.
    let mut files = BTreeMap::new();
    let mut total = 0;
    for f in &m.pack.files {
        let path = dir.join(&f.path);
        if std::fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("font symlinks are not supported".into());
        }
        if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 12 * 1024 * 1024 {
            return Err("font file too large".into());
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        total += bytes.len();
        if total > MAX_BYTES {
            return Err("expanded font pack exceeds 32 MiB".into());
        }
        if !hash(&bytes).eq_ignore_ascii_case(&f.sha256) {
            return Err(format!("font file SHA256 mismatch: {}", f.path));
        }
        files.insert(f.path.clone(), bytes);
    }
    let entry_css = validate(&m, &files)?;
    Ok(InstalledFontPack {
        manifest: m,
        entry_css,
    })
}

pub async fn font_pack_list() -> Result<Vec<InstalledFontPack>, String> {
    let _guard = PACK_IO
        .lock()
        .map_err(|_| "font pack storage lock poisoned")?;
    let mut packs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(fonts_dir()) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_dir())
                && valid_id(&entry.file_name().to_string_lossy())
            {
                if let Ok(pack) = read_installed(&entry.path()) {
                    packs.push(pack);
                }
            }
        }
    }
    packs.sort_by(|a, b| a.manifest.pack.id.cmp(&b.manifest.pack.id));
    Ok(packs)
}
pub async fn font_pack_install(zip_path: String) -> Result<FontPackManifest, String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    install_into(&fonts_dir(), &bytes)
}
pub async fn font_pack_download(url: String, sha256: String) -> Result<FontPackManifest, String> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| "invalid font URL")?;
    if parsed.scheme() != "https" || !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("font downloads require HTTPS without credentials".into());
    }
    if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("font download needs a SHA256 digest".into());
    }
    let mut response = crate::providers::send_with_direct_fallback(|c| c.get(&url)).await?;
    if !response.status().is_success() {
        return Err(format!("font download: HTTP {}", response.status()));
    }
    if response
        .content_length()
        .is_some_and(|len| len > MAX_BYTES as u64)
    {
        return Err("font download too large".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > MAX_BYTES {
            return Err("font download too large".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if !hash(&bytes).eq_ignore_ascii_case(&sha256) {
        return Err("font download SHA256 mismatch".into());
    }
    install_into(&fonts_dir(), &bytes)
}
pub async fn font_pack_remove(id: String) -> Result<(), String> {
    let _guard = PACK_IO
        .lock()
        .map_err(|_| "font pack storage lock poisoned")?;
    if !valid_id(&id) {
        return Err("invalid font pack id".into());
    }
    std::fs::remove_dir_all(fonts_dir().join(id)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn pack(tamper: bool) -> Vec<u8> {
        let font = b"\0\x01\0\0testfont";
        let m = serde_json::json!({"id":"test-font","name":"Test","version":"1.0.0","entry":"font.ttf","family":"Pane Pack Test","license":"OFL.txt","faces":[{"path":"font.ttf"}],"files":[{"path":"font.ttf","sha256":hash(font)},{"path":"OFL.txt","sha256":hash(b"Test license")}]});
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, data) in [
            ("manifest.json", serde_json::to_vec(&m).unwrap()),
            (
                "font.ttf",
                if tamper {
                    b"badfont".to_vec()
                } else {
                    font.to_vec()
                },
            ),
            ("OFL.txt", b"Test license".to_vec()),
        ] {
            z.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(&data).unwrap();
        }
        z.finish().unwrap().into_inner()
    }
    #[test]
    fn verifies_faces_and_injects_only_generated_css() {
        let (m, files) = verify(&pack(false)).unwrap();
        let css = validate(&m, &files).unwrap();
        assert!(css.contains("font-family:\"Pane Pack Test\""));
        assert!(css.contains("data:font/ttf;base64,"));
        let mut bad = m.clone();
        bad.family = "Pane Pack Test\"}body{display:none}".into();
        assert!(validate(&bad, &files).is_err());
        bad = m.clone();
        bad.license = "missing".into();
        assert!(validate(&bad, &files).is_err());
        bad = m.clone();
        bad.faces[0].path = "../escape.ttf".into();
        assert!(validate(&bad, &files).is_err());
        assert!(verify(&pack(true)).is_err());
        for id in ["../bad", "con", "a/b", "", "Bad"] {
            assert!(!valid_id(id));
        }
    }
    #[test]
    fn install_reload_and_reject_tampered_replacement_without_residue() {
        let root = std::env::temp_dir().join(format!(
            "pane-font-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        install_into(&root, &pack(false)).unwrap();
        assert_eq!(
            read_installed(&root.join("test-font"))
                .unwrap()
                .manifest
                .family,
            "Pane Pack Test"
        );
        assert!(install_into(&root, &pack(true)).is_err());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        assert!(read_installed(&root.join("test-font")).is_ok());
        install_into(&root, &pack(false)).unwrap();
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::write(root.join("test-font/font.ttf"), b"tampered").unwrap();
        assert!(read_installed(&root.join("test-font")).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    // No "official artifacts pass the gate" test: the official catalog ships
    // empty by owner decision (2026-10-14), so there are no real packs to
    // gate. The synthetic packs above cover verify/install/tamper end to end;
    // restore a real-artifact test alongside the first catalog entry.
    #[test]
    fn rejects_unlisted_traversal_and_duplicate_zip_members() {
        for name in [
            "../escape.ttf",
            "extra.ttf",
            "FONT.TTF",
            "font.ttf.",
            "con.ttf",
        ] {
            let mut input = zip::ZipArchive::new(std::io::Cursor::new(pack(false))).unwrap();
            let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            for i in 0..input.len() {
                let mut file = input.by_index(i).unwrap();
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut bytes).unwrap();
                out.start_file(file.name(), zip::write::SimpleFileOptions::default())
                    .unwrap();
                out.write_all(&bytes).unwrap();
            }
            out.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            out.write_all(b"unexpected").unwrap();
            assert!(
                verify(&out.finish().unwrap().into_inner()).is_err(),
                "{name}"
            );
        }
    }
}
