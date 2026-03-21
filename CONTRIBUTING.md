
# Contributing to AetherOS Nexus Core

Thank you for your interest in contributing to **AetherOS Nexus Core**.  
This project aims to explore next‑generation operating system design based on security, modularity, and capability‑driven architecture.  
Contributions of all kinds are welcome — from code and documentation to design discussions and issue reports.

---

## 🧭 How to Contribute

### 1. Reporting Issues
If you discover a bug, inconsistency, or security concern:
- Open an issue in the repository.
- Provide a clear description of the problem.
- Include steps to reproduce the behavior when possible.
- Add logs or screenshots if relevant.

Please avoid sharing sensitive personal information in issues.

---

## 2. Suggesting Enhancements
If you have an idea for improving AetherOS:
- Open an issue labeled **enhancement**.
- Describe the motivation behind the idea.
- Explain how it fits into the AetherOS philosophy (security, modularity, clarity).

Conceptual discussions are welcome.

---

## 3. Submitting Pull Requests

Before submitting a PR:
- Ensure your code builds without errors.
- Follow Rust best practices and keep modules clean and well‑structured.
- Write clear commit messages.
- Reference related issues when applicable.

When submitting:
- Use a descriptive title.
- Explain what the PR changes and why.
- Keep PRs focused — avoid mixing unrelated changes.

---

## 4. Code Style Guidelines

- Use Rust’s standard formatting (`cargo fmt`).
- Prefer clarity over cleverness.
- Keep modules small and cohesive.
- Document public functions and modules.
- Avoid unsafe code unless absolutely necessary, and justify its use.

---

## 5. Security

If you find a security vulnerability:
- **Do not open a public issue.**
- Contact the project maintainers privately at the email listed in the Code of Conduct.
- Provide as much detail as possible so the issue can be reproduced and fixed.

---

## 6. Community Standards

All contributors must follow the project’s  
**[Code of Conduct](./CODE_OF_CONDUCT.md)**.  
Respectful communication and constructive collaboration are expected at all times.

---

## 7. Development Setup

Basic steps to build the project:
1. Install the required Rust nightly toolchain.
2. Clone the repository.
3. Run the build script:
./scripts/build_kernel_image.sh


4. Use QEMU or your preferred emulator to test the kernel.

More detailed documentation will be added as the project evolves.

---

## 8. Thank You

Your contributions — whether small or large — help shape the future of AetherOS.  
We appreciate your time, ideas, and effort.
