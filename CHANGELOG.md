# Changelog

All notable changes to ClipVault are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [1.1.0] - 2026-11-23

### Added
- The popup can be dragged by its header, including to another monitor; it still reopens next to
  the caret or pointer.

### Security
- Updated the `cookie` dependency to 0.18.3 (RUSTSEC-2026-0594). ClipVault was not affected: the
  issue only occurs in processes that fork, which Windows does not do.

## [1.0.0] - 2026-11-22

First public release on the Microsoft Store.

### Added
- Clipboard history for text and images, with pinning, search, type filters and a virtualized list.
- Popup opened by a configurable global shortcut (default `Ctrl + Win + V`) that pastes into the
  previously focused app and returns focus to it.
- Snippets with their own global shortcuts; the previous clipboard is restored after pasting.
- Privacy: content flagged by password managers is ignored, apps can be excluded, capture can be
  paused, and nothing leaves the device.
- Retention by age and count, pinned items that never expire, and optional clearing on exit.
- Settings window with General, History, Shortcuts, Privacy, Appearance and About sections.
- English and Portuguese (Brazil), switchable without restarting.
- Light and dark themes following Windows, Windows accent color, high contrast support.
- Start with Windows (MSIX StartupTask) after the user's consent.
- A clear message, with the download link, when the WebView2 Runtime is missing.
- MSIX packages for x64 and arm64.
