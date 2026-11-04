# Security Policy

The YOLO project and its maintainers take security and vulnerability disclosure seriously.
YOLO encompasses multi-engine UI frameworks, bleeding-edge web automation runtimes, reverse-engineering engines, distributed signed plugin systems, and foundational hardware/POSIX schemas.

## Supported Versions

Security fixes and patches are targeted strictly at active branches:

| Version / Branch | Supported          | Runtime Baseline |
| ---------------- | ------------------ | ---------------- |
| `main`           | :white_check_mark: | Bun v1.4.4+ / Rust 2024 / WGPU 30 |
| `< 1.0.0`        | :x:                | Deprecated       |

## Reporting a Vulnerability

**Please do not report security issues through public GitHub issues, pull requests, or community chat.**

If you discover a vulnerability or security flaw in any YOLO repository:
1. Open a private security advisory through the repository's GitHub **Security** tab -> **Report a vulnerability**.
2. Or email the core security team directly at **security@yolo-rs.org** with `[SECURITY VULNERABILITY]` in the subject line.

### What to Include in Your Report

To help us triage, reproduce, and resolve issues promptly, please provide:
- Repository and component affected (e.g. `yolo-ui`, `yolo-web`, `yolo-plugins`, `yolo-re`, `yolo-standards`, `yolo-vendor`, `yolo`)
- Commit hash or version tag
- Clear step-by-step reproduction instructions, proof-of-concept scripts, or minimal test cases
- Potential impact and classification (e.g., SSRF, Remote Code Execution, Privilege Escalation, Ed25519 signature bypass, Memory Safety)
- Any proposed remediation or patches

## Response & Coordinated Disclosure

- **Acknowledgment**: Within 48 hours of initial report receipt.
- **Triage & Validation**: Within 5 business days, including confirmation of severity and affected subsystem.
- **Fix & Advisory**: We coordinate patches in private forks and establish a mutually agreed public disclosure date.
- **Credit**: We will credit researchers in GitHub Security Advisories and release notes upon publication.
