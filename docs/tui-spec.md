# TUI Parity Spec: yoinks

This document captures the expected behavior of the TUI as implemented in `pablostanley/yoinks` to serve as the parity contract for `yoink.web.rs`.

## 1. Interaction Flow

1. **Launch**: Starts in full-screen alternate buffer.
2. **Input Stage**:
   - Displays logo animation.
   - Shows "Paste a link" framed input.
   - If a valid URL is in the system clipboard, prompts: `link in your clipboard — ⇥ to paste it`.
   - On submission, transitions to "Probing".
3. **Probing Stage**:
   - Displays platform/site name.
   - Displays status: `warming up…`, `fetching video info…`.
   - Transitions to "Picking" or "Error".
4. **Picking Stage**:
   - Displays video title and platform details.
   - Shows resolution picker in a panel (e.g., `1080p · mp4`, `audio only · mp3`).
   - Arrow keys to select; Enter to submit.
5. **Downloading Stage**:
   - Displays progress bar, speed, and ETA.
   - Handles multi-part downloads (part N/M).
   - Cancellation with `Esc` kills the process and cleans up partial files.
6. **Done Stage**:
   - Displays success checkmark and file path.
   - Prompt: `[↵ yoink another]`.
7. **Exit**:
   - Exits alternate buffer, restores terminal.
   - Prints `✓ yoinked → <filepath>` to terminal.

## 2. Keybindings & Interactions

- **`↑ / ↓`**: Move selection (history, format picker).
- **`↵` (Enter)**: Submit/Pick choice.
- **`Esc`**: Go back / cancel run.
- **`^c`**: Exit immediately.
- **`^t`**: Cycle theme (`auto` -> `light` -> `dark`).
- **`⇥` (Tab)**: Paste clipboard content in Input Stage.

## 3. UI Components

- **FramedInput**: Custom border-framed input (top-embedded title).
- **Panel**: Bordered panel for format picking.
- **ProgressBar**: Block-based progress (`█`, `░`).
- **Shortcuts**: Footer row showing available hotkeys in `key label` pairs separated by `·`.
- **FullScreen**: Full-screen centered layout wrapper.

## 4. Error Handling
- Invalid URLs: Show `✗ that doesn’t look like a link — paste a full url`.
- Probe/Download Failures: Show error message in framed box with `[↵ try again]` prompt.
