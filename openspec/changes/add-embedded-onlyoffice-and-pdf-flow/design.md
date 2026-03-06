## Context
Office editing has backend versioning in place but still needs in-app embedded editor runtime.

## Decisions
- Preserve current office version persistence as baseline.
- Track OnlyOffice and PDF integration as explicit remaining execution tasks.

## Risks / Trade-offs
- Shipping embedded Document Server and PDF edit UX requires additional binary packaging and UI effort.
