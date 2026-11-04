# YOLO Project Governance

The YOLO open-source project operates under a consensus-driven, transparent governance model inspired by the **Linux Foundation Technical Advisory Board** and the **Rust Leadership Council**.

---

## 1. Principles

1. **Vendor Neutrality**: The YOLO specifications, runtimes, and tooling are independent of any single corporate or cloud entity.
2. **Open Standards Alignment**: We conform strictly to upstream authoritative standards bodies (W3C, WHATWG, TC39, Khronos Group, IETF, Linux Foundation).
3. **Bleeding-Edge Quality**: We target the latest stable/canary releases and maintain zero tolerance for legacy dead-code or unverified claims.
4. **Radical Modernization**: Pure native Bun v1.4.4+ and Rust 2024 with zero unnecessary legacy runtime layers.

---

## 2. Roles and Responsibilities

### Contributors
Anyone who submits code, documentation, issue reports, or architectural reviews. Contributors are expected to uphold the [Code of Conduct](CODE_OF_CONDUCT.md) and sign off on their commits (DCO).

### Subsystem Maintainers
Domain specialists responsible for technical direction, code review, and quality enforcement within their designated subsystem (as specified in [MAINTAINERS.md](MAINTAINERS.md)). Subsystem maintainers have write access and merge authority for their domains.

### Core Steering Committee
The oversight committee consisting of subsystem leads responsible for:
- Global cross-repository architecture and releases
- Security vulnerability triage and disclosure coordination
- Dispute escalation and Code of Conduct enforcement
- Strategic roadmap and upstream standards representation

---

## 3. Decision Making & RFC Process

- **Consensus Seeking**: Subsystem changes are proposed through GitHub Pull Requests and discussions. Consensus among subsystem reviewers is the preferred path.
- **Architectural RFCs**: Major breaking changes, new top-level crates, protocol modifications (MCP/JSON-RPC), or significant standards additions require a written Request for Comments (RFC) issue before implementation.
- **Voting**: In the rare event that consensus cannot be reached, decisions are decided by a majority vote of the Core Steering Committee.

---

## 4. Releases and Deprecations

- Releases follow Semantic Versioning (SemVer 2.0.0).
- Public APIs, MCP tool schemas, and command-line interfaces provide at least one minor release cycle deprecation notice before breaking changes.
