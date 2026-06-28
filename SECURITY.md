# Security Policy

## Supported Versions

`pi-doctor` `v1.0.0` is the first stable release line. The CLI exit-code
contract and the `check --json` v1 schema are treated as stable for normal
users and automation.

Security fixes are provided for:

- the latest released `1.x` line
- release candidates for the next public release while they are under staged
  validation

Older `0.x` releases are not supported. They may receive best-effort fixes only
when the change is low risk and does not conflict with the supported release
line.

## Reporting a Vulnerability

Please do not report vulnerabilities, secrets, private logs, or sensitive
support bundles in a public issue.

Use GitHub Security Advisories:

https://github.com/Halfblood-Prince/pi-doctor/security/advisories/new

Include:

- affected version or commit
- operating system and architecture
- commands needed to reproduce the issue
- whether the issue can expose host data, command output, files, or secrets
- a minimal sanitized reproducer when possible

## Disclosure Process

The maintainer will:

- acknowledge new reports within 5 days when GitHub notifications are working
- triage the report and determine severity before public disclosure
- prepare a fix, regression test, and release note
- publish a security advisory for confirmed vulnerabilities
- credit reporters when they want public credit

Public disclosure should wait until a fixed release or mitigation is available.

## Handling Sensitive Data

`pi-doctor support-bundle` is sanitized by default, but reporters should still
review every file before sharing. Use sensitive bundles only with explicit
acknowledgement and only through a private security advisory or another trusted
private channel.
