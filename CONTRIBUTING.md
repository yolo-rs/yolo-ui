# Contributing to YOLO (Linux Kernel & Google Open Source Standard)

Thank you for contributing to the **YOLO** open-source ecosystem!
YOLO upholds strict engineering standards drawn from the **Linux Kernel Subsystem Model** and **Google Open Source Guidelines**.

---

## 1. Code of Conduct

All contributors, maintainers, and automated agents must adhere to the [Code of Conduct](CODE_OF_CONDUCT.md) (Contributor Covenant v2.1). Respectful, constructive, and technically rigorous collaboration is mandatory.

---

## 2. Developer Certificate of Origin (DCO) & Commit Standards

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

### Commit Message Format (Conventional Commits)

Commit messages must follow the [Conventional Commits v1.0.0](https://www.conventionalcommits.org/) specification:

```text
<type>(<scope>): <short imperative description>

[optional body explaining technical motivation, design trade-offs]

[optional footer(s), e.g. Closes #123]
Signed-off-by: Full Name <author@example.com>
```

Allowed types:
- `feat`: A new feature or user-visible capability
- `fix`: A bug fix or regression remediation
- `docs`: Documentation only changes
- `style`: Formatting, whitespace (no code change)
- `refactor`: Code restructuring without functional behavior changes
- `perf`: Performance improvements
- `test`: Adding or correcting tests
- `build`: Build system or dependency updates
- `ci`: CI configuration and workflow scripts
- `chore`: Repository maintenance

---

## 3. Style Guidelines

### Rust
- Conforms to the [Google Rust Style Guide](https://google.github.io/styleguide/rustguide.html).
- Format and lint enforcement:
  ```bash
  cargo fmt --check
  cargo clippy --workspace --all-targets -- -D warnings
  ```

### TypeScript / JavaScript
- Conforms to the [Google TypeScript Style Guide](https://google.github.io/styleguide/tsguide.html).
- Enforces 100% native **Bun v1.4.4+** (`zero Node.js`, `zero Zig` runtime assumptions).
- Quality enforcement:
  ```bash
  bun check
  bun fmt --check
  bun lint
  bun test
  ```

### Hardcoded Paths Policy
- **Machine-specific absolute paths are strictly forbidden** (e.g., `C:/`, `C:\`, `/home/user`, user profiles).
- Dynamic relative path resolution must always be used (`resolve(import.meta.dir, ...)`, `process.cwd()`, or configurable environment variables `PORT`, `HOST`, `BASE_URL`).

---

## 4. Pull Request Review & Subsystem Maintainer Process

1. **Branching**: Branch off `main` using descriptive branch names (`feat/my-feature`, `fix/issue-description`).
2. **Atomic Commits**: Keep commits focused and logically separated.
3. **Continuous Integration**: All automated test suites (`bun test`, `cargo test`) must pass cleanly with zero regression.
4. **Subsystem Review**: PRs will be reviewed by the corresponding subsystem maintainer listed in [MAINTAINERS.md](MAINTAINERS.md).
