# MAINTAINERS

This file lists the subsystem maintainers and review areas for the YOLO open-source ecosystem, structured in the style of the Linux Kernel `MAINTAINERS` file.

General queries: maintainers@yolo-rs.org  
Project Repository: https://github.com/yolo-rs/yolo  

--------------------------------------------------------------------------------

YOLO META-WORKSPACE & GOVERNANCE
M: YOLO Lead Maintainers <core@yolo-rs.org>
L: yolo-dev@yolo-rs.org
S: Supported
F: C:\src\yolo\
F: scripts/
F: tests/

YOLO UI & DESIGN SYSTEMS (`yolo-ui`)
M: YOLO UI Subsystem Team <ui@yolo-rs.org>
S: Supported
F: packages/yolo-ui*
F: crates/yolo-*
F: skills/yolo-ui-pro-max/

YOLO WEB & BROWSER INTELLIGENCE (`yolo-web`)
M: YOLO Web Engine Team <web@yolo-rs.org>
S: Supported
F: apps/human-browser/
F: packages/cdp-client/
F: packages/web-cli/
F: crates/obscura*
F: crates/brave*

YOLO REVERSE ENGINEERING & REPLICA (`yolo-re`)
M: YOLO RE Team <re@yolo-rs.org>
S: Supported
F: crates/yolo-re-core/
F: packages/yolo-rea/
F: packages/yolo-re-mcp/
F: replica/
F: skills/

YOLO UNIVERSAL PLUGINS & MARKETPLACE (`yolo-plugins`)
M: YOLO Plugins & Security Team <plugins@yolo-rs.org>
S: Supported
F: packages/crypto/
F: packages/marketplace-server/
F: packages/yolo-universal-mcp/
F: plugins/

YOLO STANDARDS & HARDWARE SCHEMAS (`yolo-standards`)
M: YOLO Standards & POSIX Team <standards@yolo-rs.org>
S: Supported
F: schemas/
F: packages/create-yolo-app/
F: packages/yolo-hardware-db/
F: packages/yolo-posix-bun/

YOLO UPSTREAM VENDOR MIRRORS (`yolo-vendor`)
M: YOLO Toolchain & Vendor Team <vendor@yolo-rs.org>
S: Supported
F: scripts/vendor-sync.ts
F: tests/vendor-bench.test.ts
