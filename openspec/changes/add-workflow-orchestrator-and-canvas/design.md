## Context
Existing runner completed runs through a placeholder path. Phase-1 needs node-level execution observability and deterministic sequencing.

## Decisions
- Persist node state rows in `workflow_run_nodes` for every queued run.
- Execute nodes in topological order from workflow edges.
- Treat node-level failures as run failure with event evidence.

## Risks / Trade-offs
- Full n8n-grade visual canvas interactions remain a larger UI slice; current milestone prioritizes executable orchestration and run introspection.
