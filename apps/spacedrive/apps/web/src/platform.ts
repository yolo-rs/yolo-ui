import type { Platform } from "@sd/interface/platform";

/**
 * Web platform implementation for Spacedrive server
 *
 * This provides a minimal platform abstraction for the web client.
 * Unlike Tauri, web platform cannot access native file system or daemon state directly.
 */
export const platform: Platform = {
	platform: "web",

	openLink(url: string) {
		window.open(url, "_blank", "noopener,noreferrer");
	},

	// sd-server serves the UI, RPC, and sidecars from a single origin, so the
	// sidecar base URL is wherever this page was loaded from. Without this the
	// ServerContext never gets a serverUrl and thumbnails silently never load.
	async getDaemonStatus() {
		return {
			is_running: true,
			socket_path: "",
			server_url: window.location.origin,
			started_by_us: false,
		};
	},

	async getAppVersion() {
		const response = await fetch("/version");
		if (!response.ok) throw new Error(`/version returned ${response.status}`);
		const body = (await response.json()) as { version: string };
		return body.version;
	},

	// The current library is per-browser state; keep it next to the other preferences.
	async getCurrentLibraryId() {
		return localStorage.getItem("spacedrive-current-library");
	},

	async setCurrentLibraryId(libraryId: string) {
		localStorage.setItem("spacedrive-current-library", libraryId);
	},

	confirm(message: string, callback: (result: boolean) => void) {
		callback(window.confirm(message));
	},

	// Web-specific implementations (no native capabilities)
	// File pickers, daemon control, etc. are not available on web
};
