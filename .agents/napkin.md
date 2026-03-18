# Napkin

## Corrections
| Date | Source | What Went Wrong | What To Do Instead |
|------|--------|----------------|-------------------|

## User Preferences
- For coding/debugging/testing/review tasks, prioritize technical implementation detail and concrete verification.
- For repo reviews, findings should be the primary output, ordered by severity with file/line references.

## Patterns That Work
- Start by checking repo instructions plus `bd` workflow, then inspect both the Rust crates and the MCP/Tauri surfaces before making claims about app behavior.
- On macOS 26+, Rust tests that compile `whisper-rs` need `CXXFLAGS="-I$(xcrun --show-sdk-path)/usr/include/c++/v1"`; core tests pass once that is set.

## Patterns That Don't Work
- Assuming this repo is only a CLI tool misses the Tauri desktop app and MCP integration surfaces that need review too.
- Trusting `path.resolve(...).startsWith(...)` in Node is not a safe allowlist check here; it misses sibling-prefix and symlink cases.

## Domain Notes
- `minutes` is a local-first meeting capture app with Rust core/CLI, a Tauri desktop app, and a TypeScript MCP server.
- The worktree may already contain user changes; review around them carefully and do not revert unrelated edits.
- The desktop app mixes in-memory recording state with PID-file-based status, so app restarts and cross-surface recording flows are easy places for desync bugs.
