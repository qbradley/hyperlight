# User Data Region

## Overview

The User Data Region adds an optional, fixed-capacity shared byte buffer to a sandbox. Embedders configure the capacity at sandbox construction time; the host can copy bytes into or out of the region, and guest code can discover the same region through the PEB.

The region is intended for transient host/guest payload exchange. It lives in scratch memory, so it is not captured in snapshots and is cleared by restore. A capacity of `0` is the default and preserves existing behavior.

## Architecture and Design

### High-Level Architecture

The feature follows the existing configuration-to-layout-to-PEB flow:

- `SandboxConfiguration` stores the configured user data capacity.
- `SandboxMemoryLayout` places the region after the input and output data buffers and before the page-table copy area in scratch.
- `HyperlightPEB` advertises the region to guest code as a `GuestMemoryRegion`.
- `SandboxMemoryManager` performs bounded host-side copies against scratch memory.
- `MultiUseSandbox` exposes the public host APIs.
- Rust and C guest surfaces expose the PEB-provided pointer and capacity.

### Design Decisions

The implementation uses one bidirectional region rather than separate host-write and guest-write regions. This matches the in-place transformation use case: the host writes bytes, the guest mutates them, and the host reads the same region back.

Bounds are enforced by the official host APIs and Rust guest helpers. The C guest API exposes pointer-and-capacity metadata; C callers must use the reported capacity when copying bytes. The region is not separated by VM guard pages, so arbitrary guest pointer misuse is not hardware-confined to the region.

The supported exchange lifecycle is host write, mutating/non-restoring guest call, then host read. Convenience call paths that restore sandbox state before the host reads clear the region as part of restore.

### Integration Points

The region participates in scratch sizing and layout compatibility. Snapshots cannot be restored into a sandbox with a different user data capacity; capacity mismatches use the existing layout-mismatch diagnostic rather than a field-specific user data error. `MultiUseSandbox::from_snapshot` uses the snapshot layout's user data capacity, even if the caller supplies a different capacity.

## User Guide

### Prerequisites

Configure a positive user data capacity before creating or evolving the sandbox. Capacity is bounded by the existing sandbox memory limits and scratch sizing rules.

### Basic Usage

On the host, configure the region, write bytes with `write_user_data`, execute a mutating guest call, and read bytes with `read_user_data`.

Guest code can discover the capacity and pointer through Rust guest helpers or C API functions. Rust helpers also provide bounded read/write helpers.

### Advanced Usage

Use the region for large transient payloads that do not need FlatBuffer serialization. For request/response protocols with multiple logical fields, define an application-level layout inside the region or reserve fixed offsets within the configured capacity.

The tested representative capacities are 4097 bytes, 64 KiB, and 1 MiB. These cover non-page-aligned sizing, medium payloads, and MiB-scale exchange. Larger capacities are allowed when the sandbox has sufficient scratch memory, but applications should measure copy and restore behavior for their workload.

## API Reference

### Host APIs

- `SandboxConfiguration::set_user_data_size(size)` configures capacity.
- `MultiUseSandbox::user_data_size()` reports capacity.
- `MultiUseSandbox::write_user_data(data)` writes from region offset zero and fails if `data.len()` exceeds capacity.
- `MultiUseSandbox::read_user_data(out)` reads from region offset zero and fails if `out.len()` exceeds capacity.

### Guest APIs

Rust guest surfaces expose capacity, pointer, bounded read, and bounded write helpers. C guest surfaces expose `hl_user_data_size()` and `hl_user_data_ptr()`; C callers must bound memory access themselves.

## Testing

### How to Test

Build guests first, then run the targeted user data tests:

```bash
cargo test -p hyperlight-host --lib user_data
cargo test -p hyperlight-host user_data --test sandbox_host_tests
cargo test -p hyperlight-host --lib restore_user_data
```

The integration tests exercise Rust and C guests, fresh zero reads, capacity discovery, host-to-guest-to-host mutations, restore clearing, convenience restore clearing, layout mismatch rejection, and snapshot construction from saved layouts.

### Edge Cases

- Zero capacity supports empty reads and writes only.
- Oversized host writes fail before modifying the region.
- Fresh positive-capacity regions read as zero.
- Restore clears the region and leaves the sandbox usable.
- Snapshot compatibility includes user data capacity.
- Failed guest execution does not define a successful handoff; callers should treat region contents as application-defined unless the guest completed the expected exchange protocol.

## Limitations and Future Work

The first release does not provide random-access host APIs, separate directional regions, VM guard isolation around the region, or persistence across restore. Benchmark guidance is based on representative functional sizes; workloads using larger regions should measure their own copy and restore costs.
