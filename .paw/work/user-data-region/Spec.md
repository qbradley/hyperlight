# Feature Specification: User Data Region

**Branch**: feature/user-data-region  |  **Created**: 2026-06-12  |  **Status**: Draft
**Input Brief**: User-supplied PRD for an opt-in shared host/guest user data buffer.

## Overview

Embedders that exchange payloads between host and guest need a bounded data area for workloads where the existing structured call channels are not a good fit. The User Data Region gives those embedders a construction-time choice to reserve a shared byte region sized for their workload.

The region is for transient exchange between one host-managed execution flow and guest code. Host-written bytes must be visible to the guest, guest-written bytes must be visible to the host, and each operation must be constrained by the capacity the embedder selected.

Embedders that do not opt in must see the same behavior they rely on today. A default configuration reserves no user data capacity, existing sandbox lifecycle behavior remains unchanged, and restored executions begin without stale bytes from earlier writes to the region.

The first release supports exchange while the host reads before any restore operation clears transient user data. If a convenience execution path restores sandbox state before the host reads, the user data region is cleared as part of that restore and guest mutations are not preserved for later host reads.

Guest discovery exposes the region address and capacity; Rust helpers may present that as a bounded slice, while C helpers expose pointer-and-length metadata for caller-bounded `memcpy` use. These helpers make the contract explicit but do not create a hardware guard boundary around the region.

## Objectives

- Let embedders choose whether a sandbox has a user data region and how many bytes it can hold.
- Let host and guest participants exchange byte payloads through the configured region.
- Reject attempts to write more bytes than the configured capacity.
- Ensure restored executions do not inherit earlier user data contents.
- Preserve current behavior for sandboxes that do not configure user data.
- Provide discovery support for the repository's supported Rust and C guest surfaces.

## User Scenarios & Testing

### User Story P1 - Configure a shared user data capacity

Narrative: As an embedder, I want to opt into a fixed user data capacity when constructing a sandbox so each sandbox advertises exactly the byte capacity I selected.

Independent Test: Construct sandboxes with zero and positive configured capacities and verify the reported capacity in each case.

Acceptance Scenarios:
1. Given an embedder does not configure user data capacity, When a sandbox is created, Then the reported user data capacity is zero.
2. Given an embedder configures a positive user data capacity, When a sandbox is created, Then host and guest participants can discover that exact capacity.

### User Story P2 - Exchange byte payloads through the region

Narrative: As an embedder, I want to place a byte payload in the configured region, have guest code read or change it, and read the resulting bytes back on the host.

Independent Test: Write a payload within the configured capacity, invoke guest code that transforms the bytes, and verify the host reads the transformed result.

Acceptance Scenarios:
1. Given a configured user data region and a payload whose length is less than or equal to the configured capacity, When the host writes the payload, Then guest code can read the same byte sequence.
2. Given guest code changes bytes in the user data region and no restore occurs before the host read, When the host reads the same number of bytes back, Then the host observes the guest-written values.
3. Given guest code changes bytes in the user data region and a restore occurs before the host read, When the host reads the region after restore, Then the earlier guest-written bytes are no longer present.
4. Given a payload whose length is greater than the configured capacity, When the host attempts to write it, Then the write fails without modifying any bytes in or adjacent to the configured region.

### User Story P3 - Restore without stale user data

Narrative: As an embedder using reusable sandboxes, I want restored executions to start without previously exchanged user data so each restored run begins with a clean transient exchange region.

Independent Test: Write non-zero data to the region, restore the sandbox from an earlier state, and verify the region no longer contains the previously written bytes while unrelated restored behavior still works.

Acceptance Scenarios:
1. Given data has been written to the user data region, When the sandbox is restored from an earlier state, Then reading the configured region returns zeroed bytes.
2. Given a sandbox has zero configured user data capacity, When restore is performed, Then the sandbox continues to operate as it did before this feature was added.

### Edge Cases

- Zero configured capacity means non-empty writes exceed capacity and fail.
- Empty reads and writes succeed and do not change observable region contents.
- Reads cannot return bytes beyond the configured capacity.
- Payload lengths of one byte, exactly the configured capacity, and greater than the configured capacity have defined outcomes.
- Non-page-aligned positive capacities have the exact configured capacity, not a rounded capacity.
- Capacity values that cannot be represented safely within sandbox memory bounds fail deterministically.
- Restoring a sandbox with a configuration that is incompatible with the saved state fails rather than silently reusing mismatched user data settings.
- Official host and guest helpers enforce capacity bounds; arbitrary guest pointer misuse outside those helpers is not isolated by a dedicated guard boundary in this release.
- Fresh positive-capacity regions read as zero before the first host or guest write.
- Reads after failed guest execution are not a successful handoff signal; callers should treat contents as application-defined unless the guest completed the expected exchange protocol.

## Requirements

### Functional Requirements

- FR-001: The system MUST support a configurable user data capacity with a default of zero bytes. (Stories: P1)
- FR-002: The system MUST report the configured user data capacity to host-side consumers. (Stories: P1, P2)
- FR-003: The system MUST report the configured user data capacity and starting address to guest-side consumers. (Stories: P1, P2)
- FR-004: The system MUST allow the host to write a byte sequence whose length is less than or equal to the configured capacity. (Stories: P2)
- FR-005: The system MUST reject host writes whose length is greater than the configured capacity. (Stories: P2)
- FR-006: The system MUST allow the host to read a requested byte sequence only from within the configured capacity. (Stories: P2)
- FR-007: The system MUST make host-written bytes observable to guest code and guest-written bytes observable to the host. (Stories: P2)
- FR-008: The system MUST clear user data contents when restoring a sandbox from an earlier state. (Stories: P3)
- FR-009: The system MUST preserve sandbox creation, execution, and restore behavior for configurations with zero user data capacity. (Stories: P1, P3)
- FR-010: The system MUST reject restore or snapshot reuse when user data capacity differs between the saved state and the target sandbox. (Stories: P3)
- FR-011: The system MUST define the supported execution lifecycle for reading guest mutations and MUST make restore-clearing behavior observable and documented. (Stories: P2, P3)
- FR-012: The system MUST provide user data discovery support for supported Rust and C guest consumers, including enough capacity metadata for callers to bound direct memory copies. (Stories: P1, P2)
- FR-013: The system MUST reject configured capacities whose derived bounds or addresses cannot be represented safely. (Stories: P1)
- FR-014: The system MUST initialize fresh positive-capacity user data regions to zero before first guest-visible use. (Stories: P1, P3)

### Key Entities

- User Data Region: Optional fixed-capacity byte storage for transient host/guest exchange.
- User Data Capacity: The number of bytes selected by the embedder at sandbox construction time.
- Region Discovery Metadata: The address and capacity information guest code uses to locate the region.

### Cross-Cutting / Non-Functional

- Host operations MUST be bounded by the configured capacity for every read and write.
- A zero-capacity configuration MUST require no action from existing embedders.
- The configured capacity MUST be the single source of truth for host and guest bounds.

## Success Criteria

- SC-001: With default configuration, existing construction, execution, and restore tests continue to pass without requiring caller changes. (FR-001, FR-009)
- SC-002: For configured capacities of zero bytes, one byte, and at least one multi-KiB value, host and guest consumers report the exact configured capacity. (FR-001, FR-002, FR-003)
- SC-003: Host-to-guest-to-host round trips preserve or transform byte sequences correctly for payload lengths of zero, one, half capacity, and exact capacity. (FR-004, FR-006, FR-007)
- SC-004: A write of configured capacity plus one byte fails and preserves the previously written in-region bytes and adjacent state. (FR-005)
- SC-005: After restore, every byte in a positive-capacity user data region reads as zero, including when restore is performed by a convenience execution path before the host reads. (FR-008, FR-011)
- SC-006: Restore or snapshot reuse across two configurations that differ only by user data capacity fails deterministically. (FR-010)
- SC-007: Rust and C guest consumers can discover matching region address and capacity values for the same sandbox configuration. (FR-003, FR-012)
- SC-008: Impossible capacity values fail before sandbox execution, and non-page-aligned positive capacities retain their exact configured capacity. (FR-013)
- SC-009: Before any host or guest write, every byte in a fresh positive-capacity user data region reads as zero from host and guest consumers. (FR-014)

## Assumptions

- Initial host reads and writes operate from the beginning of the region; random-access subrange operations are not part of the first release.
- Guest code receives enough discovery information to locate both the beginning and the capacity of the region.
- The first release uses official helper/API-level capacity bounds rather than a dedicated guard boundary against arbitrary in-guest pointer misuse.
- The first release supports configured sizes up to existing sandbox memory limits, with representative KiB and MiB-scale validation or guidance.
- The C guest helper exposes raw pointer-and-capacity metadata; C guest callers remain responsible for using the reported capacity when copying bytes.
- Performance validation will use existing benchmark infrastructure if suitable, but passing functional and lifecycle behavior is required for the initial release.

## Scope

In Scope:
- Construction-time configuration of user data capacity.
- Host-side whole-region read and write behavior.
- Rust and C guest-side discovery of region address and capacity.
- Compatibility checks for saved state and restored sandboxes.
- Tests for configuration, discovery, data exchange, capacity enforcement, restore clearing, impossible capacities, and non-page-aligned capacities.
- Documentation for the new observable behavior.

Out of Scope:
- Replacing the existing structured call and return protocol.
- Preserving user data contents across restore.
- General writable external mappings.
- Dedicated VM/page-level guard isolation around the user data region.
- Random-access subrange operations.
- Migration work in downstream applications.

## Dependencies

- Existing sandbox construction and lifecycle flows.
- Existing host/guest test infrastructure.
- Existing saved-state compatibility checks.
- Existing project documentation locations for public sandbox behavior.

## Risks & Mitigations

- Capacity mismatch risk: Host and guest consumers could observe different capacities. Mitigation: verify both consumers report the same configured values.
- Stale data risk: Restored executions could expose bytes from previous use. Mitigation: add restore tests that check every configured byte is cleared.
- Bounds safety risk: Reads or writes could access bytes outside the configured capacity. Mitigation: add zero, boundary, and oversized operation tests.
- Compatibility risk: Saved state could be applied to a sandbox with a different user data capacity. Mitigation: add compatibility checks and mismatch tests.

## References

- Input brief: User Data Region PRD supplied by the user.
