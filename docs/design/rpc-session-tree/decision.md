# Decision — RPC Session Tree Surface

- **Status:** exploring
- **Decision key:** rpc-session-tree-v1
- **Scope:** A minimal RPC surface for external clients that want the classic
  TUI session-tree behavior.
- **Implementation status:** Not implemented.

## Problem

The classic TUI can render the persisted session graph, move the active leaf
to a historical entry, and treat a selected user message as text to be
resubmitted. An external client cannot reuse the terminal renderer or
TreeUiState, while the current RPC surface does not expose the complete
session tree or same-session leaf navigation.

## Direction

Expose only the session facts and mutation that the client cannot safely
implement itself:

~~~text
get_session_tree  -> complete flat entry graph and active-leaf metadata
tree_navigate     -> move the active leaf within the current session
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
Agent-context replacement, and lifecycle compatibility
~~~

## Key choices

1. **Keep the session tree flat on the wire.** Each entry carries id and
   parentId; clients reconstruct their own tree.
2. **Use one generic mutation.** tree_navigate accepts an existing entry ID or
   null for root. Pidian maps a selected user/custom entry to its parent and
   restores resubmitText locally. A separate tree_resubmit command is not
   required.
3. **Return only client-required entry data.** Previews are bounded. Full
   editable text is returned only as resubmitText for user/custom entries.
   Derived presentation fields remain client-local.
4. **Require stale-transition protection.** External mutations carry both
   expectedSessionId and expectedLeafId.
5. **Reuse existing transition safety.** Tree mutations use the same authority
   as switch_session, fork, rewind, and retry.
6. **Do not add summary or subscription requirements to v1.** The initial
   navigation behavior preserves branches without generating summaries and
   uses the existing session-switch lifecycle compatibility. Summary generation
   and a dedicated tree event can be added later.

## Non-goals

- Reproducing the classic TUI renderer or keyboard state machine in RPC.
- Making the default FrankenTUI /tree overlay interactive.
- Changing JSONL/session persistence semantics.
- Automatically submitting a prompt after a user entry is selected.
- Adding arbitrary session-file access to RPC.
- Returning unrestricted tool output through the tree query.

## Assumptions

- The RPC process owns one live AgentSession.
- Tree mutations are serialized through its existing transition authority.
- targetLeafId = null is a valid root transition.
- Existing get_messages is sufficient for refreshing the client's current
  path after a successful navigation.
- The session store continues to provide stable entry and parent identities.

## Re-evaluation conditions

Revisit this decision if:

- RPC must support multiple independent writers against the same session file;
- tree mutations need a revisioned subscription protocol rather than
  request/response refreshes;
- branch summaries become a required part of the external-client experience;
- extension consumers require dedicated session_before_tree or session_tree
  hooks;
- the session store changes the stable entry/parent identity contract.
