This document describes how to report security vulnerabilities in Orgsidian and the project's response commitments.

## Reporting a Vulnerability

The preferred reporting channel is **GitHub Security Advisories**:

- Open a private advisory at <https://github.com/orgsidian/orgsidian/security/advisories/new>.

If GitHub is unavailable, the fallback channel is email: `security@orgsidian.example`. (The address is a placeholder until the project owns a domain; the GitHub Security Advisories form is the operational channel.)

Please include: affected version (commit SHA or release tag), reproduction steps, and impact assessment. Do not open a public issue for unpatched vulnerabilities.

## Security Patch SLA

Security patches ship **within 14 days** of credible disclosure.

## Supported Versions

The **latest minor of the latest major** receives security patches. Older minors are best-effort; users on older minors should upgrade to receive fixes.

## Disclosure Policy

- **90-day coordinated disclosure** is the default: details are kept private for up to 90 days from initial credible disclosure to allow patch development and rollout.
- **Immediate disclosure** applies to vulnerabilities that are being actively exploited in the wild.

> See also: [`docs/security/advisory-exceptions.md`](./docs/security/advisory-exceptions.md) — quarterly review of accepted advisories.
