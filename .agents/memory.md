# Project Memory

Store only non-obvious product/API decisions that are not already clear from code, tests, README, or other `.agents` files. Remove entries once they become implemented, tested, and obvious elsewhere.

---

## Clockify API: Time entry update requires start
Date: 2026-04-23
The documented update payload for `PUT /v1/workspaces/{workspaceId}/time-entries/{id}` includes required `start`. Build update logic accordingly.

## Clockify API: Pagination naming is endpoint-specific
Date: 2026-04-23
Clockify documentation is not fully uniform in query parameter naming. Use the documented parameter names for each endpoint exactly as shown.

## Clockify API: tagIds may be null
Date: 2026-04-23
Time-entry responses may contain `tagIds: null`. Treat that as an empty tag list during deserialization.

## Reference
Date: 2026-04-23
Primary API reference: https://docs.clockify.me/
