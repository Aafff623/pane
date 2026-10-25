//! Skin resource packs: verify, install, list, remove — and serve each
//! pack's stylesheet with every asset inlined as a data URL, so the
//! frontend can hot-apply a downloaded skin with zero extra privileges
//! (no asset-protocol scope, no CSP network additions; the CSS is
//! self-contained and the share-PNG pipeline needs no pack awareness).
//!
//! A pack is a zip with a `manifest.json` plus its files:
//! `{ id, name, version, min_app_version, entry, files: [{ path, sha256 }] }`.
//! Every archived file must be listed with its sha256, every listed file
//! must exist, and paths must be relative and traversal-free — anything
//! else rejects the whole pack (never a partial install). Packs live in
//! `%APPDATA%\Pane\skins\<id>\`; installing over an id replaces it.
//!
//! Distribution (index endpoint, download URLs) is ticket 04; this module
//! already accepts full-artifact bytes so the client only adds a fetch.

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_PACK_BYTES: usize = 32 * 1024 * 1024;
const MAX_FILE_BYTES: usize = 12 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 64 * 1024;

#[derive(Serialize, Deserialize, Clone)]
pub struct PackFileEntry {
    pub path: String,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SkinPackManifest {
    /// Namespace for the CSS scope (`[data-skin="<id>"]`) and install dir.
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub min_app_version: String,
    /// Stylesheet path inside the pack, e.g. "skin.css".
    pub entry: String,
    /// Every file the pack carries (the entry included), with hashes.
    pub files: Vec<PackFileEntry>,
}

/// An installed pack plus its stylesheet, ready to inject: `entry_css`
/// has every `url(...)` rewritten to a data URL.
#[derive(Serialize, Clone)]
pub struct InstalledPack {
    #[serde(flatten)]
    pub manifest: SkinPackManifest,
    /// Absolute path of the installed pack directory.
    pub dir: String,
    pub entry_css: String,
}

/// Is this a value `config.skinId` may persist? The two built-ins, or a
/// resource-pack id. A shape check (not a membership check): a pack id may
/// be persisted before the pack exists locally — the frontend's
/// `migrateSkinFromIndex` downloads it on first run — so unknown-but-shaped
/// ids must survive the config patch. Path characters and oversized values
/// stay rejected. (Regression: the patch once accepted only "signal" |
/// "ink", silently wiping every pack selection on save.)
pub fn valid_skin_id(id: &str) -> bool {
    id == "signal"
        || id == "ink"
        || ((1..=64).contains(&id.len())
            && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
}

pub fn skins_dir() -> std::path::PathBuf {
    #[cfg(test)]
    if let Some(dir) = tests::SKINS_DIR.with(|dir| dir.borrow().clone()) {
        return dir;
    }
    crate::providers::config_dir().join("skins")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let hex: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
    hex
}

/// Path safety inside a pack: relative, forward slashes only, no `..`,
/// conservative charset — a hostile archive must not write outside the
/// pack directory.
fn safe_pack_path(path: &str) -> bool {
    if path.is_empty() || path.len() > 160 || path.starts_with('/') || path.starts_with('\\') {
        return false;
    }
    if path.contains('\\') || path.contains(':') {
        return false;
    }
    path.split('/').all(|segment| {
        !segment.is_empty()
            && segment != "."
            && segment != ".."
            && segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b' '))
    })
}

pub(crate) fn validate_manifest(m: &SkinPackManifest) -> Result<(), String> {
    let id_ok = !m.id.is_empty()
        && m.id.len() <= 32
        && m.id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !m.id.starts_with('-');
    if !id_ok {
        return Err("manifest id must be 1-32 chars of [a-z0-9-]".into());
    }
    if m.name.trim().is_empty() || m.name.len() > 64 {
        return Err("manifest name must be 1-64 chars".into());
    }
    if m.version.trim().is_empty() || m.version.len() > 32 {
        return Err("manifest version must be 1-32 chars".into());
    }
    if m.min_app_version.len() > 32 {
        return Err("manifest min_app_version too long".into());
    }
    if !safe_pack_path(&m.entry) {
        return Err("manifest entry is not a safe pack path".into());
    }
    if m.files.is_empty() {
        return Err("manifest files list is empty".into());
    }
    if m.files.len() > 512 {
        return Err("manifest lists too many files".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for f in &m.files {
        if !safe_pack_path(&f.path) {
            return Err(format!("unsafe file path in manifest: {}", f.path));
        }
        if f.sha256.len() != 64 || !f.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("bad sha256 for {}", f.path));
        }
        if !seen.insert(f.path.clone()) {
            return Err(format!("duplicate file path in manifest: {}", f.path));
        }
    }
    if !seen.contains(&m.entry) {
        return Err("manifest entry is not listed in files".into());
    }
    Ok(())
}

/// Verifies the zip in memory: manifest → validate → hash-verify every
/// file against the manifest. Returns the manifest and the verified
/// members (no writes) — the install step just persists them.
pub(crate) fn verify_pack(bytes: &[u8]) -> Result<(SkinPackManifest, std::collections::BTreeMap<String, Vec<u8>>), String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("not a readable skin pack zip: {e}"))?;
    if archive.len() > 600 {
        return Err("skin pack holds too many entries".into());
    }
    let manifest_bytes = {
        let mut file = archive
            .by_name("manifest.json")
            .map_err(|_| "skin pack has no manifest.json")?;
        if file.size() as usize > MAX_MANIFEST_BYTES {
            return Err("manifest.json too large".into());
        }
        let mut buf = Vec::with_capacity(file.size() as usize);
        std::io::Read::read_to_end(&mut file, &mut buf)
            .map_err(|e| format!("read manifest.json: {e}"))?;
        buf
    };
    let manifest: SkinPackManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| format!("manifest.json parse: {e}"))?;
    validate_manifest(&manifest)?;

    let by_path: std::collections::HashMap<&str, &PackFileEntry> = manifest
        .files
        .iter()
        .map(|f| (f.path.as_str(), f))
        .collect();
    let mut extracted: std::collections::BTreeMap<String, Vec<u8>> = Default::default();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("zip entry {i}: {e}"))?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_string();
        if name == "manifest.json" {
            continue;
        }
        if !safe_pack_path(&name) {
            return Err(format!("unsafe path inside pack: {name}"));
        }
        let entry = by_path.get(name.as_str())
            .ok_or_else(|| format!("file not listed in manifest: {name}"))?;
        if file.size() as usize > MAX_FILE_BYTES {
            return Err(format!("file too large inside pack: {name}"));
        }
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut buf)
            .map_err(|e| format!("read {name}: {e}"))?;
        if buf.len() > MAX_FILE_BYTES {
            return Err(format!("file too large inside pack: {name}"));
        }
        if sha256_hex(&buf) != entry.sha256.to_ascii_lowercase() {
            return Err(format!("sha256 mismatch for {name}"));
        }
        extracted.insert(name, buf);
    }
    for f in &manifest.files {
        if !extracted.contains_key(&f.path) {
            return Err(format!("manifest file missing from pack: {}", f.path));
        }
    }
    // The loader requires this file; persist the exact bytes we validated.
    extracted.insert("manifest.json".into(), manifest_bytes);
    Ok((manifest, extracted))
}

/// Reads the zip in memory and installs it into `skins/<id>/` (replacing
/// any prior install of the same id). Returns the verified manifest.
fn install_from_bytes(bytes: &[u8]) -> Result<SkinPackManifest, String> {
    let (manifest, extracted) = verify_pack(bytes)?;

    let dir = skins_dir().join(&manifest.id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("replace old pack: {e}"))?;
    }
    for (path, bytes) in &extracted {
        let target = dir.join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create pack dir: {e}"))?;
        }
        std::fs::write(&target, bytes).map_err(|e| format!("write {path}: {e}"))?;
    }
    Ok(manifest)
}

/// Rewrites every `url(...)` in the stylesheet to a data URL backed by the
/// installed file, so the injected CSS is self-contained (works inside the
/// share-PNG foreignObject, which cannot fetch external resources).
fn inline_css(dir: &std::path::Path, css: &str) -> String {
    let base64 = base64::engine::general_purpose::STANDARD;
    let rewrite = |url: &str| -> Option<String> {
        let url = url.trim().trim_matches(['"', '\'']);
        if url.is_empty()
            || url.starts_with("data:")
            || url.starts_with("http://")
            || url.starts_with("https:")
            || url.contains("..")
        {
            return None;
        }
        if !safe_pack_path(url) {
            return None;
        }
        let bytes = std::fs::read(dir.join(url.replace('/', std::path::MAIN_SEPARATOR_STR))).ok()?;
        let mime = match url.rsplit('.').next()?.to_ascii_lowercase().as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "woff2" => "font/woff2",
            "woff" => "font/woff",
            "ttf" => "font/ttf",
            "otf" => "font/otf",
            _ => "application/octet-stream",
        };
        Some(format!("data:{mime};base64,{}", base64.encode(bytes)))
    };
    let mut out = String::with_capacity(css.len() + 4096);
    let mut rest = css;
    while let Some(pos) = rest.find("url(") {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos + 4..];
        match tail.find(')') {
            Some(end) => {
                let url = &tail[..end];
                match rewrite(url) {
                    Some(data) => {
                        out.push_str("url(\"");
                        out.push_str(&data);
                        out.push_str("\")");
                    }
                    None => out.push_str(&rest[pos..pos + 4 + end + 1]),
                }
                rest = &tail[end + 1..];
            }
            None => {
                out.push_str(&rest[pos..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

fn read_installed(id: &str) -> Result<InstalledPack, String> {
    if !safe_pack_path(id) || id.contains('/') {
        return Err("invalid pack id".into());
    }
    let dir = skins_dir().join(id);
    let manifest_path = dir.join("manifest.json");
    let raw = std::fs::read(&manifest_path).map_err(|e| format!("read manifest: {e}"))?;
    let manifest: SkinPackManifest = serde_json::from_slice(&raw)
        .map_err(|e| format!("manifest parse: {e}"))?;
    validate_manifest(&manifest)?;
    let entry_path = dir.join(manifest.entry.replace('/', std::path::MAIN_SEPARATOR_STR));
    let css = std::fs::read_to_string(&entry_path)
        .map_err(|e| format!("read entry css: {e}"))?;
    Ok(InstalledPack {
        manifest: manifest.clone(),
        dir: dir.to_string_lossy().into_owned(),
        entry_css: inline_css(&dir, &css),
    })
}

/// Every installed pack, entry CSS inlined and ready to inject. A pack
/// whose manifest no longer parses (or whose entry CSS is missing) is
/// skipped — the workshop simply does not offer it. Files are NOT
/// re-hashed here: the sha256 gate ran once at install time, and
/// re-verifying every pack on every list would tax each refresh.
pub fn list_packs() -> Vec<InstalledPack> {
    let mut packs = Vec::new();
    let Ok(entries) = std::fs::read_dir(skins_dir()) else {
        return packs;
    };
    for e in entries.flatten() {
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            if let Ok(pack) = read_installed(&e.file_name().to_string_lossy()) {
                packs.push(pack);
            }
        }
    }
    packs.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    packs
}

/// Installs a pack from full artifact bytes (the download command fetches
/// them; tests and offline flows pass them directly).
pub fn install_bytes(bytes: &[u8]) -> Result<SkinPackManifest, String> {
    if bytes.len() > MAX_PACK_BYTES {
        return Err(format!("skin pack exceeds {} bytes", MAX_PACK_BYTES));
    }
    install_from_bytes(bytes)
}

pub fn remove_pack(id: &str) -> Result<(), String> {
    if !safe_pack_path(id) || id.contains('/') {
        return Err("invalid pack id".into());
    }
    let dir = skins_dir().join(id);
    if !dir.exists() {
        return Err("pack is not installed".into());
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("remove pack: {e}"))
}

pub async fn skin_pack_download(url: String, sha256: String) -> Result<SkinPackManifest, String> {
    let resp = super::providers::send_with_direct_fallback(|client| client.get(&url)).await?;
    if !resp.status().is_success() {
        return Err(format!("skin pack download: HTTP {}", resp.status()));
    }
    if let Some(len) = resp.content_length() {
        if len as usize > MAX_PACK_BYTES {
            return Err("skin pack download: response too large".into());
        }
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("skin pack download: {e}"))?;
    if bytes.len() > MAX_PACK_BYTES {
        return Err("skin pack download: response too large".into());
    }
    if !sha256.eq_ignore_ascii_case(&sha256_hex(bytes.as_ref())) {
        return Err("skin pack download: sha256 mismatch".into());
    }
    install_bytes(bytes.as_ref())
}

pub async fn skin_pack_install(zip_path: String) -> Result<SkinPackManifest, String> {
    let bytes = std::fs::read(&zip_path).map_err(|e| format!("read {zip_path}: {e}"))?;
    install_bytes(&bytes)
}

pub async fn skin_pack_list() -> Result<Vec<InstalledPack>, String> {
    Ok(list_packs())
}

pub async fn skin_pack_remove(id: String) -> Result<(), String> {
    remove_pack(&id)
}

#[cfg(test)]
mod tests {
    use super::*;

    std::thread_local! {
        pub(super) static SKINS_DIR: std::cell::RefCell<Option<std::path::PathBuf>> = const {
            std::cell::RefCell::new(None)
        };
    }

    struct TestSkinsDir(std::path::PathBuf);

    impl TestSkinsDir {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
            let dir = std::env::temp_dir()
                .join(format!("pane-skinpack-install-{}-{nonce}", std::process::id()));
            std::fs::create_dir(&dir).unwrap();
            SKINS_DIR.with(|root| *root.borrow_mut() = Some(dir.clone()));
            Self(dir)
        }
    }

    impl Drop for TestSkinsDir {
        fn drop(&mut self) {
            SKINS_DIR.with(|root| *root.borrow_mut() = None);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn installed_pack_is_listed_and_loads_its_stylesheet() {
        use std::io::Write;

        let _root = TestSkinsDir::new();
        let css = b"[data-skin=roundtrip]{color:tomato}";
        let raw_manifest = format!(
            "{{\n  \"id\":\"roundtrip\",\"name\":\"Round Trip\",\"version\":\"1.2.3\",\"min_app_version\":\"0.7.0\",\"entry\":\"skin.css\",\"files\":[{{\"path\":\"skin.css\",\"sha256\":\"{}\"}}],\"extra\":true\n}}",
            sha256_hex(css)
        );
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(raw_manifest.as_bytes()).unwrap();
        zip.start_file("skin.css", options).unwrap();
        zip.write_all(css).unwrap();
        let bytes = zip.finish().unwrap().into_inner();

        let installed = install_bytes(&bytes).expect("valid pack installs");
        assert_eq!(installed.id, "roundtrip");
        let packs = list_packs();
        assert_eq!(packs.len(), 1, "successful installation must appear in the workshop");
        let loaded = read_installed("roundtrip").expect("installed pack loads from disk");
        for pack in [&packs[0], &loaded] {
            assert_eq!(pack.manifest.id, "roundtrip");
            assert_eq!(pack.manifest.name, "Round Trip");
            assert_eq!(pack.manifest.version, "1.2.3");
            assert_eq!(pack.manifest.min_app_version, "0.7.0");
            assert_eq!(pack.manifest.entry, "skin.css");
            assert_eq!(pack.manifest.files.len(), 1);
            assert_eq!(pack.entry_css.as_bytes(), css);
        }
        assert_eq!(
            std::fs::read(skins_dir().join("roundtrip/manifest.json")).unwrap(),
            raw_manifest.as_bytes(),
            "persist the exact verified manifest, including formatting and unknown fields"
        );
    }

    fn manifest() -> SkinPackManifest {
        SkinPackManifest {
            id: "ink".into(),
            name: "Ink Wash".into(),
            version: "1.0.0".into(),
            min_app_version: "0.7.0".into(),
            entry: "skin.css".into(),
            files: vec![
                PackFileEntry { path: "skin.css".into(), sha256: "a".repeat(64) },
                PackFileEntry { path: "img/koi.png".into(), sha256: "b".repeat(64) },
            ],
        }
    }

    #[test]
    fn manifest_validation_accepts_a_well_formed_pack() {
        assert!(validate_manifest(&manifest()).is_ok());
    }

    #[test]
    fn manifest_validation_rejects_bad_ids_entries_and_dupes() {
        let mut m = manifest();
        m.id = "Bad_Id".into();
        assert!(validate_manifest(&m).is_err(), "uppercase/underscore id");
        let mut m = manifest();
        m.id = "-lead".into();
        assert!(validate_manifest(&m).is_err(), "leading dash id");
        let mut m = manifest();
        m.entry = "other.css".into();
        assert!(validate_manifest(&m).is_err(), "entry not listed");
        let mut m = manifest();
        m.files.push(m.files[0].clone());
        assert!(validate_manifest(&m).is_err(), "duplicate path");
        let mut m = manifest();
        m.files.clear();
        assert!(validate_manifest(&m).is_err(), "empty files");
        let mut m = manifest();
        m.files[0].sha256 = "zz".into();
        assert!(validate_manifest(&m).is_err(), "malformed hash");
    }

    #[test]
    fn pack_paths_reject_traversal_and_absolutes() {
        for bad in [
            "../evil.css", "a/../../b", "/abs.css", "C:/x.css", "a\\b.css",
            "", "a//b", "a/", ".", "..",
        ] {
            assert!(!safe_pack_path(bad), "{bad} must be rejected");
        }
        for good in ["skin.css", "img/koi.png", "fonts/MaShanZheng-ui.ttf", "a b/name.svg"] {
            assert!(safe_pack_path(good), "{good} must be accepted");
        }
    }

    #[test]
    fn css_inliner_embeds_local_files_and_leaves_remote_urls() {
        let dir = std::env::temp_dir().join(format!("pane-skinpack-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("koi.png"), b"\x89PNG").unwrap();
        let css = "a{background:url('koi.png')}b{background:url(https://x/y.png)}c{background:url(data:image/png;base64,AA)}";
        let out = inline_css(&dir, css);
        assert!(out.contains("data:image/png;base64,"), "{out}");
        assert!(out.contains("https://x/y.png"), "remote url untouched");
        assert!(out.contains("data:image/png;base64,AA"), "data url untouched");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_shipped_official_packs_pass_verification() {
        // The committed artifacts (site/public/skins) are the exact bytes
        // the workshop serves; running them through the real verifier is
        // the release gate for the pack format (entry-name separators,
        // hashes, manifest completeness).
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../site/public/skins");
        let mut checked = 0;
        for e in std::fs::read_dir(&dir).expect("site/public/skins must exist").flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.starts_with("pane-skin-") || !name.ends_with(".zip") {
                continue;
            }
            let bytes = std::fs::read(e.path()).unwrap();
            let (manifest, extracted) =
                verify_pack(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(extracted.contains_key(&manifest.entry));
            checked += 1;
        }
        assert!(checked >= 2, "expected the official packs, found {checked}");
    }

    #[test]
    fn official_packs_inline_assets_for_the_frontend() {
        // `skin_pack_list` serves self-contained CSS. A relative asset URL
        // would resolve against the webview origin instead of the pack and
        // silently lose the ink landscape/koi or signal atlas.
        let _root = TestSkinsDir::new();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../site/public/skins");
        for name in ["pane-skin-signal-1.0.1.zip", "pane-skin-ink-1.0.1.zip"] {
            let bytes = std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
            let manifest = install_bytes(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
            let pack = read_installed(&manifest.id).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(
                pack.entry_css.contains("data:image/webp;base64"),
                "{name}: image asset was not inlined"
            );
            if manifest.id == "ink" {
                assert!(
                    pack.entry_css.contains("data:font/ttf"),
                    "{name}: font asset was not inlined"
                );
            }
            assert!(
                !pack.entry_css.contains("url('assets/") && !pack.entry_css.contains("url(assets/"),
                "{name}: relative pack asset URL survived in the served CSS"
            );
            // Chromium silently drops declarations carrying multi-megabyte
            // data URLs once the served stylesheet grows large — the ink
            // landscape and koi vanished from the preview at 16 MB. Keep the
            // inlined sheet modest so oversized artwork fails here instead.
            assert!(
                pack.entry_css.len() < 4 * 1024 * 1024,
                "{name}: inlined CSS is {} bytes; recompress the artwork",
                pack.entry_css.len()
            );
        }
    }

    #[test]
    fn sha256_hex_matches_the_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn skin_id_shape_accepts_pack_ids_rejects_garbage() {
        // Regression (2026-10-12): the config patch once coerced every pack
        // id to null, so a downloaded skin silently reverted to native on
        // restart and the cross-machine migration id never survived a save.
        for good in ["signal", "ink", "ink-wash", "pane-skin-ink", "a", &"x".repeat(64)] {
            assert!(valid_skin_id(good), "{good:?} should persist");
        }
        for bad in [
            "",                                  // empty
            &"x".repeat(65),                     // oversized
            "../escape", "with space", "a/b", "a.b", // path / separator / dot
            "说明",                              // non-ascii label, not an id
        ] {
            assert!(!valid_skin_id(bad), "{bad:?} must not persist");
        }
    }
}
