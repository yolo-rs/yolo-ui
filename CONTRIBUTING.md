# Contributing to YOLO (Linux kernel and Google open source standards)

Thank you for contributing to the YOLO open-source ecosystem. YOLO follows strict engineering standards drawn from the Linux kernel subsystem model and the Google Open Source Guidelines.

## 1. Code of Conduct

All contributors, maintainers, and automated agents must follow the [Code of Conduct](CODE_OF_CONDUCT.md) (Contributor Covenant v2.1). Collaboration must be respectful, constructive and technically rigorous.

## 2. Developer Certificate of Origin (DCO) and commit standards

Every commit submitted to YOLO repositories must include a `Signed-off-by` line indicating agreement with the Developer Certificate of Origin (DCO 1.1):

```text
Developer Certificate of Origin
Version 1.1

By making a contribution to this project, I certify that:
(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or
(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications; or
(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.
(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution is maintained
    indefinitely.
```

### Commit message format (Conventional Commits)

Commit messages must follow the [Conventional Commits v1.0.0](https://www.conventionalcommits.org/) specification:

```text
<type>(<scope>): <short imperative description>

[optional body explaining technical motivation, design trade-offs]

[optional footer(s), e.g. Closes #123]
Signed-off-by: Full Name <author@example.com>
```

Allowed types:
- `feat`: a new feature or user-visible capability
- `fix`: a bug fix or regression fix
- `docs`: documentation-only changes
- `style`: formatting and whitespace (no code change)
- `refactor`: code restructuring that does not change behavior
- `perf`: performance improvements
- `test`: adding or correcting tests
- `build`: build system or dependency updates
- `ci`: CI configuration and workflow scripts
- `chore`: repository maintenance

## 3. Style guidelines

### Rust

Rust code follows the [Google Rust Style Guide](https://google.github.io/styleguide/rustguide.html). Formatting and lints are enforced with:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

### TypeScript / JavaScript

TypeScript and JavaScript follow the [Google TypeScript Style Guide](https://google.github.io/styleguide/tsguide.html). Code runs on native Bun v1.4.4+ only (`zero Node.js`, `zero Zig` runtime assumptions). Quality checks:

```bash
bun check
bun fmt --check
bun lint
bun test
```

### Hardcoded paths policy

Machine-specific absolute paths (e.g., `C:/`, `C:\`, `/home/user`, user profiles) are strictly forbidden. Always resolve paths dynamically and relatively (`resolve(import.meta.dir, ...)`, `process.cwd()`, or the configurable environment variables `PORT`, `HOST`, `BASE_URL`).

## 4. Pull request review and subsystem maintainer process

1. Branch off `main` with a descriptive branch name (`feat/my-feature`, `fix/issue-description`).
2. Keep commits atomic: focused and logically separated.
3. All automated test suites in continuous integration (`bun test`, `cargo test`) must pass with no regressions.
4. The subsystem maintainer listed in [MAINTAINERS.md](MAINTAINERS.md) for the affected area reviews the PR.
