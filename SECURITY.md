# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest  | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability in RustBill, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please email **evan@rantai.dev** with:

1. A description of the vulnerability
2. Steps to reproduce the issue
3. Potential impact assessment
4. Any suggested fixes (optional)

We will acknowledge receipt within 48 hours and aim to provide a fix or mitigation plan within 7 days for critical issues.

## Security Considerations

RustBill handles sensitive billing and license data. Key security measures include:

- **Ed25519 cryptographic signing** for license keys
- **Argon2/Bcrypt** for password hashing
- **HMAC-SHA256** for webhook signature verification
- **Rate limiting** via Tower Governor on API endpoints
- **API key authentication** for backend access
- **Input validation** via Zod (frontend) and validator crate (backend)

## Responsible Disclosure

We appreciate security researchers who follow responsible disclosure practices. We are committed to working with the security community to verify and address any potential vulnerabilities.
