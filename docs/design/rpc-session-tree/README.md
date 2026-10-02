# Pi Agent Rust — RPC Session Tree Surface

> **Status:** Draft design for review; no implementation is included.
>
> **Scope:** Minimal RPC surface for an external client such as Pidian to
> reproduce the classic TUI session-tree behavior.
>
> **Decision:** [decision.md](decision.md), decision key
> rpc-session-tree-v1.

## Goal

Expose the minimum session-tree capability needed by an external client:

- query the complete current session graph;
- display historical user, assistant, tool, compaction, and other entries;
- move the active context to a selected historical entry;
- let the client re-submit a user/custom entry by navigating to its parent and
  putting its text back in the editor;
- preserve abandoned branches in the same session file.

The RPC server owns session and Agent state. The client owns presentation and
selection behavior. The client must not import terminal-specific TUI state or
write the session JSONL directly.

## Boundary

The classic TUI already provides the relevant session operations through
src/interactive/tree.rs and src/interactive/tree_ui.rs. The RPC surface only
extracts the session facts and mutation needed by an external client.

The existing RPC transition authority remains the single authority for tree
mutations. The implementation must reuse the same admission, persistence, and
Agent-context rules as other session transitions.

## Minimal RPC surface

The first version has two commands:

~~~text
get_session_tree  -> query the complete graph
tree_navigate     -> move the active leaf within the current session
~~~

There is deliberately no separate tree_resubmit command. Pidian can derive
the resubmit target from the selected node's parentId and use the same
navigation command.

### get_session_tree

Request:

~~~json
{
  "id": "tree-1",
  "type": "get_session_tree"
}
~~~

Response:

~~~json
{
  "type": "response",
  "command": "get_session_tree",
  "id": "tree-1",
  "success": true,
  "data": {
    "sessionId": "session-abc",
    "activeLeafId": "entry-7",
    "entries": [
      {
        "id": "entry-1",
        "parentId": null,
        "kind": "user",
        "preview": "Implement an HTTP client",
        "resubmitText": "Implement an HTTP client"
      },
      {
        "id": "entry-2",
        "parentId": "entry-1",
        "kind": "assistant",
        "preview": "I will inspect the current client boundary..."
      }
    ]
  }
}
~~~

entries must contain every graph entry, including entries outside the active
path. The server must define a stable ordering for the array; clients may use
that order when ordering sibling nodes. Each entry contains:

~~~text
id             persisted session entry ID
parentId       nullable persisted parent ID
kind           user | assistant | tool_result | bash | compaction |
               branch_summary | custom | other
preview        bounded display text; never unrestricted tool output
resubmitText   full editable text for user/custom entries only
~~~

activeLeafId may be null when the session is at root. resubmitText is needed
because a user entry on an inactive branch is not available through the
existing current-path get_messages command.

The server does not need to return role, isLeaf, isOnActivePath,
childrenCount, canResubmit, or leaves. Pidian can derive those values from
entries, parentId, kind, and activeLeafId. User-only and show-all filters are
also client-side presentation state, not session state.

### tree_navigate

Moves the active leaf to an existing entry, or to root when targetLeafId is
null.

Request:

~~~json
{
  "id": "tree-nav-1",
  "type": "tree_navigate",
  "targetLeafId": "entry-2",
  "expectedSessionId": "session-abc",
  "expectedLeafId": "entry-7"
}
~~~

expectedSessionId and expectedLeafId protect against a stale client
selection. They should be required for external clients. A missing or changed
expected value must reject the request without installing the candidate.

For a non-null targetLeafId, the server validates that the entry belongs to
the current live session. For a null targetLeafId, it resets the leaf to root.

Success response:

~~~json
{
  "type": "response",
  "command": "tree_navigate",
  "id": "tree-nav-1",
  "success": true,
  "data": {
    "sessionId": "session-abc",
    "oldLeafId": "entry-7",
    "newLeafId": "entry-2"
  }
}
~~~

The command does not start an Agent turn and does not create a new session.
The existing get_messages command can be called after success to refresh the
client's current-path display.

## Client selection semantics

Pidian implements the selection behavior locally:

~~~text
assistant/tool/compaction/other entry:
    targetLeafId = selected.id

user/custom entry:
    targetLeafId = selected.parentId (null means root)
    after success, put selected.resubmitText in the editor
~~~

This reproduces the classic TUI behavior without making the RPC server aware
of mouse, cursor, modal, or editor state. Selecting a user/custom entry does
not automatically submit it.

The minimal Pidian flow is:

~~~text
get_session_tree
    → render/filter locally
    → tree_navigate
    → get_messages
    → optionally restore resubmitText in the editor
~~~

## Server transition behavior

Every tree_navigate mutation follows the existing RPC session-transition
authority:

1. Validate the command and target leaf.
2. Reject streaming, compaction, pending acknowledged input, pending
   extension actions, or active background bash according to the existing RPC
   transition policy.
3. Verify expectedSessionId and expectedLeafId.
4. Clone the live Session into a candidate.
5. Call Session::navigate_to for an entry target or Session::reset_leaf for a
   root target.
6. Persist the candidate using the existing save and error-reconciliation
   rules.
7. Replace the live Session only after the transition is accepted.
8. Replace Agent messages with candidate.to_messages_for_current_path().
9. Return the new leaf IDs.
10. Reuse the existing session-switch lifecycle event with a tree-navigation
    reason.

A failed or unconfirmed persistence transition must not install the candidate
into the live Agent session. Existing provider-admission quarantine behavior
remains authoritative when persistence becomes indeterminate.

## Scope

### Included in v1

- complete current-session graph query;
- stable entry and parent IDs;
- bounded previews and user/custom resubmit text;
- same-session leaf navigation, including root;
- stale-session/stale-leaf rejection;
- abandoned-branch preservation;
- persistence and Agent-context replacement under existing RPC rules;
- no automatic prompt submission.

### Deferred

- branch-summary generation and custom summary prompts;
- a separate tree_resubmit RPC;
- a dedicated tree-change subscription event;
- unbounded tool-output retrieval through the tree endpoint;
- server-side user-only/show-all filtering;
- copying classic TUI rendering or keyboard handling into RPC.

Branch summaries may later be added as an optional field on tree_navigate
without changing the tree query or client-side tree model. The safe v1
behavior is no summary.

## Invariants

- Entry IDs and parent IDs remain the persisted session identities.
- A mutation only targets the current live RPC session.
- A stale client selection never silently overwrites a newer leaf.
- Abandoned branches remain addressable after navigation.
- The live Agent projection equals the selected candidate session path.
- Tree mutations are rejected while the existing transition blocker rejects
  session changes.
- Persistence failure is not reported as success.
- Tree previews are bounded and do not expose unrestricted sensitive tool
  payloads.
- Selecting a user/custom entry never starts a model turn automatically.

## Verification

An implementation should prove at minimum:

1. A linear session query returns stable IDs, parent links, and the active
   leaf.
2. A branched session query returns every branch, not only the active path.
3. Navigating to an assistant entry updates the active leaf, persists, and
   replaces Agent context.
4. Navigating to a tool or compaction entry follows the same non-user path.
5. Navigating to null resets to root and preserves the old branch.
6. A client can resubmit a user entry by navigating to its parent and using
   resubmitText without starting a turn.
7. A stale session or leaf is rejected without mutation.
8. Existing streaming/compaction/pending-input blockers reject the mutation.
9. Save failure leaves the live session and Agent projection unchanged unless
   existing reconciliation proves the exact transition.
10. Existing get_messages, fork, rewind, retry, and switch_session behavior
    does not regress.

## Notes

The current Pidian client already has a local get_tree compatibility path.
That client-side name may remain inside its adapter, but it must not preserve
the old semantics of calling fork and changing sessionPath. The new server
contract is same-session navigation through get_session_tree and
tree_navigate.

The protocol is intentionally small. If a future client needs summary
generation or a long-lived tree subscription, those should be added as
explicit extensions rather than making them required for basic navigation.
