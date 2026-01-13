# Project Context: Unicorn Input Method

## Overview
**Unicorn** is a cross-platform Unicode Input Method designed to allow easy insertion of Agda and Unicode symbols (e.g., typing `\lambda` to get `λ`).

The project follows a **"Functional Core, Imperative Shell"** architecture:
*   **Core:** A portable, high-performance state machine written in **Rust**. It handles the Trie traversal, symbol lookup, and input logic.
*   **Shell:** OS-specific frontends (currently **macOS**) that handle system events, rendering, and candidate windows, communicating with the core via UniFFI generated bindings.

## Architecture

### 1. Backend: `crates/core` (Rust)
*   **Role:** The brain of the input method.
*   **Data Structure:** Uses a nested `TrieNode` structure loaded from a JSON keymap.
*   **State Management:** The `Engine` struct tracks the current input buffer, Trie position (`path`), active status, and candidate selection.
*   **Modules:**
    *   `lib.rs`: Public API re-exports.
    *   `engine.rs`: Core logic (`Engine`, `TrieNode`, `EngineAction`).

### 2. Adapter: `crates/adapter-uniffi` (Rust)
*   **Role:** Exposes the Core API to foreign languages (Swift, Kotlin, Python) via UniFFI.
*   **API:**
    *   `Engine::new`: Initializing with JSON data.
    *   `Engine::process_key`: Handling character input and returning a list of `EngineAction`s.
    *   `Engine::get_candidates`: Retrieving suggestions.
    *   `Engine::select_candidate`: Updating the internal selected index.
    *   `Engine::deactivate`: Resetting the engine state.

### 3. Frontend: `apps/macos` (Swift)
*   **Role:** The "Dumb Pipe" for macOS.
*   **Framework:** Built on `InputMethodKit`.
*   **Integration:**
    *   Links against the generated UniFFI library.
    *   Initializes the Rust `Engine` with the path to the bundled `keymap.json`.
    *   Forwards user input to `engine.process_key`.
    *   Syncs UI selection state to the engine via `select_candidate`.
    *   Reacts to `EngineAction` actions: `Commit`, `UpdateComposition`, `ShowCandidates`, or `Reject`.
    *   **State Coordination:** Respects the Engine's state by allowing it to remain active after a commit if the Engine signals a session restart (e.g., typing `\==\`).

## Building and Running

### Rust Core & Adapter
The backend components are standard Rust crates.

**Build All (including FFI):**
```bash
cargo build --release
```

**Test Core Logic:**
```bash
cd crates/core
cargo test
```

### macOS Application
The frontend is an Xcode project located in `apps/macos/`.

1.  **Build:** Use the root `Makefile` for a streamlined build: `make build-macos`.
2.  **Install:** `make install-macos`.

## Key Files & Directories

*   `docs/SPECIFICATION.md`: Detailed architectural and behavioral specification.
*   **`crates/core/`**: The core logic implementation.
*   **`crates/adapter-uniffi/`**: UniFFI interface definitions.
*   **`apps/macos/`**: Swift implementation and project configuration.
    *   `unicorn/InputController.swift`: The heart of the macOS frontend.

## Development Conventions

*   **Logic in Rust:** All complex logic (Trie navigation, state transitions, candidate selection) belongs in `crates/core`.
*   **UI in Swift:** All UI handling (Candidates window, key interception, marked text) belongs in `apps/macos`.
*   **State Management:** The Rust `Engine` is the single source of truth. The Frontend should avoid forcing state changes (like `deactivate`) unless necessary for lifecycle management (e.g., focus loss).
*   **Testing:**
    *   **Unit Tests:** Logic sequences (e.g., `\lambda` -> `λ`) and complex state transitions are tested in `crates/core/src/engine.rs`.

## Current Status
*   **Rust Core:** Fully implemented with Trie traversal, candidate selection, and session-aware logic.
*   **macOS Frontend:** Integrated via UniFFI, handling composition and candidate navigation. Optimized for rapid typing and complex symbol sequences.