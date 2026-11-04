# Yolo UI browser workbench

This app owns the browser workbench frontend and its Bun HTTP host. The browser
process, CDP connection, navigation/evaluation APIs, and screenshot capture stay
in `yolo-web`; this app proxies to that API through `YOLO_WEB_API_URL`.

The transferred renderer and theme data came from
`yolo-web/apps/human-browser/src/ui.tsx` and `theme-presets.json`. The original
pre-format SHA-256 values were `02927DBD440DC8B521A60AD884D75CD2C44507D39EE9C654D4BD16EEB3DD63E6`
for `ui.tsx` and `5C4EDDB613048E045585059680AEE2C5A6BFA46426EBAB86948517FC926BD9DC`
for `theme-presets.json`; the latter also retains its generator/source commit
metadata. A pre-transfer snapshot of all six app files, the tracked patch, and
root manifest/lock snapshots is kept in the local `migration-backups` directory.

Run both services in separate terminals:

```powershell
# yolo-web
$env:YOLO_CDP_PORT = '9222'
$env:YOLO_HTTP_PORT = '8080'
bun --cwd=apps/human-browser run start

# yolo-ui
$env:YOLO_WEB_API_URL = 'http://127.0.0.1:8080'
bun --filter @yolo-rs/browser-workbench dev
```

The UI listens on `127.0.0.1:8081` by default. Set `YOLO_UI_HTTP_PORT` to
change that port. The UI uses a same-origin proxy for the browser API and media
stream, so it does not require permissive CORS on the engine service.

## YOLO-Web API contract

| Request | Response | Purpose |
| --- | --- | --- |
| `GET /api/status` | JSON | Browser connection, version, feature flags, and viewer counts |
| `POST /api/navigate` | JSON `{ url }` → JSON | Navigate; includes the final URL and removed tracking parameters |
| `POST /api/eval` | JSON `{ expression }` → JSON | Evaluate an expression in the active browser target |
| `GET /stream` | `multipart/x-mixed-replace; boundary=yolo-frame` | Stream JPEG frames captured by the browser service |

The frontend maps its form routes to those service endpoints and forwards
their HTTP status. `GET /healthz` reports frontend process health and the
configured API origin; it does not imply that a browser is connected.
