# Resilience & Fallback Plans (LD-47)

Authoritative reference for escape hatches and known-unsupported configurations.
Downstream stories that touch the filesystem watcher, atomic writes, or the
webview host MUST `grep` this file before assuming a configuration is supported.

## Filesystem Watcher — Unsupported Configurations (LD-9)

The external-edits watcher ([`crates/orgsidian-watcher/`](../../crates/orgsidian-watcher/),
Story 5.1) wraps `notify-rs` and coalesces atomic-save event bursts into a
single change per file. Timely detection — an external write reported within 5
seconds — is verified on macOS, Linux, and Windows, on **local** storage. The
following configurations are **unsupported in v0.1**: Orgsidian does not
guarantee correct external-edit handling on them, and users are advised to keep
their Vault on local storage.

Note the distinction between the two axes below. The 5-second detection latency
is verified on each platform's default local filesystem regardless of its
case-sensitivity; what case-*folding* filesystems break is **path identity**
(the aliasing described below), not detection latency.

- **Network mounts** (NFS, SMB/CIFS, SSHFS, cloud-sync overlay folders such as
  Dropbox/Google Drive/OneDrive virtual filesystems). The OS change-notification
  backends `notify-rs` relies on — `FSEvents` (macOS), `inotify` (Linux), and
  `ReadDirectoryChangesW` (Windows) — do not reliably report events for files
  changed by another host across a network mount, so external writes may be
  missed or arbitrarily delayed. Cloud-sync overlays additionally emit
  synthetic event storms that the 250ms debounce is not calibrated for.

- **Case-folding / case-insensitive filesystems** (default APFS and HFS+ on
  macOS, NTFS and exFAT on Windows). Path identity in the watcher and index is
  byte-exact and case-sensitive; on a case-folding filesystem `Notes.org` and
  `notes.org` denote the same on-disk file but distinct watcher/index keys,
  which can desynchronize change tracking. Case-sensitive local filesystems
  (ext4, case-sensitive APFS volumes) are the supported configuration.

Full support for these configurations is out of scope for v0.1 Alpha and
tracked for a later milestone.

## Webview Host — Tauri Escape Hatch (LD-47)

If Tauri 2.x evolves in a way that breaks Orgsidian's window/event/IPC needs,
the pre-budgeted fallback (~3 weeks) is to drop to `wry` directly with custom
window, event, and IPC plumbing. Documented here as a deliberate escape hatch;
not planned work.
