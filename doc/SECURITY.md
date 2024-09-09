# Security Reporting

When hosting (1) a web service that (2) accepts user input, security problems can happen.

## Reporting Security Issues

Firstly, thank you for your interest in filing a report!
Nobody wants to be hacked, us included.

Please privately report security issues through [GitHub's Security Advisories](https://github.com/Jaxydog/1N4/security/advisories/new) feature.
Do not file a public issue to report a security issue.

## Got an Issue That You Want to Fix?

Begin with a [private security report](#reporting-security-issues).
Once the private report is submitted, you will be able to open a temporary private fork.
Use this to propose a fix, or collaborate with us on it.

For more information, see ["Privately report a security vulnerability"](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability) from GitHub's documentation.

For generic contribution guidelines,
see [`CONTRIBUTING.md`](./CONTRIBUTING.md).

## Security in 1N4

1. Discord bots do not accept arbitrary connections,
   only WebSocket connections and outgoing HTTP requests to Discord's servers.
1. Though its dependencies do, 1N4 does not use unsafe code.
