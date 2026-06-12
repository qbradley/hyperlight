# User Data Region Implementation Plan

## Overview

Implement an opt-in User Data Region as a fixed-capacity byte region associated with each sandbox. The architecture follows the existing configuration-derived memory layout flow: sandbox configuration provides sizes, memory layout computes host and guest addresses, the PEB advertises guest-visible regions, the memory manager owns bounded host access, and `MultiUseSandbox` exposes the public host API.

The supported exchange lifecycle is host write -> mutating/non-restoring guest execution -> host read before restore. Convenience execution paths that restore sandbox state before the host reads also clear user data, and this restore-clearing behavior is part of the documented contract.

The feature is local-only for this workflow. No branches will be pushed and no final PR will be created, per `.paw/work/user-data-region/WorkflowContext.md`.

## Current State Analysis

`SandboxConfiguration` already stores input/output data sizes, default/minimum constants, setters, crate-private getters, default construction, and tests (`.paw/work/user-data-region/CodeResearch.md:49-60`). `SandboxMemoryLayout` already stores input/output sizes, computes their scratch offsets and guest addresses, includes configured sizing fields in compatibility checks, and writes input/output region descriptors into the PEB (`.paw/work/user-data-region/CodeResearch.md:61-75`). Architecture-specific `min_scratch_size` functions currently account for input and output data sizes on amd64 and i686, while aarch64 exposes placeholder layout constants and an unimplemented minimum scratch function (`.paw/work/user-data-region/CodeResearch.md:77-88`).

The memory manager already performs bounded reads and writes through scratch-backed shared memory operations and zeroes or replaces scratch memory during restore before reinitializing scratch bookkeeping (`.paw/work/user-data-region/CodeResearch.md:90-113`). Public host APIs live on `MultiUseSandbox` and delegate lower-level memory operations to `SandboxMemoryManager` (`.paw/work/user-data-region/CodeResearch.md:114-125`). Guest code discovers shared regions through `HyperlightPEB`, and guest helper crates expose PEB-backed accessors for existing input/output/init-data flows (`.paw/work/user-data-region/CodeResearch.md:127-139`).

No existing user data configuration field, host API, or guest helper was found (`.paw/work/user-data-region/CodeResearch.md:207-211`).

## Design Alternatives Considered

### Alternative A: Single bidirectional User Data Region

Alternative A is the current plan: one configured user data capacity, one guest-visible region descriptor, and one shared byte region that both host and guest may read and write during the supported non-restoring exchange lifecycle. This aligns with the PRD's single "User Data region" model and reuses the existing pattern where scratch memory already contains host-addressable input/output buffers and PEB-advertised memory regions (`.paw/work/user-data-region/CodeResearch.md:61-75`, `.paw/work/user-data-region/CodeResearch.md:127-139`).

**Strengths:**
- Smallest configuration and layout surface: one size field, one layout offset, one PEB descriptor, and one compatibility value.
- Best fit for in-place transformation workflows where the host writes bytes, the guest mutates them, and the host reads the same region back.
- Lowest documentation and API complexity for embedders: one capacity to size, one region to discover, one lifecycle contract to understand.
- Preserves the first-release containment model: helper/API-level bounds without implying hardware-enforced directional permissions.

**Costs and risks:**
- Directional ownership is a convention rather than a type-level or API-level distinction; host and guest can both read/write the same bytes through official APIs.
- Protocol mistakes can overwrite input before output is consumed unless callers define their own in-region layout or sequencing.
- If future use cases need simultaneous independent request/response buffers, callers must partition the single region themselves or request a later API extension.

### Alternative B: Split directional regions

Alternative B would expose two configured regions: a host-write/guest-read region for input-like data and a guest-write/host-read region for output-like data. Both would still live in scratch and be advertised to the guest; under the current first-release containment model, directionality would be enforced by API/helper contract rather than by VM page permissions unless the design later adds separate mappings or guard behavior. The existing input/output scratch buffers already use directional naming and stack protocols for structured calls, so this alternative is conceptually familiar but would be a new raw-data pair rather than a replacement for those buffers (`.paw/work/user-data-region/CodeResearch.md:90-110`, `.paw/work/user-data-region/CodeResearch.md:195-204`).

**Strengths:**
- Clearer ownership model: host-authored input bytes and guest-authored output bytes are separate by construction.
- Lower risk of accidental in-place overwrite between request and response payloads.
- Easier to document for request/response workloads because each side has an obvious writable region and readable region.
- Leaves room for future stronger enforcement if the architecture ever introduces separate page permissions or mappings for directional buffers.

**Costs and risks:**
- Larger public surface: two size fields, two setters/getters, two PEB descriptors, two sets of guest helpers, two compatibility values, and more tests.
- Less natural for in-place transform workloads because the guest must copy results into a separate output region instead of mutating the input bytes directly.
- More layout and overflow cases: fixed-buffer sizing must account for input data, output data, host-to-guest user data, and guest-to-host user data before page-table alignment.
- Direction names may be confused with existing structured Input Data and Output Data buffers unless documentation and API names are very explicit.
- If directionality remains helper/API-level only, the split design may appear more isolated than it actually is; arbitrary guest pointer misuse would still not be VM-guarded.

### Comparison Summary

| Concern | Alternative A: Single bidirectional region | Alternative B: Split directional regions |
|---|---|---|
| Primary workflow | In-place host/guest byte transformation | Request/response byte exchange with separate ownership |
| Configuration | One capacity | Two capacities |
| PEB metadata | One additional `GuestMemoryRegion` | Two additional `GuestMemoryRegion` entries |
| Host API shape | One capacity plus read/write methods over one region | Separate host-write and host-read APIs, likely with separate capacities |
| Guest API shape | One discovered pointer/capacity or bounded view | Separate read-only-intent and write-only-intent helper surfaces |
| Bounds model | Helper/API-level bounds; no VM guard isolation | Same unless separate mappings/page permissions are added |
| Layout complexity | Adds one fixed buffer before page-table alignment | Adds two fixed buffers before page-table alignment |
| Compatibility surface | One capacity participates in snapshot compatibility | Two capacities participate in snapshot compatibility |
| Test burden | Single set of zero, exact, oversized, restore, and exchange tests | Same categories duplicated per direction plus cross-region non-overlap tests |
| Fit to current Spec/PRD | Direct fit | Requires revising the spec from one region to a directional pair |

### Current Planning Decision

The current implementation plan keeps Alternative A because it directly satisfies the approved spec's single-region requirements while minimizing new configuration, layout, PEB, API, and documentation surfaces. Alternative B is a viable design if the primary requirement changes from "shared in-place user data" to "directional raw request/response buffers"; adopting it would require revising `Spec.md`, Phase 1 layout/config work, Phase 2 host APIs, Phase 3 Rust/C guest helpers, and the documentation contract.

## Desired End State

Embedders can configure a zero-default user data capacity, observe the configured capacity from host, Rust guest, and C guest surfaces, write and read bounded byte slices through host APIs, and exchange bytes with guest code using the same guest-visible region during the supported non-restoring lifecycle. Restore clears the region, zero-capacity configurations preserve existing behavior, and snapshot compatibility rejects capacity mismatches.

Verification will combine targeted unit tests for configuration/layout/PEB structures, host API tests for bounds behavior and atomic oversized-write failure, integration tests with Rust and C test guests for host/guest byte exchange and restore clearing, capacity/overflow tests, fresh-zero tests, representative 4 KiB plus 1 byte, 64 KiB, and 1 MiB validation, formatting/linting, and existing project test commands discovered in CodeResearch.

## What We're NOT Doing

- Adding random-access offset-based host APIs.
- Replacing the existing structured call and return protocol.
- Changing convenience execution paths so guest user-data mutations survive an automatic restore.
- Persisting user data contents across restore.
- Adding general writable external mappings or new mapped-region behavior.
- Adding VM/page-level guard isolation around the user data region; bounds are enforced by host APIs and by guest helper contracts where the language surface supports it, while C guest callers remain responsible for honoring pointer-and-capacity metadata.
- Introducing a lower maximum user-data capacity than existing sandbox memory limits.
- Migrating downstream Orion/PageServer code.
- Creating or pushing a final PR from this workflow.

## Phase Status

- [x] **Phase 1: Layout and Metadata Plumbing** - Add user data capacity to configuration, memory layout, checked scratch sizing, compatibility checks, and PEB metadata.
- [x] **Phase 2: Host User Data API** - Add bounded memory-manager operations and public `MultiUseSandbox` capacity/read/write methods.
- [ ] **Phase 3: Guest Access and Lifecycle Tests** - Add Rust and C guest discovery helpers, test guest functions, integration tests, and restore compatibility coverage.
- [ ] **Phase 4: Documentation and Final Verification** - Add technical/project documentation and run final formatting, linting, build, and test commands.

## Phase Candidates

---

## Phase 1: Layout and Metadata Plumbing

### Dependencies:

- None. This phase establishes the configuration, layout, compatibility, and metadata foundations used by later phases.

### Changes Required:

- **`src/hyperlight_host/src/sandbox/config.rs`**: Add a zero-default `user_data_size` configuration field with public `set_user_data_size` and crate-private `get_user_data_size`, following the existing input/output configuration pattern (`.paw/work/user-data-region/CodeResearch.md:49-60`). Keep the floor at zero rather than applying input/output minimums, matching Spec FR-001. Ensure this internal capacity value can be surfaced by Phase 2's public host capacity reporting API.
- **`src/hyperlight_host/src/mem/layout.rs`**: Add `user_data_size` to `SandboxMemoryLayout`, debug output, constructor flow, compatibility checks, scratch minimum validation, and page-table base calculation. Add user data scratch offset and guest address helpers adjacent to input/output helpers, following existing layout organization (`.paw/work/user-data-region/CodeResearch.md:61-75`). Use checked addition/alignment for user-data-derived sizing and reject capacities that cannot fit within existing sandbox memory bounds.
- **`src/hyperlight_common/src/layout.rs` and architecture layout files**: Thread `user_data_size` through `min_scratch_size` and include it in fixed buffer sizing for amd64 and i686. Preserve aarch64's existing compile behavior while updating the function signature consistently (`.paw/work/user-data-region/CodeResearch.md:77-88`).
- **`src/hyperlight_common/src/mem.rs`**: Extend `HyperlightPEB` with a user data `GuestMemoryRegion` so guest code can discover the region through the existing metadata structure (`.paw/work/user-data-region/CodeResearch.md:127-131`). Add the field in an append-compatible position or otherwise preserve existing region offsets with static/round-trip tests so zero-capacity and mixed consumer behavior are explicit.
- **Tests**: Extend configuration tests in `config.rs`, layout compatibility tests in `layout.rs`, and PEB round-trip tests in `mem.rs` to cover default zero size, positive sizes, non-page-aligned capacity 4097 bytes, 64 KiB, 1 MiB, address/offset math, scratch minimum sizing, impossible capacity rejection, preserved existing PEB fields, and user-data capacity mismatch behavior (`.paw/work/user-data-region/CodeResearch.md:141-150`).

### Success Criteria:

#### Automated Verification:

- [ ] Config tests pass: `cargo test -p hyperlight-host sandbox::config::tests`
- [ ] Layout tests pass: `cargo test -p hyperlight-host mem::layout::tests`
- [ ] Common PEB tests pass: `cargo test -p hyperlight-common mem::tests`
- [ ] i686/common checks pass: `just check-i686`

#### Manual Verification:

- [ ] A zero-capacity configuration leaves the input/output offset relationship unchanged.
- [ ] A positive-capacity configuration places user data after output data and moves later scratch users only by the page-aligned fixed-buffer span.
- [ ] Layout compatibility compares user data capacity for both restore and snapshot reuse decisions and continues to ignore snapshot/PT size fields as before.
- [ ] PEB compatibility policy is explicit: new user-data metadata preserves existing field interpretation or intentionally documents required lockstep host/guest versions.

---

## Phase 2: Host User Data API

### Dependencies:

- Phase 1 complete. Host operations require the user data capacity field, layout offset helpers, scratch sizing, and compatibility metadata from Phase 1.

### Changes Required:

- **`src/hyperlight_host/src/mem/mgr.rs`**: Add bounded user data capacity/read/write operations on `SandboxMemoryManager<HostSharedMemory>` using the layout's user data scratch offset and configured capacity. Reads and writes operate from the beginning of the region; non-empty operations whose buffer length exceeds capacity return an error rather than clamping silently. Oversized writes must be atomic with respect to user data bytes and adjacent scratch state. Fresh scratch allocation/restore paths must leave positive-capacity user data bytes zeroed before first guest-visible use. Follow existing scratch memory copy patterns and error propagation from shared memory bounds checks (`.paw/work/user-data-region/CodeResearch.md:90-113`).
- **`src/hyperlight_host/src/sandbox/initialized_multi_use.rs`**: Add public `MultiUseSandbox` methods `user_data_size`, `write_user_data`, and `read_user_data`, delegating to the memory manager in the same style as existing public APIs delegate lower-level work (`.paw/work/user-data-region/CodeResearch.md:114-125`). Rustdoc must distinguish these APIs from init-data/user-memory helpers and state that guest mutations are readable only before restore.
- **`src/hyperlight_host/src/lib.rs`**: Confirm no additional re-export is required because `MultiUseSandbox` is already public; update only if rustdoc/API surface requires it (`.paw/work/user-data-region/CodeResearch.md:114-117`).
- **Tests**: Add host-side unit tests for public capacity reporting at zero, one-byte, 4097-byte, 64 KiB, and 1 MiB capacities; zero-length operations; fresh initial zero reads; exact-capacity writes; capacity-plus-one rejection; reads that exceed the configured region; and prefill -> capacity-plus-one write -> verify error plus unchanged in-region bytes and adjacent scratch state. Place tests with the host API or memory-manager tests, matching the existing patterns in `initialized_multi_use.rs`, `mgr.rs`, and `shared_mem.rs` (`.paw/work/user-data-region/CodeResearch.md:141-154`).

### Success Criteria:

#### Automated Verification:

- [ ] Host API tests pass: `cargo test -p hyperlight-host user_data`
- [ ] Memory manager tests pass: `cargo test -p hyperlight-host mem::mgr::tests`
- [ ] Formatting applies cleanly: `just fmt-apply`

#### Manual Verification:

- [ ] Public host capacity reporting returns the configured capacity for zero, one-byte, and multi-KiB configurations.
- [ ] Fresh positive-capacity user data reads as zero before first host write.
- [ ] Non-empty writes fail for zero-capacity sandboxes.
- [ ] Exact-capacity writes succeed.
- [ ] Capacity-plus-one writes fail without modifying existing in-region bytes or bytes outside the configured region.
- [ ] Public methods are documented with rustdoc comments describing bounds behavior.

---

## Phase 3: Guest Access and Lifecycle Tests

### Dependencies:

- Phase 1 complete for PEB user data metadata and layout compatibility.
- Phase 2 complete for public host capacity reporting and host read/write operations used by integration tests.

### Changes Required:

- **`src/hyperlight_guest/src/guest_handle/handle.rs` and `src/hyperlight_guest/src/guest_handle/host_comm.rs`**: Add Rust guest-side accessors that read user data region metadata from the stored PEB pointer, following existing PEB-backed helper patterns (`.paw/work/user-data-region/CodeResearch.md:127-139`). The Rust surface should expose capacity together with a bounded view or bounded copy helpers so safe callers cannot accidentally address beyond the advertised region; it does not create VM-level containment for arbitrary unsafe pointer misuse.
- **`src/hyperlight_guest_bin/src/host_comm.rs`**: Expose stable guest-bin convenience wrappers for user data pointer/capacity access so Rust guest binaries and tests use the same public guest-bin path as existing host communication helpers (`.paw/work/user-data-region/CodeResearch.md:136-138`).
- **`src/hyperlight_guest_capi/src/lib.rs`, `src/hyperlight_guest_capi/src/types.rs`, and generated headers from `src/hyperlight_guest_capi/cbindgen.toml`**: Add C guest API functions/types for discovering the user data pointer and capacity, following the C wrapper crate structure and generated-header workflow (`src/hyperlight_guest_capi/src/lib.rs:17-27`, `src/hyperlight_guest_capi/src/types.rs:17-24`, `src/hyperlight_guest_capi/cbindgen.toml:9`). Document that the C surface returns raw pointer-and-capacity metadata and that C callers must bound `memcpy` by the reported capacity.
- **`src/tests/rust_guests/simpleguest/src/main.rs`**: Add guest functions that read user data, transform user data in place, and report discovered capacity/address values, following existing simpleguest function registration patterns (`.paw/work/user-data-region/CodeResearch.md:151-153`).
- **`src/tests/c_guests/c_simpleguest/main.c`**: Add C guest functions that discover user data metadata and mutate user data in place, following existing C guest registration patterns (`.paw/work/user-data-region/CodeResearch.md:153`, `src/tests/c_guests/c_simpleguest/main.c:372-405`).
- **`src/hyperlight_host/tests/sandbox_host_tests.rs`**: Add integration tests using existing sandbox helpers to verify mutating/non-restoring host-to-guest-to-host round trips for zero, one, half-capacity, exact-capacity, 4097-byte, 64 KiB, and representative 1 MiB payloads for Rust and C guests. Add assertions that guest-reported capacity/address values and fresh guest-visible zero reads are automated, not manual-only (`.paw/work/user-data-region/CodeResearch.md:151-154`).
- **`src/hyperlight_host/src/sandbox/initialized_multi_use.rs` tests**: Add restore clearing, convenience execution path clearing, failed-call semantics documentation coverage if an existing failure test path is available, post-restore usability assertions that invoke an existing simple guest function successfully, and restore/snapshot-reuse capacity mismatch tests using existing snapshot/restore patterns (`.paw/work/user-data-region/CodeResearch.md:147-150`, `.paw/work/user-data-region/CodeResearch.md:180-193`).

### Success Criteria:

#### Automated Verification:

- [ ] Guest libraries build: `just guests`
- [ ] Host/guest user data tests pass: `cargo test -p hyperlight-host user_data --test sandbox_host_tests`
- [ ] Snapshot/restore targeted tests pass: `cargo test -p hyperlight-host restore_user_data`
- [ ] Capacity-scale benchmark or measurement command is recorded for 4 KiB plus 1 byte, 64 KiB, and 1 MiB cases; if no benchmark can be extended without new tooling, document the manual measurement and rationale in Docs.md.
- [ ] Debug test suite passes: `just test`

#### Manual Verification:

- [ ] Guest-discovered capacity equals the host-configured capacity for zero, one-byte, and multi-KiB configurations.
- [ ] Rust and C guest mutations are visible through host reads on the mutating/non-restoring execution path without using init-data reads.
- [ ] Restore and convenience execution paths that restore state return positive-capacity user data bytes to zero while leaving the sandbox usable.
- [ ] Tests and docs document that Rust bounded helpers and C pointer/capacity discovery are helper/API contracts, not VM guard isolation against arbitrary guest pointer misuse.

---

## Phase 4: Documentation and Final Verification

### Dependencies:

- Phases 1-3 complete so documentation reflects the final public API, guest behavior, lifecycle semantics, and test coverage.

### Changes Required:

- **`.paw/work/user-data-region/Docs.md`**: Create the PAW as-built technical reference using `paw-docs-guidance`, including configuration behavior, host API behavior, guest discovery behavior, restore semantics, helper/API-level containment, tests, and verification commands.
- **`docs/hyperlight-execution-details.md`**: Add public-facing documentation for configuring user data capacity, host read/write behavior, Rust bounded guest access, C pointer/capacity discovery, supported mutating/non-restoring exchange lifecycle, failed-call handoff expectations, restore clearing, fresh-zero behavior, 4097-byte/64 KiB/1 MiB capacity-scale guidance, and helper/API-level containment. Follow the plain Markdown documentation style found in CodeResearch (`.paw/work/user-data-region/CodeResearch.md:27-35`, `.paw/work/user-data-region/CodeResearch.md:195-205`).
- **`docs/README.md` and `README.md`**: Update documentation entry points so users can find the new user data documentation from the existing project docs index and root docs link (`.paw/work/user-data-region/CodeResearch.md:30`, `.paw/work/user-data-region/CodeResearch.md:195-205`).
- **`CHANGELOG.md`**: Add a public API entry for the user data region, following the observed Keep a Changelog format (`.paw/work/user-data-region/CodeResearch.md:34`).
- **Verification**: Run formatting, debug/release clippy, debug/release builds, guest build, debug/release tests, and CI-like test commands discovered in CodeResearch where the local environment supports them (`.paw/work/user-data-region/CodeResearch.md:36-45`).

### Success Criteria:

#### Automated Verification:

- [ ] Formatting applied: `just fmt-apply`
- [ ] Debug clippy passes: `just clippy debug`
- [ ] Release clippy passes: `just clippy release`
- [ ] Debug build passes: `just build`
- [ ] Release build passes: `just build release`
- [ ] Guest build passes: `just guests`
- [ ] Debug tests pass: `just test`
- [ ] Release tests pass: `just test release`
- [ ] CI-like debug tests pass if environment supports them: `just test-like-ci`
- [ ] CI-like release tests pass if environment supports them: `just test-like-ci release`

#### Manual Verification:

- [ ] Docs accurately describe default zero capacity, capacity bounds, fresh-zero behavior, Rust bounded guest access, C pointer/capacity discovery, mutating/non-restoring exchange lifecycle, failed-call handoff expectations, restore clearing, helper/API-level containment, generic compatibility diagnostics unless implementation supports field-specific errors, and 4097-byte/64 KiB/1 MiB capacity guidance.
- [ ] No final PR is created or pushed.
- [ ] Any verification command blocked by missing local hypervisor access is recorded with diagnostics for `/dev/kvm` and `/dev/mshv` access, per repository instructions.

---

## References

- Issue: none
- Spec: `.paw/work/user-data-region/Spec.md`
- Research: `.paw/work/user-data-region/CodeResearch.md`
