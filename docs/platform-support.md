# Platform support

| Platform | Status |
| --- | --- |
| macOS 15+ on Apple Silicon | Supported. This is the only tested target. |
| Intel Macs, Linux, Windows | Not supported and not tested. |

The bundle's minimum system version is `LSMinimumSystemVersion` in
`apps/mterm/macos/Info.plist`; `scripts/app.sh` links against it.

The data directory is `~/Library/Application Support/mTerm`, and the terminal font
is Menlo, which ships with macOS.
