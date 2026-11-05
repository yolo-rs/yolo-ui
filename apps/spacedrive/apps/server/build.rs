use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-env-changed=SD_SKIP_WEB_BUILD");

	if env::var_os("SD_SKIP_WEB_BUILD").is_some() {
		println!("cargo:warning=SD_SKIP_WEB_BUILD set — using existing apps/web/dist");
		return;
	}

	let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
	let repo_root = manifest_dir
		.parent()
		.and_then(Path::parent)
		.expect("apps/server is two levels below the repo root")
		.to_path_buf();
	let web_dir = repo_root.join("apps/web");

	// If bun isn't available (e.g., Docker Rust build stage), the caller is
	// expected to have prebuilt apps/web/dist. RustEmbed needs the folder to
	// exist even for a check or clippy run, so a debug build without either
	// gets an empty one; a release build refuses to ship a server without a UI.
	if Command::new("bun").arg("--version").output().is_err() {
		use_prebuilt_dist(&web_dir, "bun not found on PATH");
		return;
	}

	// Without installed workspace dependencies the UI cannot be built. A debug
	// build (check, clippy, test) falls back to a prebuilt or empty dist like
	// the no-bun case; a release build refuses.
	let node_modules = repo_root.join("node_modules");
	println!("cargo:rerun-if-changed={}", node_modules.display());
	if !node_modules.exists() && env::var("PROFILE").as_deref() != Ok("release") {
		use_prebuilt_dist(&web_dir, "node_modules missing");
		return;
	}

	// Refuse to proceed if workspace dependencies aren't installed.
	if !node_modules.exists() {
		panic!(
			"node_modules missing at {} — run `bun install` (or `just setup`) before building sd-server, \
			 or set SD_SKIP_WEB_BUILD=1 to skip the embedded UI build.",
			repo_root.display()
		);
	}

	// Invalidate the build script when any UI source or relevant config changes.
	// Cargo will cache this build script's output otherwise, so Rust-only changes
	// won't pay the cost of rebuilding the web bundle.
	watch_dir(&web_dir.join("src"));
	watch_dir(&repo_root.join("packages/interface/src"));
	watch_dir(&repo_root.join("packages/ts-client/src"));
	for path in [
		web_dir.join("index.html"),
		web_dir.join("vite.config.ts"),
		web_dir.join("package.json"),
		web_dir.join("tsconfig.json"),
		repo_root.join("packages/interface/package.json"),
		repo_root.join("packages/ts-client/package.json"),
	] {
		if path.exists() {
			rerun(&path);
		}
	}

	let status = Command::new("bun")
		.args(["run", "build"])
		.current_dir(&web_dir)
		.status()
		.expect("failed to spawn `bun run build`");

	if !status.success() {
		panic!(
			"`bun run build` in {} failed with status {}",
			web_dir.display(),
			status
		);
	}
}

/// Builds against an existing `apps/web/dist` without running the UI build.
/// A debug build without one gets an empty dist so RustEmbed has a folder; a
/// release build refuses to ship a server without a UI.
fn use_prebuilt_dist(web_dir: &Path, why: &str) {
	let dist = web_dir.join("dist");
	if dist.exists() {
		println!("cargo:warning={why} — using existing apps/web/dist");
		return;
	}
	if env::var("PROFILE").as_deref() == Ok("release") {
		panic!(
			"{why} and {} does not exist; build the web UI first or set SD_SKIP_WEB_BUILD=1 with a prebuilt dist",
			dist.display()
		);
	}
	std::fs::create_dir_all(&dist).expect("create an empty apps/web/dist");
	println!(
		"cargo:warning={why} — created an empty apps/web/dist so the server compiles without the web UI"
	);
}

fn watch_dir(dir: &Path) {
	if !dir.exists() {
		return;
	}
	let entries = match std::fs::read_dir(dir) {
		Ok(e) => e,
		Err(_) => return,
	};
	for entry in entries.flatten() {
		let path = entry.path();
		let file_type = match entry.file_type() {
			Ok(ft) => ft,
			Err(_) => continue,
		};
		if file_type.is_dir() {
			if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
				if matches!(name, "node_modules" | "dist" | ".turbo" | "build" | ".next") {
					continue;
				}
			}
			watch_dir(&path);
		} else if file_type.is_file() {
			rerun(&path);
		}
	}
}

fn rerun(path: &Path) {
	println!("cargo:rerun-if-changed={}", path.display());
}
