# Contributing to Metalogos

Thank you for your interest in contributing to Metalogos! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Commit Message Guidelines](#commit-message-guidelines)
- [Pull Request Process](#pull-request-process)
- [Reporting Bugs](#reporting-bugs)
- [Requesting Features](#requesting-features)
- [Security Issues](#security-issues)
- [Licensing](#licensing)

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Cargo](https://doc.rust-lang.org/cargo/) (comes with Rust)
- [Git](https://git-scm.com/)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/ShkodnikAI/Metalogos.git
cd Metalogos

# Build the project
cargo build --release

# Run tests
cargo test

# Run the compiler
./target/release/metalogos --help
```

## How to Contribute

### Types of Contributions

We welcome the following types of contributions:

- **Bug fixes** — Fix existing issues or bugs you discover
- **Feature implementations** — Add new language features or compiler optimizations
- **Documentation** — Improve README, inline docs, or write tutorials
- **Tests** — Add unit tests, integration tests, or benchmark tests
- **Performance improvements** — Optimize the compiler or runtime
- **Security enhancements** — Improve the security model or fix vulnerabilities
- **Translations** — Translate documentation to other languages

### Before You Start

1. Check [existing issues](https://github.com/ShkodnikAI/Metalogos/issues) to avoid duplicate work
2. For significant changes, open a [discussion](https://github.com/ShkodnikAI/Metalogos/discussions) first
3. Comment on an issue you'd like to work on so we can assign it to you

## Development Workflow

### 1. Fork and Clone

```bash
git clone https://github.com/YOUR_USERNAME/Metalogos.git
cd Metalogos
git remote add upstream https://github.com/ShkodnikAI/Metalogos.git
```

### 2. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-description
```

Branch naming conventions:
- `feature/` — New features
- `fix/` — Bug fixes
- `docs/` — Documentation changes
- `refactor/` — Code refactoring
- `test/` — Test additions or improvements
- `perf/` — Performance improvements
- `security/` — Security-related changes

### 3. Make Changes

- Write clean, well-documented code
- Follow our [Coding Standards](#coding-standards)
- Add or update tests as needed
- Update documentation if your changes affect the public API

### 4. Test Your Changes

```bash
# Run all tests
cargo test

# Run with all features
cargo test --all-features

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy -- -D warnings

# Run security audit
cargo audit

# Build documentation
cargo doc --no-deps
```

### 5. Commit

Follow our [Commit Message Guidelines](#commit-message-guidelines).

```bash
git add .
git commit -m "feat(parser): add support for generic type constraints"
```

### 6. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then open a Pull Request on GitHub.

## Coding Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting: `cargo fmt`
- Use `clippy` for linting: `cargo clippy`
- Maximum line length: 100 characters
- Use meaningful variable and function names

### Documentation

- All public APIs must have doc comments (`///`)
- Use examples in doc comments where helpful
- Keep README.md up to date

### Testing

- All new features must include tests
- Aim for >80% code coverage
- Write both unit tests and integration tests
- Use property-based testing where applicable

### Security

- Follow secure coding practices
- Never commit secrets, API keys, or passwords
- Report security issues privately (see [Security Issues](#security-issues))

## Commit Message Guidelines

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

- `feat` — New feature
- `fix` — Bug fix
- `docs` — Documentation only
- `style` — Formatting, missing semicolons, etc. (no code change)
- `refactor` — Code refactoring
- `perf` — Performance improvement
- `test` — Adding or correcting tests
- `chore` — Build process, dependencies, etc.
- `security` — Security fix

### Examples

```
feat(compiler): add type inference for polymorphic functions

Implements Hindley-Milner type inference algorithm for generic
function calls. This enables automatic type deduction without
explicit annotations in most cases.

Closes #123
```

```
fix(parser): resolve ambiguity in nested block expressions

Previously, nested blocks with implicit returns caused incorrect
AST generation. This fix ensures proper scoping of block-level
expressions.

Fixes #456
```

## Pull Request Process

1. **Open a PR** with a clear title and description
2. **Link related issues** using keywords (`Fixes #123`, `Closes #456`)
3. **Ensure CI passes** — All GitHub Actions checks must be green
4. **Request review** from maintainers
5. **Address feedback** — Make requested changes and push updates
6. **Squash commits** if requested (maintainers may do this on merge)
7. **Merge** — Only maintainers can merge PRs

### PR Checklist

- [ ] Code compiles without warnings
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Clippy is clean (`cargo clippy`)
- [ ] Documentation is updated
- [ ] Commit messages follow conventions
- [ ] PR description explains what and why

## Reporting Bugs

### Before Reporting

- Search [existing issues](https://github.com/ShkodnikAI/Metalogos/issues) to avoid duplicates
- Check if the bug is already fixed in the latest version
- Try to isolate the bug with a minimal example

### Bug Report Template

```markdown
**Description:**
Clear description of the bug.

**Steps to Reproduce:**
1. Step one
2. Step two
3. Step three

**Expected Behavior:**
What should happen.

**Actual Behavior:**
What actually happens.

**Environment:**
- OS: [e.g., Ubuntu 22.04]
- Rust version: [e.g., 1.75.0]
- Metalogos version: [e.g., 0.3.1]

**Code Example:**
```metalogos
// Minimal code that reproduces the issue
```

**Additional Context:**
Screenshots, error messages, stack traces, etc.
```

## Requesting Features

### Feature Request Template

```markdown
**Feature Description:**
Clear description of the proposed feature.

**Motivation:**
Why is this feature needed? What problem does it solve?

**Proposed Solution:**
How should this feature work?

**Alternatives Considered:**
Other approaches you've thought about.

**Additional Context:**
Any other relevant information.
```

## Security Issues

**DO NOT open a public issue for security vulnerabilities.**

Instead, email security concerns to:

📧 **security@metalogos.dev** (or create a private security advisory on GitHub)

We will:
- Acknowledge receipt within 48 hours
- Provide a timeline for the fix
- Coordinate disclosure after the fix is released

See [SECURITY.md](SECURITY.md) for our full security policy.

## Licensing

By contributing to Metalogos, you agree that your contributions will be licensed under the **MIT OR Apache-2.0** dual license.

You represent that:
- You have the right to license your contribution
- Your contribution does not violate any third-party rights
- You are not aware of any patents that your contribution may infringe

---

**Questions?** Open a [discussion](https://github.com/ShkodnikAI/Metalogos/discussions) or reach out to the maintainers.

Thank you for helping make Metalogos better! 🦀
