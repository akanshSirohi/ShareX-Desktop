# Known limits

- Windows is the primary validated platform. Automated builds provide macOS and Linux packages, but functional testing on native hardware is still required.
- Non-Windows drive discovery currently exposes a filesystem fallback. Use a single folder; mounted-volume discovery and FileVault/LUKS protection detection are not implemented.
- Windows shares exclude BitLocker-protected drives and drives whose protection state cannot be determined. This can exclude an otherwise usable volume.
- Both devices must be able to reach one another. Guest Wi-Fi isolation, VPN routing, firewalls, and separate subnets can prevent local connections.
- Self-signed HTTPS certificates require client trust. HTTP provides no transport encryption.
- Browser camera access depends on browser permissions and secure-context rules; QR image import and manual addressing remain available.
- Plugins execute browser code. Install plugins you trust and review their requested permissions.
- Tray behavior, file permissions, and network discovery vary by operating system and desktop environment.
- Release signing and macOS notarization are not configured. Maintainers must supply their own credentials for public distribution.

Report reproducible issues through the repository's issue templates. Send security details privately as described in [SECURITY.md](../SECURITY.md).
