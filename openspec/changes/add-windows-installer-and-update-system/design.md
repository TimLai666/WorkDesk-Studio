## Context
Current scripts prepare payloads but do not yet produce MSI installers or full updater orchestration.

## Decisions
- Use MSI/WiX as target packaging line.
- Separate application binary updates from toolchain updates.
- Keep rollback primitives in runner toolchain manager.

## Risks / Trade-offs
- Full production updater requires signing, feed service, and installer authoring not yet complete.
