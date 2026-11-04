// SPDX-License-Identifier: Apache-2.0
//! Signed update feed client. Checks and stages; never installs.
//!
//! Feed format (one JSON document served over HTTPS):
//!
//! ```json
//! { "payload": "<base64 of the manifest JSON bytes>",
//!   "signature": "<base64 ed25519 signature over the decoded payload bytes>",
//!   "key_id": "optional key label" }
//! ```
//!
//! The manifest ([`UpdateManifest`]) lists one asset per platform key
//! (`linux-x86_64`, `windows-x86_64`, `macos-aarch64`, see
//! [`current_platform`]) with its URL and SHA-256. The client verifies the
//! signature with the app's pinned public key before parsing the manifest,
//! then downloads the asset into `<cache_dir>/updates/<version>/`, checks the
//! digest and renames the file into place. Installing (replacing the binary,
//! running an installer or swapping a bundle) is left to the app.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{AppInfo, Error, Result};

/// Largest feed document accepted.
pub const MAX_FEED_BYTES: u64 = 1024 * 1024;

/// Signed envelope served at the feed URL.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedFeed {
    pub payload: String,
    pub signature: String,
    #[serde(default)]
    pub key_id: Option<String>,
}

/// Release manifest carried by the feed payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateManifest {
    /// Must equal [`AppInfo::id`].
    pub app_id: String,
    pub version: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub pub_date: Option<String>,
    pub platforms: BTreeMap<String, PlatformAsset>,
}

/// Downloadable artifact for one platform.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformAsset {
    pub url: String,
    /// Lower-case hex SHA-256 of the artifact.
    pub sha256: String,
    #[serde(default)]
    pub size: Option<u64>,
}

/// A verified update newer than the running version.
#[derive(Clone, Debug)]
pub struct AvailableUpdate {
    pub manifest: UpdateManifest,
    pub version: semver::Version,
    pub asset: PlatformAsset,
}

/// A downloaded, digest-checked artifact waiting to be installed by the app.
#[derive(Clone, Debug)]
pub struct StagedUpdate {
    pub version: semver::Version,
    pub path: PathBuf,
    pub notes: Option<String>,
}

/// `<os>-<arch>` key of the running build (`linux-x86_64`, `windows-aarch64`).
pub fn current_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn check_url(url: &str) -> Result<()> {
    let local = ["http://127.0.0.1", "http://localhost", "http://[::1]"];
    if url.starts_with("https://") || local.iter().any(|p| url.starts_with(p)) {
        Ok(())
    } else {
        Err(Error::Update(format!("refusing non-HTTPS update URL {url}")))
    }
}

/// Parses a base64 (standard alphabet) ed25519 public key.
pub fn parse_public_key(b64: &str) -> Result<VerifyingKey> {
    let bytes = B64.decode(b64.trim()).map_err(|e| Error::Update(format!("public key: {e}")))?;
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| Error::Update("public key must be 32 bytes".into()))?;
    Ok(VerifyingKey::from_bytes(&bytes)?)
}

/// Verifies `feed` with `key` and returns the manifest.
pub fn verify_feed(feed: &SignedFeed, key: &VerifyingKey) -> Result<UpdateManifest> {
    let payload = B64.decode(feed.payload.trim()).map_err(|e| Error::Update(format!("payload: {e}")))?;
    let signature = B64.decode(feed.signature.trim()).map_err(|e| Error::Update(format!("signature: {e}")))?;
    let signature: [u8; 64] = signature.try_into().map_err(|_| Error::Update("signature must be 64 bytes".into()))?;
    key.verify_strict(&payload, &Signature::from_bytes(&signature))?;
    Ok(serde_json::from_slice(&payload)?)
}

/// Signs a manifest with a 32-byte ed25519 secret key (release tooling side).
pub fn sign_manifest(secret_key: &[u8; 32], manifest: &UpdateManifest, key_id: Option<&str>) -> Result<SignedFeed> {
    let payload = serde_json::to_vec(manifest)?;
    let signature = SigningKey::from_bytes(secret_key).sign(&payload);
    Ok(SignedFeed {
        payload: B64.encode(&payload),
        signature: B64.encode(signature.to_bytes()),
        key_id: key_id.map(str::to_owned),
    })
}

/// Update client for one app.
#[derive(Clone, Debug)]
pub struct Updater {
    app_id: String,
    feed_url: String,
    key: VerifyingKey,
    current: semver::Version,
    platform: String,
    staging_dir: PathBuf,
}

impl Updater {
    /// `public_key` is the base64 ed25519 key pinned in the app build.
    pub fn new(app: &AppInfo, feed_url: impl Into<String>, public_key: &str) -> Result<Self> {
        let feed_url = feed_url.into();
        check_url(&feed_url)?;
        let current = semver::Version::parse(&app.version).map_err(|e| Error::Update(format!("app version: {e}")))?;
        Ok(Self {
            app_id: app.id.clone(),
            feed_url,
            key: parse_public_key(public_key)?,
            current,
            platform: current_platform(),
            staging_dir: app.cache_dir()?.join("updates"),
        })
    }

    pub fn with_platform(mut self, platform: impl Into<String>) -> Self {
        self.platform = platform.into();
        self
    }

    pub fn with_staging_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.staging_dir = dir.into();
        self
    }

    pub fn staging_dir(&self) -> &Path {
        &self.staging_dir
    }

    /// Verifies feed bytes and selects the update for this platform, if newer.
    pub fn evaluate(&self, feed_bytes: &[u8]) -> Result<Option<AvailableUpdate>> {
        let feed: SignedFeed = serde_json::from_slice(feed_bytes)?;
        let manifest = verify_feed(&feed, &self.key)?;
        if manifest.app_id != self.app_id {
            return Err(Error::Update(format!("feed is for {}, not {}", manifest.app_id, self.app_id)));
        }
        let version = semver::Version::parse(&manifest.version).map_err(|e| Error::Update(format!("feed version: {e}")))?;
        if version <= self.current {
            return Ok(None);
        }
        let Some(asset) = manifest.platforms.get(&self.platform).cloned() else {
            return Ok(None);
        };
        check_url(&asset.url)?;
        if asset.sha256.len() != 64 || !asset.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::Update("asset sha256 must be 64 hex digits".into()));
        }
        Ok(Some(AvailableUpdate { manifest, version, asset }))
    }

    /// Fetches and verifies the feed.
    pub fn check(&self) -> Result<Option<AvailableUpdate>> {
        let mut response = ureq::get(&self.feed_url).call()?;
        let bytes = response.body_mut().with_config().limit(MAX_FEED_BYTES).read_to_vec()?;
        self.evaluate(&bytes)
    }

    /// Downloads the asset into the staging directory and checks its digest.
    /// `progress(downloaded, total)` is called as bytes arrive.
    pub fn download(&self, update: &AvailableUpdate, mut progress: impl FnMut(u64, Option<u64>)) -> Result<StagedUpdate> {
        let dir = self.staging_dir.join(update.version.to_string());
        std::fs::create_dir_all(&dir)?;
        let name = update
            .asset
            .url
            .rsplit('/')
            .next()
            .map(|s| s.split(['?', '#']).next().unwrap_or(s))
            .filter(|s| !s.is_empty() && !s.contains(['\\', ':']) && *s != "." && *s != "..")
            .unwrap_or("update.bin")
            .to_owned();
        let target = dir.join(&name);
        let partial = dir.join(format!("{name}.part"));

        let response = ureq::get(&update.asset.url).call()?;
        let total = update.asset.size.or_else(|| {
            response.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok())
        });
        let mut reader = response.into_body().into_reader();
        let mut file = std::fs::File::create(&partial)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024];
        let mut downloaded = 0u64;
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
            file.write_all(&buffer[..n])?;
            downloaded += n as u64;
            if let Some(size) = update.asset.size
                && downloaded > size
            {
                drop(file);
                let _ = std::fs::remove_file(&partial);
                return Err(Error::Update("download exceeds the declared size".into()));
            }
            progress(downloaded, total);
        }
        file.sync_all()?;
        drop(file);

        let digest = hex(&hasher.finalize());
        if !digest.eq_ignore_ascii_case(&update.asset.sha256) {
            let _ = std::fs::remove_file(&partial);
            return Err(Error::Update(format!("sha256 mismatch: expected {}, got {digest}", update.asset.sha256)));
        }
        std::fs::rename(&partial, &target)?;
        std::fs::write(dir.join("manifest.json"), serde_json::to_vec_pretty(&update.manifest)?)?;
        Ok(StagedUpdate { version: update.version.clone(), path: target, notes: update.manifest.notes.clone() })
    }

    /// Removes staged versions other than `keep`.
    pub fn clean_staging(&self, keep: Option<&semver::Version>) -> Result<()> {
        let Ok(entries) = std::fs::read_dir(&self.staging_dir) else { return Ok(()) };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if keep.is_some_and(|k| name.to_string_lossy() == k.to_string()) {
                continue;
            }
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                std::fs::remove_dir_all(entry.path())?;
            }
        }
        Ok(())
    }
}
