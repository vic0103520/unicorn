# Unicorn Input Method Specification

## 1. Core Philosophy
Unicorn is a "Functional Core, Imperative Shell" input method. It follows the **Longest Matching Prefix** rule to ensure users can type extended sequences (e.g., `(1)` vs `(1)`) without premature commitment.

## 2. Activation & Deactivation
*   **Activation:** Typing the backslash `\` character activates "Unicorn Mode".
*   **ABC Fallback:** When not in Unicorn Mode (no active buffer), all keys are passed through to the system, making Unicorn behave exactly like the default "ABC" input method.
*   **Deactivation:** 
    *   Committing a symbol (usually).
    *   Typing an invalid character (Implicit Commit).
    *   Pressing Escape (Cancels composition).
*   **Stateful Commits (Re-activation):**
    *   Some sequences result in a commit followed immediately by a re-activation (e.g., `\==` commits `≡` and allows continued typing). The Backend signals this intent, and the Frontend must respect it.

## 3. Input Processing Logic (Priority Order)

When a key is pressed in Unicorn Mode, the Controller follows this strict priority sequence:

### A. Trie Continuation (Highest Priority)
The engine checks if the key extends the current buffer to a valid path in the Keymap (Trie).
*   **Match:** The key is consumed and added to the buffer.
    *   If the node has > 1 candidates: **Show Candidates Window**.
    *   Otherwise: **Update Composition** (Inline marked text).
*   **Leaf:** If the new path is a leaf node (no further children):
    *   **Single Candidate:** The candidate is automatically **Committed**.
    *   **No Candidate:** The raw buffer is **Committed**.

### B. Special Keys in Active Mode
*   **Backslash (`\`):**
    *   **Trigger Only (Buffer is `\`):** Commits a literal backslash `\` and restarts the active session (buffer resets to `\`).
    *   **With Composition:** Commits the **currently selected candidate** (or the raw buffer if no valid selection) and restarts the active session (buffer resets to `\`).
*   **Navigation Keys:**
    *   The frontend must update the Engine's internal `selected_candidate` index whenever the user navigates the candidate list. This ensures the Engine commits the correct symbol when `\` or other commit triggers are pressed.

### C. Candidate Selection (On Reject)
If the Trie **Rejects** the key (it's not a valid continuation), the Controller checks if it's a selection command **IF** the candidate window is visible.
*   **Digits (1-9):** Selects the candidate at the corresponding index (1-based).
*   **Space / Enter:** Selects the first candidate.
    *   **Space:** Commits candidate + Space.
    *   **Enter:** Commits candidate + Newline.
*   **Action:** The selected candidate is committed, and the engine resets. The key is consumed.

### C. Implicit Commit (Fallback)
If the key is rejected by the Trie AND is not a valid selection key, the system assumes the user has finished the sequence and started typing the next word.
*   **Check Buffer Candidates:** 
    *   **If candidates exist:** The first candidate (index 0) is **Committed** (converted).
    *   **If no candidates:** The raw buffer text is **Committed** (e.g., typing `\z` results in `\` then `z`).
*   **Pass-Through:** The rejected key is then passed to the system to be inserted normally.
    *   *Example:* Buffer `\l` (candidate `λ`). User types `z` (invalid). Result: `λ` is committed, then `z` is inserted -> `λz`.
    *   *Example:* Buffer `(1` (candidate `⑴`). User types `.` (invalid). Result: `⑴` committed, then `.` inserted -> `⑴.`.

## 4. UI Behavior & Navigation
*   **Composition:** Underlined text showing the current buffer (e.g., `\lambda`).
*   **Candidates Window:** A floating window appearing when multiple options exist.
*   **Navigation:** 
    *   **Up/Down Arrows:** Move selection one by one.
    *   **Left/Right Arrows (Paging):**
        *   **Right (Page Down):** Jumps to the start of the next page (10 items). If manually scrolled, it snaps to the next logical page relative to the current top item.
        *   **Left (Page Up):** Jumps to the start of the previous page.
*   **State Tracking:** The UI maintains a "Sliding Window" state (`firstVisibleCandidateIndex`) to ensure paging remains synchronized even if the user scrolls item-by-item using Down arrow.

## 5. Ambiguity Resolution (Numbers)
Scenario: User types `(1)`.
1.  `(`: Valid. Buffer `(`. Candidates: `["(", "⊂", ...]
2.  `1`: Is `(1` a valid path? **Yes.**
    *   **Trie Continuation takes precedence.** The `1` is added to the buffer.
    *   The user sees `(1` underlined. (First candidate might be `⑴`).
3.  `)`: Is `(1)` a valid path? **Yes.**
    *   Buffer `(1)`. Leaf node -> Commit `⑴`.

Scenario: User types `\l1`.
1.  `\`: Valid.
2.  `l`: Valid. Buffer `\l`. Candidates `["λ", ...]`.
3.  `1`: Is `\l1` a valid path? **No.** (Reject).
4.  **Fallback to Candidate Selection:** `1` is a digit. Candidates are visible. **Select Candidate 1.** Result: `λ`.
