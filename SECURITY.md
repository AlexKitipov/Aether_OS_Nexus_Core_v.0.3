# Security Policy

## Supported Versions
The following table outlines which versions of AetherOS Nexus Core currently receive security updates and patches.  
Only actively maintained branches will be eligible for fixes related to vulnerabilities.

| Version | Supported |
|--------|-----------|
| 0.3.x  | ✅ Active support |
| 0.2.x  | ❌ No longer supported |
| < 0.2  | ❌ End of life |

## Reporting a Vulnerability
We take security seriously and appreciate responsible disclosure.

If you discover a vulnerability, please follow these steps:

1. **Do not open a public issue.**  
   Security reports must be handled privately to protect users.

2. **Contact the maintainers directly:**  
   Send an email to:  
   **aetheros-security@protonmail.com**  
   (or your preferred address)

3. Include the following information:
   - Detailed description of the vulnerability  
   - Steps to reproduce  
   - Potential impact  
   - Suggested fixes (if any)

4. You will receive:
   - An acknowledgment within **72 hours**  
   - A status update within **7 days**  
   - Notification when the issue is resolved or if more information is required

## Disclosure Policy
- Valid vulnerabilities will be patched as quickly as possible.  
- Public disclosure will occur **only after** a fix is released.  
- Credit will be given to the reporter unless anonymity is requested.

## Security Best Practices for Contributors
To maintain a secure codebase, contributors should:

- Avoid introducing unnecessary `unsafe` Rust blocks  
- Follow the project’s coding standards and CI checks  
- Ensure new modules include tests  
- Avoid committing secrets, tokens, or private keys  
- Use reproducible builds and verify dependencies

## Scope
This policy applies to:
- The AetherOS kernel  
- V‑Node applications  
- Shared libraries (`common/`)  
- Build scripts and tooling  
- Documentation that may affect security posture

---

