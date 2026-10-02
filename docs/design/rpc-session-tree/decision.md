# Decision — RPC Session Tree Surface

- **Status:** confirmed
- **Decision key:** rpc-session-tree-v1
- **Scope:** A minimal RPC surface for the external Pidian client to reproduce the classic TUI session-tree behavior.
- **Implementation status:** Not implemented.
- **Contract status:** The v1 public semantics are frozen; source-level integration and executable contract tests remain to be planned and implemented.

## Problem

The classic TUI can render the persisted session graph, move the active leaf to a historical entry, and treat a selected user message as text to be resubmitted. An external client cannot reuse the terminal renderer or `TreeUiState`, while the current RPC surface does not expose the complete session tree or same-session leaf navigation.

Pidian is the RPC client for this surface. It must not write session JSONL directly or depend on terminal-specific TUI state.

## Direction

Expose only the session facts and mutation that the client cannot safely implement itself:

~~~text
get_session_tree  -> one atomic snapshot of the complete current-session graph
 tree_navigate     -> move the active context cursor within the current session
~~~

The client owns:

~~~text
tree layout, indentation, expansion, scrolling, hover state, mouse handling,
selection state, user-only/show-all filters, and user-message resubmission
mapping
~~~

The Pi RPC server owns:

~~~text
entry validation, session mutation, transition admission, persistence,
Agent-context replacement, lifecycle compatibility, bounded preview generation,
and stable RPC error codes
~~~

## Key choices

1. **Keep the session tree flat on the wire.** Each entry carries its persisted `id` and `parentId`; clients reconstruct their own tree. `parentId`, not array position, defines the structure.

2. **Retain `targetLeafId` as an established Pi term, with an explicit RPC definition.** It does not mean a terminal leaf in the tree data structure. It means the entry ID that becomes the active leaf/context cursor after navigation. It may identify an assistant, tool, compaction, or other entry. `null` means root.

3. **Use one generic mutation.** `tree_navigate` accepts an existing entry ID or `null` for root. Pidian maps a selected user/custom entry to its `parentId` and restores its text locally. A separate `tree_resubmit` command is not required.

4. **Require strict stale-transition protection.** External mutations must carry `expectedSessionId` and `expectedLeafId`. Both fields are required. `expectedLeafId` is a string when the current cursor is an entry and `null` when the current cursor is root. Missing, malformed, or mismatched values reject without installing a candidate or replacing Agent context.

5. **Make root navigation idempotent.** `targetLeafId: null` with `expectedLeafId: null` succeeds when the session is already at root and does not create a new turn, session, or persistence mutation.

6. **Return one atomic tree snapshot.** `sessionId`, `activeLeafId`, and `entries` must come from the same read-only session snapshot. `entries` contains every currently stored navigable entry, including inactive branches. The array uses the session store's canonical/persistent entry order, normally JSONL order. Clients may rely on order stability within one snapshot, but must not infer tree structure or a depth-first algorithm from it.

7. **Separate bounded previews from resubmission text.** `preview` is server-generated, single-line, bounded display text and must never expose unrestricted tool output. `resubmitText` is the full persisted editable text for user/custom entries and must not reuse preview truncation. The RPC layer must not silently alter resubmission text; any existing prompt/entry limit is an upstream/session rule. Unknown entry kinds must use the same safe preview helper.

8. **Use an open `kind` string.** The wire value is not a closed enum. The server preserves unknown kinds; `other` means the server itself could not classify an entry. Pidian renders unknown values as `other`. Only `user` and `custom` make `resubmitText` actionable; an unknown kind must not be automatically resubmitted even if it carries such a field.

9. **Reuse the common response envelope and add an RPC-layer stable code.** Tree errors do not introduce a tree-specific response shape. The common envelope gains an optional top-level `code` field for machine-readable request semantics; `error` remains human-readable and `errorHints` remains supplementary. Existing RPC commands may continue omitting `code` for compatibility. New tree commands should provide one of the v1 `RpcErrorCode` values on every expected failure: `invalid_request`, `stale_session`, `stale_leaf`, `unknown_target`, `transition_blocked`, `persistence_failed`, `session_unavailable`, or `quarantined`. This mapping belongs in the RPC layer and must not overload `Error::category_code()`, whose coarse categories are not request semantics.

10. **Define unknown targets relative to the live session.** `unknown_target` means a non-null `targetLeafId` cannot be resolved as a navigable entry in the current live session. v1 does not promise to distinguish a typo, an entry from another session, an old snapshot, or an entry not loaded into the live session. `target_not_in_session` is not a v1 code; it may be added only when the protocol gains an explicit cross-session target contract.

11. **Keep tree queries pure with respect to entry IDs.** `get_session_tree` must not call `ensure_entry_ids()`, generate temporary IDs, mutate entries, persist JSONL, or return a partial tree. Session loading/creation or an explicit migration boundary must establish stable IDs before the query. If a live session still contains an entry without a stable ID, return `session_unavailable` or `quarantined` according to the existing session-integrity state. Tree navigation must use the same already-stable IDs.

## Non-goals

- Reproducing the classic TUI renderer or keyboard state machine in RPC.
- Making the default FrankenTUI `/tree` overlay interactive.
- Changing JSONL/session persistence semantics.
- Automatically submitting a prompt after a user entry is selected.
- Adding arbitrary session-file access to RPC.
- Returning unrestricted tool output through the tree query.
- Adding a revisioned subscription protocol in v1.
- Adding branch-summary generation to basic navigation.

## Assumptions

- The RPC process owns one live `AgentSession`.
- Tree queries and mutations are serialized or snapshotted through the existing session transition authority.
- `targetLeafId = null` is a valid root transition.
- `expectedLeafId = null` is the only valid expected value when the current cursor is root.
- Existing `get_messages` is sufficient for refreshing the client's current path after a successful navigation.
- The session store provides stable entry and parent identities and a canonical persistent entry order.
- The session store retains the editable source text needed for user/custom `resubmitText`.
- The common RPC response envelope can carry an optional top-level `code`; the tree implementation must add an RPC-layer `RpcErrorCode` mapping without reusing `Error::category_code()`.

## Re-evaluation conditions

Revisit this decision if:

- RPC must support multiple independent writers against the same session file;
- tree mutations need a revisioned subscription protocol rather than request/response refreshes;
- branch summaries become a required part of the external-client experience;
- extension consumers require dedicated `session_before_tree` or `session_tree` hooks;
- the session store changes the stable entry/parent identity or canonical-order contract;
- the existing RPC error envelope cannot represent stable tree error codes;
- the session store cannot expose full editable user/custom text without introducing a new persistence or security boundary.
