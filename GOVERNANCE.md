# YOLO project governance

The YOLO open-source project uses a consensus-driven, transparent governance model inspired by the Linux Foundation Technical Advisory Board and the Rust Leadership Council.

## 1. Principles

1. Vendor neutrality: the YOLO specifications, runtimes and tooling are independent of any single company or cloud provider.
2. Open standards alignment: the project follows upstream standards bodies (W3C, WHATWG, TC39, Khronos Group, IETF, Linux Foundation) strictly.
3. Bleeding-edge quality: the project targets the latest stable/canary releases and accepts no legacy dead code or unverified claims.
4. Modernization: native Bun v1.4.4+ and Rust 2024, with no unnecessary legacy runtime layers.

## 2. Roles and responsibilities

### Contributors

A contributor is anyone who submits code, documentation, issue reports, or architectural reviews. Contributors are expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md) and sign off on their commits (DCO).

### Subsystem maintainers

Subsystem maintainers are domain specialists responsible for technical direction, code review and quality enforcement in their designated subsystem, as listed in [MAINTAINERS.md](MAINTAINERS.md). They have write access and merge authority for their domains.

### Core Steering Committee

The Core Steering Committee is made up of subsystem leads and oversees:
- cross-repository architecture and releases
- security vulnerability triage and disclosure coordination
- dispute escalation and Code of Conduct enforcement
- the roadmap and representation to upstream standards bodies

## 3. Decision making and RFC process

Subsystem changes are proposed through GitHub pull requests and discussions, and consensus among subsystem reviewers is the preferred path. Major breaking changes, new top-level crates, protocol modifications (MCP/JSON-RPC) and significant standards additions require a written Request for Comments (RFC) issue before implementation. In the rare case that reviewers cannot reach consensus, a majority vote of the Core Steering Committee decides.

## 4. Releases and deprecations

- Releases follow Semantic Versioning (SemVer 2.0.0).
- Public APIs, MCP tool schemas, and command-line interfaces give notice of a deprecation at least one minor release cycle before breaking changes.
