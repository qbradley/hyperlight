---
date: 2026-06-12T19:21:05.611+00:00
git_commit: 7f87d904075398da067551366d3afa32e1117f51
branch: feature/user-data-region
repository: hyperlight
topic: "User Data Region implementation research"
tags: [research, codebase, sandbox, memory-layout, guest-api]
status: complete
last_updated: 2026-06-12
---

# Research: User Data Region

## Research Question

Map the existing Hyperlight sandbox configuration, memory layout, scratch memory management, initialized sandbox API, guest discovery helpers, tests, documentation infrastructure, and verification commands relevant to implementing an opt-in User Data Region with host/guest exchange, bounds enforcement, restore clearing, and snapshot compatibility checks (`.paw/work/user-data-region/Spec.md:65-77`, `.paw/work/user-data-region/Spec.md:107-113`).

## Summary

- Sandbox configuration currently has host-configurable input and output data sizes with defaults of `0x4000`, minimums of `0x2000`, public setters, crate-private getters, and unit/property tests (`src/hyperlight_host/src/sandbox/config.rs:47-52`, `src/hyperlight_host/src/sandbox/config.rs:80-87`, `src/hyperlight_host/src/sandbox/config.rs:124-135`, `src/hyperlight_host/src/sandbox/config.rs:197-204`, `src/hyperlight_host/src/sandbox/config.rs:298-343`).
- Sandbox scratch layout places input data at scratch offset `0`, output data immediately after input data, computes guest virtual addresses from `scratch_base_gva`, and advertises input/output stacks through `HyperlightPEB` (`src/hyperlight_host/src/mem/layout.rs:430-453`, `src/hyperlight_host/src/mem/layout.rs:690-697`).
- Snapshot compatibility is checked through `SandboxMemoryLayout::is_compatible_with`, which includes input/output sizes, heap, code, init data, init-data permissions, and scratch size while excluding `snapshot_size` and `pt_size` (`src/hyperlight_host/src/mem/layout.rs:298-331`, `src/hyperlight_host/src/mem/layout.rs:829-870`).
- Restore validates snapshot compatibility before memory/vCPU mutation, then `restore_snapshot` zeroes or replaces scratch memory and reinitializes scratch bookkeeping and I/O stack pointers (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:501-519`, `src/hyperlight_host/src/mem/mgr.rs:525-572`, `src/hyperlight_host/src/mem/mgr.rs:607-617`).
- Guest code discovers input/output/init/heap regions through the PEB pointer stored in `GuestHandle`; guest input/output stack helpers read PEB `input_stack`/`output_stack`, and existing user-memory reads use PEB `init_data` (`src/hyperlight_guest/src/guest_handle/handle.rs:17-47`, `src/hyperlight_guest/src/guest_handle/io.rs:31-39`, `src/hyperlight_guest/src/guest_handle/io.rs:92-99`, `src/hyperlight_guest/src/guest_handle/host_comm.rs:38-59`).
- Not found: no dedicated host-side `user_data`, `UserData`, `userdata`, or public arbitrary scratch read/write API was found; searches matched existing init-data user-memory reading, guest allocator comments using “user data” as allocation payload terminology, and trace-gated internal guest-memory reading (`src/hyperlight_guest/src/guest_handle/host_comm.rs:38-59`, `src/hyperlight_guest_bin/src/memory.rs:84-87`, `src/hyperlight_host/src/mem/mgr.rs:755-861`).

## Documentation System

- **Framework**: Plain Markdown files were observed. No `mkdocs`, `docusaurus`, `sphinx`, or `mdbook` config was found by searching repository Markdown/TOML/YAML/JSON/JS/TS/Python/Justfile files for those framework names. The docs landing page is `docs/README.md` and links directly to Markdown files (`docs/README.md:1-6`, `docs/README.md:25-46`).
- **Docs Directory**: `docs/` contains project documentation, and the root README links to `docs/README.md` as the Docs entry (`README.md:115-120`, `docs/README.md:25-46`).
- **Navigation Config**: Not found. Navigation is represented by Markdown link lists in `docs/README.md`, and no separate docs navigation config was found by searching for `mkdocs`, `docusaurus`, `sphinx`, `mdbook`, `docsify`, `just.*docs`, `serve docs`, and `build docs` (`docs/README.md:25-46`).
- **Style Conventions**: Docs use H1 titles, H2 sections, bulleted link lists, fenced code blocks, and relative links (`docs/README.md:1-7`, `docs/README.md:25-46`, `docs/getting-started.md:1-24`, `docs/getting-started.md:51-78`, `docs/how-to-build-a-hyperlight-guest-binary.md:29-80`).
- **Build Command**: Not found. The Justfile exposes Rust doc tests via `test-doc`, but no static-site docs build or serve recipe was found (`Justfile:276-277`).
- **Standard Files**: Root `README.md` contains overview, examples, repository structure, and community/docs links (`README.md:1-17`, `README.md:84-101`, `README.md:111-120`). `CHANGELOG.md` follows Keep a Changelog (`CHANGELOG.md:1-7`). `CONTRIBUTING.md` describes PR workflow, docs updates, tests, DCO sign-off, and GPG signing (`CONTRIBUTING.md:20-35`, `CONTRIBUTING.md:42-100`). `SECURITY.md` and `SUPPORT.md` contain security reporting and support information (`SECURITY.md:1-21`, `SUPPORT.md:1-13`).

## Verification Commands

- **Build Command**: `just build` builds the host library in debug mode by default, and `just build release` uses the release profile; `just guests` builds and moves Rust and C guest binaries for debug and release (`Justfile:43-48`, `Justfile:69-70`).
- **Test Command**: `just test` runs unit, isolated, integration, and doc tests; `just test release` uses the release profile (`Justfile:218-239`, `Justfile:276-277`). CI-like test recipes include `just test-like-ci` and `just test-like-ci release` (`Justfile:86-108`).
- **Lint Command**: `just clippy debug`, `just clippy release`, `just clippy-guests`, and `just clippy-exhaustive` run clippy variants with `-D warnings` (`Justfile:316-347`). CI code checks run `just fmt-check`, clippy exhaustive checks on Linux, and `just clippy`/`just clippy-guests` on Windows (`Justfile:109-135`, `.github/workflows/dep_code_checks.yml:70-91`, `.github/workflows/dep_code_checks.yml:138-152`).
- **Formatting Command**: `just fmt-check` checks formatting with the configured nightly toolchain, and `just fmt-apply` applies formatting to the workspace, Rust guest workspace, and guest C API crate (`Justfile:299-314`).
- **Type/Feature Check**: `just check` runs cargo check for default and feature combinations including crashdump, print_debug, gdb, trace_guest/mem_profile, i686-guest, executable_heap, and hw-interrupts (`Justfile:289-297`). `just check-i686` checks i686-related common/guest builds (`Justfile:268-274`).
- **CI Aggregate**: `just like-ci` composes code checks, guest builds, build/test, examples, benchmarks, fuzz smoke tests, typos, and license header checks (`Justfile:189-216`).
- **Guest Build Details**: Rust guest builds use `cargo hyperlight build --workspace`; C guest builds depend on `build-rust-capi`, compile with `clang`, link with `ld.lld`, and move artifacts into `src/tests/c_guests/bin/<profile>/` (`Justfile:58-70`, `c.just:11-26`).
- **Docs Command**: No static docs build command was found; Rust doc tests use `just test-doc` (`Justfile:276-277`).

## Detailed Findings

### 1. Sandbox configuration

- `SandboxConfiguration` is a `#[repr(C)]` struct with input/output data sizes, heap override, interrupt settings, and scratch size fields (`src/hyperlight_host/src/sandbox/config.rs:32-77`).
- Input data size and output data size are stored as private `usize` fields, with doc comments describing buffers available to the guest binary (`src/hyperlight_host/src/sandbox/config.rs:47-52`).
- Input/output defaults are `DEFAULT_INPUT_SIZE = 0x4000` and `DEFAULT_OUTPUT_SIZE = 0x4000`; minimums are `MIN_INPUT_SIZE = 0x2000` and `MIN_OUTPUT_SIZE = 0x2000` (`src/hyperlight_host/src/sandbox/config.rs:80-87`).
- The private constructor clamps input and output data sizes to the configured minimums using `max`, and stores scratch size without clamping (`src/hyperlight_host/src/sandbox/config.rs:97-122`).
- Public setters `set_input_data_size` and `set_output_data_size` clamp values to the minimum sizes; crate-private getters return current input/output data sizes (`src/hyperlight_host/src/sandbox/config.rs:124-135`, `src/hyperlight_host/src/sandbox/config.rs:197-204`).
- `set_scratch_size` is public and assigns `scratch_size`; `get_scratch_size` is crate-private (`src/hyperlight_host/src/sandbox/config.rs:207-215`).
- `Default` constructs a configuration with default input/output sizes, default scratch size, default interrupt delay/offset, optional gdb state unset, and crashdump enabled when compiled (`src/hyperlight_host/src/sandbox/config.rs:244-259`).
- Unit tests cover override propagation and minimum-size clamping, and proptests cover input/output setters and getters for ranges from minimum through ten times minimum (`src/hyperlight_host/src/sandbox/config.rs:266-321`, `src/hyperlight_host/src/sandbox/config.rs:323-361`).
- Uninitialized sandbox construction accepts an optional `SandboxConfiguration`, defaults it when absent, and passes it into `Snapshot::from_env` before building from that snapshot (`src/hyperlight_host/src/sandbox/uninitialized.rs:391-420`).

### 2. Sandbox memory layout

- The module-level layout documentation describes the snapshot region and scratch region; input data and output data are shown at the bottom of scratch memory below general scratch memory and exception/metadata space (`src/hyperlight_host/src/mem/layout.rs:16-61`).
- `SandboxMemoryLayout` stores input/output data sizes, heap, code, init data, init-data permissions, scratch size, snapshot size, and page-table size (`src/hyperlight_host/src/mem/layout.rs:218-245`).
- The `Debug` implementation includes total memory, code, heap, init data, input data, output data, scratch, snapshot, PT, guest code, PEB, heap-buffer, and init-data offsets (`src/hyperlight_host/src/mem/layout.rs:247-295`).
- Not found: `SandboxMemoryLayout` does not derive or implement `PartialEq` in `src/hyperlight_host/src/mem/layout.rs`; compatibility is represented by `is_compatible_with` instead (`src/hyperlight_host/src/mem/layout.rs:218-219`, `src/hyperlight_host/src/mem/layout.rs:298-331`).
- `SandboxMemoryLayout::new` converts heap size from config, reads scratch size, rejects scratch larger than `MAX_MEMORY_SIZE`, reads input/output sizes from config, computes architecture-specific minimum scratch via `hyperlight_common::layout::min_scratch_size`, and rejects scratch smaller than the minimum (`src/hyperlight_host/src/mem/layout.rs:346-380`).
- The guest virtual address of output data is `scratch_base_gva(self.scratch_size) + input_data_size`; its host scratch offset is `input_data_size` (`src/hyperlight_host/src/mem/layout.rs:430-440`).
- The guest virtual address of input data is `scratch_base_gva(self.scratch_size)`; its host scratch offset is `0` (`src/hyperlight_host/src/mem/layout.rs:443-453`).
- Page-table scratch offset starts after `input_data_size + output_data_size` rounded up to the page size, and page-table GPA adds that offset to `scratch_base_gpa(self.scratch_size)` (`src/hyperlight_host/src/mem/layout.rs:456-470`).
- `set_pt_size` recomputes minimum fixed scratch from input/output sizes and adds the page-table byte size; it rejects layouts where scratch is too small for both fixed scratch and page tables (`src/hyperlight_host/src/mem/layout.rs:540-557`).
- `get_memory_regions_` emits page-aligned code, PEB, heap, and init-data regions and validates that computed offsets match expected PEB/heap/init/final offsets (`src/hyperlight_host/src/mem/layout.rs:571-670`).
- `write_peb` writes `HyperlightPEB` fields for `input_stack`, `output_stack`, `init_data`, and `guest_heap`; input/output stack sizes and pointers come from layout input/output sizes and GVA helpers (`src/hyperlight_host/src/mem/layout.rs:679-739`).
- `resolve_gpa` maps GPAs into scratch, snapshot, or memory-mapped regions; scratch GPAs are recognized from `scratch_base_gpa(self.scratch_size)` through `scratch_base + scratch_size` (`src/hyperlight_host/src/mem/layout.rs:742-771`).
- Layout tests cover computed memory size, maximum memory rejection, identical compatibility, ignoring snapshot/PT sizes for compatibility, and rejecting mutations to each configured field (`src/hyperlight_host/src/mem/layout.rs:800-870`).

### 3. Architecture layout functions

- `hyperlight_common::layout` selects architecture-specific layout files with `cfg_attr`: x86 uses `arch/i686/layout.rs`, x86_64 without `i686-guest` uses `arch/amd64/layout.rs`, x86_64 with `i686-guest` uses `arch/i686/layout.rs`, and aarch64 uses `arch/aarch64/layout.rs` (`src/hyperlight_common/src/layout.rs:17-27`).
- Common `scratch_base_gpa(size)` returns `MAX_GPA - size + 1`, and `scratch_base_gva(size)` returns `MAX_GVA - size + 1` (`src/hyperlight_common/src/layout.rs:52-57`).
- Common scratch-top bookkeeping offsets include size, allocator, snapshot page-table GPA base, snapshot generation, exception stack, and optional guest counter offsets (`src/hyperlight_common/src/layout.rs:36-50`).
- `min_scratch_size` is re-exported from the selected architecture module (`src/hyperlight_common/src/layout.rs:59-60`).
- amd64 layout sets `MAX_GVA` to `0xffff_ffff_ffff_efff`, snapshot page-table GVA bounds to `0xffff_8000_0000_0000..=0xffff_80ff_ffff_ffff`, `MAX_GPA` to `0x0000_000f_ffff_ffff`, and computes minimum scratch as page-aligned input+output plus twelve pages (`src/hyperlight_common/src/arch/amd64/layout.rs:17-44`).
- i686 layout sets `MAX_GVA` to `0xffff_ffff`, `MAX_GPA` below the KVM APIC access page at `0xFEDF_FFFF`, and computes minimum scratch as page-aligned input+output plus twelve pages (`src/hyperlight_common/src/arch/i686/layout.rs:17-30`).
- aarch64 layout contains placeholder constants copied from amd64 and an unimplemented `min_scratch_size` (`src/hyperlight_common/src/arch/aarch64/layout.rs:17-25`).
- Guest-side layout exposes scratch-top bookkeeping GVAs using common `MAX_GVA` and offsets, and re-exports arch-specific `scratch_base_gpa` and `scratch_base_gva` (`src/hyperlight_guest/src/layout.rs:17-39`).
- Guest amd64 reads scratch size from the scratch-top size GVA and derives guest scratch base GPA/GVA through `hyperlight_common::layout` helpers (`src/hyperlight_guest/src/arch/amd64/layout.rs:29-44`).
- Guest i686 currently returns a page-sized scratch size and derives scratch base GPA/GVA from common helpers; guest aarch64 scratch helpers are unimplemented (`src/hyperlight_guest/src/arch/i686/layout.rs:23-33`, `src/hyperlight_guest/src/arch/aarch64/layout.rs:21-31`).

### 4. Sandbox memory manager and shared memory read/write patterns

- `SandboxMemoryManager` owns snapshot/shared memory, scratch memory, memory layout, entrypoint, abort buffer, and snapshot generation count (`src/hyperlight_host/src/mem/mgr.rs:136-156`).
- `SandboxMemoryManager::new` stores the layout, shared memory, scratch memory, entrypoint, an empty abort buffer, and snapshot count zero (`src/hyperlight_host/src/mem/mgr.rs:271-291`).
- `SandboxMemoryManager::from_snapshot` clones layout and snapshot memory, allocates scratch memory sized by the snapshot layout, inherits the snapshot entrypoint, and inherits the snapshot generation count (`src/hyperlight_host/src/mem/mgr.rs:326-339`).
- `SandboxMemoryManager::build` converts exclusive shared/scratch memory to host and guest handles, creates host and guest managers with the same layout and entrypoint, then calls `update_scratch_bookkeeping` on the host manager (`src/hyperlight_host/src/mem/mgr.rs:341-376`).
- `update_scratch_bookkeeping` writes scratch size, first free scratch GPA, snapshot PT GPA base, and snapshot generation to scratch-top offsets (`src/hyperlight_host/src/mem/mgr.rs:582-605`).
- `update_scratch_bookkeeping` initializes input and output stack pointers by writing `SandboxMemoryLayout::STACK_POINTER_SIZE_BYTES` to the input/output scratch host offsets (`src/hyperlight_host/src/mem/mgr.rs:607-617`).
- `update_scratch_bookkeeping` copies page-table bytes from the end of shared memory into scratch at `layout.get_pt_base_scratch_offset()` (`src/hyperlight_host/src/mem/mgr.rs:618-639`).
- Host-to-guest guest function calls are serialized and written with `scratch_mem.push_buffer` at the input data offset with the input data capacity; guest function results, host function calls, and guest logs are read from the output data offset with output data capacity using `try_pop_buffer_into` (`src/hyperlight_host/src/mem/mgr.rs:442-493`, `src/hyperlight_host/src/mem/mgr.rs:495-502`).
- Host function responses are written to the input buffer via `push_buffer` with input data capacity (`src/hyperlight_host/src/mem/mgr.rs:451-465`).
- `clear_io_buffers` repeatedly pops and zeroes data from output and input buffers until popping fails (`src/hyperlight_host/src/mem/mgr.rs:504-523`).
- `restore_snapshot` replaces shared memory when snapshot memory differs, zeroes existing scratch memory when sizes match, allocates a new scratch mapping when the scratch size changes, then replaces layout and updates scratch bookkeeping (`src/hyperlight_host/src/mem/mgr.rs:525-572`).
- `SharedMemory::zero` uses `madvise(MADV_DONTNEED)` on Linux KVM when enabled and otherwise fills the exclusive memory slice with zero bytes (`src/hyperlight_host/src/mem/shared_mem.rs:498-521`).
- `ExclusiveSharedMemory::new` rejects zero-size shared memory, checked-adds two guard pages, requires page-size multiples, and returns `MemoryRequestTooBig` when total size exceeds `isize::MAX` (`src/hyperlight_host/src/mem/shared_mem.rs:560-590`, `src/hyperlight_host/src/mem/shared_mem.rs:655-684`).
- `bounds_check!` rejects offset+size combinations that overflow or exceed memory length and returns an error string with offset, size, and memory size (`src/hyperlight_host/src/mem/shared_mem.rs:49-60`).
- `ExclusiveSharedMemory::copy_from_slice` bounds-checks destination range and copies bytes into the mutable host slice (`src/hyperlight_host/src/mem/shared_mem.rs:840-847`).
- `HostSharedMemory::copy_to_slice` and `copy_from_slice` bounds-check ranges, use a lock, and perform volatile byte/u128 reads or writes (`src/hyperlight_host/src/mem/shared_mem.rs:1198-1247`, `src/hyperlight_host/src/mem/shared_mem.rs:1249-1297`).
- `HostSharedMemory::fill` bounds-checks the range and writes the fill byte with volatile byte/u128 operations (`src/hyperlight_host/src/mem/shared_mem.rs:1300-1346`).
- `push_buffer` reads the stack pointer, validates it is between 8 and buffer size, rejects insufficient capacity using `data.len() + 8`, writes bytes and a back pointer, then updates the stack pointer (`src/hyperlight_host/src/mem/shared_mem.rs:1348-1395`).
- `try_pop_buffer_into` validates stack pointer and back pointer, validates size-prefixed flatbuffer length against the element slot, copies bytes out, updates the stack pointer, and zeroes the popped bytes (`src/hyperlight_host/src/mem/shared_mem.rs:1397-1485`).
- Shared memory unit tests cover bounds-check overflow and copy/read/write edge cases, including offsets beyond memory, oversized reads/writes, and oversized buffers (`src/hyperlight_host/src/mem/shared_mem.rs:1874-1900`, `src/hyperlight_host/src/mem/shared_mem.rs:1902-1957`).
- A trace-gated internal method `read_guest_memory_by_gva` walks page tables from a GVA to GPA mappings, resolves each GPA into snapshot/scratch/mapped backing memory, copies page slices into a result, and errors when mappings are missing or incomplete (`src/hyperlight_host/src/mem/mgr.rs:755-861`).

### 5. Public initialized sandbox API

- Public host-side sandbox entry points are re-exported from `hyperlight_host::lib.rs`; `MultiUseSandbox`, `UninitializedSandbox`, `HostFunctions`, and `GuestBinary` are public re-exports (`src/hyperlight_host/src/lib.rs:24-25`, `src/hyperlight_host/src/lib.rs:89-98`).
- `MultiUseSandbox` stores `SandboxMemoryManager<HostSharedMemory>` as `mem_mgr` and a `HyperlightVm` as `vm` (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:81-96`).
- `MultiUseSandbox::from_uninit` is crate-visible through `pub(super)` and stores the host functions, memory manager, VM, optional debugger memory wrapper, no current snapshot, and no page-table root finder (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:110-133`).
- Public `MultiUseSandbox::snapshot` checks poison state, reuses an existing snapshot if present, collects mapped regions and page-table roots, then delegates to `self.mem_mgr.snapshot(...)` and stores the returned `Arc<Snapshot>` (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:309-392`).
- Public `MultiUseSandbox::restore` validates compatibility through the snapshot, delegates memory restoration to `self.mem_mgr.restore_snapshot`, updates VM mappings when returned, and resets vCPU state (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:394-530`).
- Public `MultiUseSandbox::call` clears cached snapshot state before mutating execution, delegates to `call_guest_function_by_name_no_reset`, and converts the return value into the requested supported return type (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:616-714`).
- `call_guest_function_by_name_no_reset` serializes a `FunctionCall`, writes it via `self.mem_mgr.write_guest_function_call`, dispatches into the VM with `self.vm.dispatch_call_from_host`, reads the result via `self.mem_mgr.get_guest_function_call_result`, clears abort bytes, and clears I/O buffers on error (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:870-950`).
- Public `call_guest_function_by_name` snapshots first, calls the mutating `call`, restores the saved snapshot, and returns the result (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:600-614`).
- Public `map_region` and `map_file_cow` live on `MultiUseSandbox`; `map_file_cow` validates overlap against the primary shared memory region and records metadata in the PEB under `nanvix-unstable` (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:716-747`, `src/hyperlight_host/src/sandbox/initialized_multi_use.rs:749-842`).
- Not found: no public `MultiUseSandbox` method dedicated to arbitrary host read/write of scratch memory or a user data region was found by searching initialized sandbox methods for read/write/user data terms; the matching host-side APIs were call/snapshot/restore/map functions and trace-gated internal read helpers (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:692-714`, `src/hyperlight_host/src/sandbox/initialized_multi_use.rs:716-842`, `src/hyperlight_host/src/mem/mgr.rs:755-861`).

### 6. Guest-side access and PEB structures

- `GuestMemoryRegion` is a C-compatible POD struct with `size: u64` and `ptr: u64` fields (`src/hyperlight_common/src/mem.rs:21-29`).
- `HyperlightPEB` contains `input_stack`, `output_stack`, `init_data`, and `guest_heap` memory regions, plus optional `file_mappings` under `nanvix-unstable` (`src/hyperlight_common/src/mem.rs:68-82`).
- The PEB round-trip test constructs a PEB, converts it to bytes and back with bytemuck, and asserts both struct and byte equality (`src/hyperlight_common/src/mem.rs:84-118`).
- `GuestHandle` stores an optional raw `*mut HyperlightPEB`, is initialized with a PEB pointer, and returns the PEB pointer via `peb()` (`src/hyperlight_guest/src/guest_handle/handle.rs:17-47`).
- Guest binary architecture-specific entrypoint receives `peb_address`, then pivots to `generic_init`; `generic_init` stores the PEB pointer in global `GUEST_HANDLE` and initializes the heap from PEB `guest_heap` (`src/hyperlight_guest_bin/src/arch/amd64/init.rs:129-158`, `src/hyperlight_guest_bin/src/lib.rs:222-247`).
- Guest-side input helper `try_pop_shared_input_data_into` reads PEB `input_stack.size` and `input_stack.ptr`, builds a mutable slice, validates stack pointer bounds, converts the top buffer to `T`, rewinds stack pointer, and zeroes popped bytes (`src/hyperlight_guest/src/guest_handle/io.rs:28-89`).
- Guest-side output helper `push_shared_output_data` reads PEB `output_stack.size` and `output_stack.ptr`, validates stack pointer and capacity, writes data and back pointer, and advances stack pointer (`src/hyperlight_guest/src/guest_handle/io.rs:91-149`).
- Guest-side user memory helper `read_n_bytes_from_user_memory` reads PEB `init_data.ptr` and `init_data.size`, rejects requests larger than `init_data.size`, and copies the requested bytes to a `Vec<u8>` (`src/hyperlight_guest/src/guest_handle/host_comm.rs:38-59`).
- `hyperlight_guest_bin::host_comm` exposes public wrappers that use global `GUEST_HANDLE` for host function calls, raw host return retrieval, and `read_n_bytes_from_user_memory` (`src/hyperlight_guest_bin/src/host_comm.rs:31-72`).
- Test guest `simpleguest` imports `read_n_bytes_from_user_memory` and exposes a `ReadFromUserMemory` guest function that reads bytes and compares them with an expected vector (`src/tests/rust_guests/simpleguest/src/main.rs:51-56`, `src/tests/rust_guests/simpleguest/src/main.rs:736-750`).
- Not found: no guest helper dedicated to writing an init/user memory region or discovering a separate user data region was found; searches for `read_user`, `write_user`, `user_data`, and `userdata` in guest crates matched only init-data reading and allocator payload comments (`src/hyperlight_guest/src/guest_handle/host_comm.rs:38-59`, `src/hyperlight_guest_bin/src/host_comm.rs:69-72`, `src/hyperlight_guest_bin/src/memory.rs:84-87`).

### 7. Tests and test patterns

- Sandbox configuration tests cover overrides, minimum input/output clamping, and property-based setter/getter behavior (`src/hyperlight_host/src/sandbox/config.rs:266-321`, `src/hyperlight_host/src/sandbox/config.rs:323-361`).
- Layout tests cover memory size, too-large scratch rejection, compatibility for identical layouts, ignoring snapshot/PT size, and rejecting each configured field mutation (`src/hyperlight_host/src/mem/layout.rs:800-870`).
- Shared memory tests cover overflow bounds checks and copy/read/write edge cases including too-large offsets and too-large buffers (`src/hyperlight_host/src/mem/shared_mem.rs:1874-1900`, `src/hyperlight_host/src/mem/shared_mem.rs:1902-1957`).
- Memory-manager tests verify page tables for configurations with default, small, medium, and large heaps, including a large scratch setting (`src/hyperlight_host/src/mem/mgr.rs:864-917`).
- `test_load_extra_blob` constructs a `GuestEnvironment` with init data, evolves a sandbox, calls `ReadFromUserMemory`, and asserts the returned bytes match the init buffer (`src/hyperlight_host/src/sandbox/uninitialized.rs:600-615`).
- `small_scratch_sandbox` configures input/output sizes of `0x24000` with scratch `0x48000` and asserts `MemoryRequestTooSmall` from `UninitializedSandbox::new` (`src/hyperlight_host/tests/sandbox_host_tests.rs:212-227`).
- `io_buffer_reset` configures 4096-byte input/output sizes, registers `HostAdd`, loops successful and failing guest calls, and exercises I/O buffer reset paths (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:1284-1305`).
- Snapshot/restore tests verify restore resets state after `call_guest_function_by_name`, restores evolved state, restores across sandboxes, rejects incompatible heap-size layouts, and leaves the target usable after restore rejection (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:1307-1331`, `src/hyperlight_host/src/sandbox/initialized_multi_use.rs:1377-1398`, `src/hyperlight_host/src/sandbox/initialized_multi_use.rs:1697-1775`).
- Host/guest exchange tests use `with_all_sandboxes` helpers to run both Rust and C guests for byte-array roundtrips, echo, size-prefixed buffers, and host print callbacks (`src/hyperlight_host/tests/common/mod.rs:112-155`, `src/hyperlight_host/tests/sandbox_host_tests.rs:32-45`, `src/hyperlight_host/tests/sandbox_host_tests.rs:239-267`).
- The Rust test guest defines `Echo`, `GetSizePrefixedBuffer`, `24K_in_8K_out`, `Add` through a host `HostAdd` function, and `ReadFromUserMemory` (`src/tests/rust_guests/simpleguest/src/main.rs:380-388`, `src/tests/rust_guests/simpleguest/src/main.rs:693-728`, `src/tests/rust_guests/simpleguest/src/main.rs:736-750`).
- The C test guest defines `echo`, byte-array zeroing, host-call examples, and registers `Echo`, `SetByteArrayToZero`, and `GetSizePrefixedBuffer` (`src/tests/c_guests/c_simpleguest/main.c:22-35`, `src/tests/c_guests/c_simpleguest/main.c:248-260`, `src/tests/c_guests/c_simpleguest/main.c:372-405`).
- Trace-gated tests compare host `read_guest_memory_by_gva` output with guest `ReadMappedBuffer` for single-page, full-page, cross-page, and multi-page reads (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:1985-2067`).

## Code References

- `src/hyperlight_host/src/sandbox/config.rs:32-77` - `SandboxConfiguration` fields including input/output/scratch settings.
- `src/hyperlight_host/src/sandbox/config.rs:80-95` - input/output/scratch default and minimum constants.
- `src/hyperlight_host/src/sandbox/config.rs:124-135` - public input/output size setters.
- `src/hyperlight_host/src/sandbox/config.rs:197-215` - crate-private input/output/scratch getters and public scratch setter.
- `src/hyperlight_host/src/sandbox/config.rs:244-259` - default configuration constructor.
- `src/hyperlight_host/src/mem/layout.rs:218-245` - `SandboxMemoryLayout` fields.
- `src/hyperlight_host/src/mem/layout.rs:298-331` - layout compatibility fields and excluded fields.
- `src/hyperlight_host/src/mem/layout.rs:346-380` - layout construction from configuration and scratch-size validation.
- `src/hyperlight_host/src/mem/layout.rs:430-453` - input/output GVA and host scratch offset helpers.
- `src/hyperlight_host/src/mem/layout.rs:540-557` - page-table size participates in minimum scratch validation.
- `src/hyperlight_host/src/mem/layout.rs:679-739` - PEB writing and region advertisement.
- `src/hyperlight_host/src/mem/layout.rs:742-771` - GPA resolution into scratch/snapshot/mapped regions.
- `src/hyperlight_common/src/layout.rs:17-60` - architecture layout module selection, scratch-top offsets, scratch base helpers, and `min_scratch_size` export.
- `src/hyperlight_common/src/arch/amd64/layout.rs:17-44` - amd64 scratch sizing and address bounds.
- `src/hyperlight_common/src/arch/i686/layout.rs:17-30` - i686 scratch sizing and address bounds.
- `src/hyperlight_common/src/arch/aarch64/layout.rs:17-25` - aarch64 placeholder layout and unimplemented minimum scratch sizing.
- `src/hyperlight_host/src/mem/mgr.rs:442-523` - memory-manager I/O buffer read/write/clear operations.
- `src/hyperlight_host/src/mem/mgr.rs:525-572` - memory-manager restore and scratch zero/replace behavior.
- `src/hyperlight_host/src/mem/mgr.rs:582-639` - scratch bookkeeping and page-table copy into scratch.
- `src/hyperlight_host/src/mem/shared_mem.rs:49-60` - shared-memory bounds-check macro.
- `src/hyperlight_host/src/mem/shared_mem.rs:1198-1346` - host shared-memory copy/fill methods.
- `src/hyperlight_host/src/mem/shared_mem.rs:1348-1485` - stack-buffer push/pop methods with capacity and corruption checks.
- `src/hyperlight_host/src/sandbox/initialized_multi_use.rs:309-392` - public snapshot API delegates to memory manager.
- `src/hyperlight_host/src/sandbox/initialized_multi_use.rs:394-530` - public restore API validates compatibility and delegates to memory manager/VM.
- `src/hyperlight_host/src/sandbox/initialized_multi_use.rs:692-714` - public call API.
- `src/hyperlight_host/src/sandbox/initialized_multi_use.rs:870-950` - internal call path writes guest function call and reads guest result through memory manager.
- `src/hyperlight_common/src/mem.rs:21-82` - `GuestMemoryRegion` and `HyperlightPEB`.
- `src/hyperlight_guest/src/guest_handle/handle.rs:17-47` - guest PEB pointer storage.
- `src/hyperlight_guest/src/guest_handle/io.rs:28-149` - guest input/output stack helpers.
- `src/hyperlight_guest/src/guest_handle/host_comm.rs:38-59` - guest init-data user-memory read helper.
- `src/hyperlight_guest_bin/src/host_comm.rs:31-72` - guest-bin public wrappers over `GUEST_HANDLE`.
- `src/hyperlight_guest_bin/src/lib.rs:222-247` - generic guest initialization stores PEB pointer and initializes heap.
- `src/hyperlight_host/src/sandbox/uninitialized.rs:600-615` - init-data host/guest exchange test.
- `src/hyperlight_host/src/sandbox/initialized_multi_use.rs:1697-1775` - snapshot/restore compatibility tests.
- `src/hyperlight_host/tests/common/mod.rs:112-155` - shared Rust/C sandbox test helpers.
- `src/tests/rust_guests/simpleguest/src/main.rs:736-750` - Rust guest reads PEB init-data bytes through helper.

## Architecture Documentation

- Configuration-derived memory layout is centralized: `SandboxConfiguration` stores sizes, `Snapshot::from_env` constructs `SandboxMemoryLayout` from the config, and `UninitializedSandbox::new` routes creation through `Snapshot::from_env` (`src/hyperlight_host/src/sandbox/config.rs:32-77`, `src/hyperlight_host/src/sandbox/snapshot/mod.rs:269-305`, `src/hyperlight_host/src/sandbox/uninitialized.rs:391-420`).
- Scratch memory has fixed subregions at its base for input and output buffers and fixed bookkeeping fields at the top of scratch; input/output locations are advertised through the PEB and initialized by host scratch bookkeeping (`src/hyperlight_host/src/mem/layout.rs:48-61`, `src/hyperlight_host/src/mem/layout.rs:430-453`, `src/hyperlight_host/src/mem/mgr.rs:582-617`, `src/hyperlight_host/src/mem/layout.rs:690-697`).
- Snapshot creation maps the scratch region through `map_specials`, appends page-table bytes to snapshot memory, records PT size, and stores only the guest-visible prefix as snapshot memory (`src/hyperlight_host/src/sandbox/snapshot/mod.rs:250-266`, `src/hyperlight_host/src/sandbox/snapshot/mod.rs:323-365`, `src/hyperlight_host/src/sandbox/snapshot/mod.rs:520-547`).
- Snapshot restore is compatibility-first: `MultiUseSandbox::restore` validates layout and host functions before delegating to memory restore and before updating VM mappings or vCPU state (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:501-530`, `src/hyperlight_host/src/sandbox/snapshot/mod.rs:658-676`).
- Restored scratch memory is transient: when scratch size is unchanged it is zeroed, and when scratch size differs it is replaced with a new zeroed `ExclusiveSharedMemory`; scratch bookkeeping then rewrites stack pointers and page-table bytes (`src/hyperlight_host/src/mem/mgr.rs:548-572`, `src/hyperlight_host/src/mem/mgr.rs:607-639`).
- Existing host/guest structured calls use stack-like buffers inside scratch with an 8-byte stack pointer at buffer start, data bytes, and an 8-byte back pointer per pushed item; host and guest both follow this format (`src/hyperlight_host/src/mem/shared_mem.rs:1348-1395`, `src/hyperlight_host/src/mem/shared_mem.rs:1397-1485`, `src/hyperlight_guest/src/guest_handle/io.rs:48-87`, `src/hyperlight_guest/src/guest_handle/io.rs:106-147`).
- Existing guest region discovery uses the PEB as the shared metadata format; PEB regions use `GuestMemoryRegion { size, ptr }`, and guest APIs read region descriptors from the PEB pointer captured at initialization (`src/hyperlight_common/src/mem.rs:21-29`, `src/hyperlight_common/src/mem.rs:68-82`, `src/hyperlight_guest/src/guest_handle/handle.rs:17-47`, `src/hyperlight_guest_bin/src/lib.rs:222-247`).
- Public host APIs are implemented on `MultiUseSandbox`, with low-level memory operations kept in `SandboxMemoryManager` and `SharedMemory` types (`src/hyperlight_host/src/lib.rs:89-98`, `src/hyperlight_host/src/sandbox/initialized_multi_use.rs:81-96`, `src/hyperlight_host/src/mem/mgr.rs:136-156`, `src/hyperlight_host/src/mem/shared_mem.rs:409-522`).
- Existing docs for public behavior are plain Markdown in `README.md` and `docs/`, including host/guest examples, snapshot/restore mention, and guest binary construction (`README.md:19-56`, `README.md:84-101`, `docs/getting-started.md:101-128`, `docs/how-to-build-a-hyperlight-guest-binary.md:29-90`).

## Open Questions

- Not found: a current host-side user data capacity field, default, getter, setter, or tests. Search terms included `user_data`, `UserData`, `userdata`, `user data`, `data region`, `read_user`, and `write_user`; matches were existing init-data user-memory reads, guest allocator comments, trace-gated internal memory reads, and hypervisor `UserMemory` terminology (`src/hyperlight_guest/src/guest_handle/host_comm.rs:38-59`, `src/hyperlight_guest_bin/src/memory.rs:84-87`, `src/hyperlight_host/src/mem/mgr.rs:755-861`, `src/hyperlight_host/src/hypervisor/virtual_machine/kvm/x86_64.rs:69-75`).
- Not found: a public `MultiUseSandbox` method for arbitrary host read/write of a configured scratch subregion. Searches of `src/hyperlight_host/src/sandbox` for public read/write methods found call/snapshot/restore/map APIs and internal/trace paths rather than a dedicated public raw region API (`src/hyperlight_host/src/sandbox/initialized_multi_use.rs:692-714`, `src/hyperlight_host/src/sandbox/initialized_multi_use.rs:716-842`, `src/hyperlight_host/src/mem/mgr.rs:755-861`).
- Not found: a guest helper for discovering or writing a separate user data region. Searches of guest crates for `user_data`, `userdata`, `read_user`, and `write_user` found only init-data reads and allocator comments (`src/hyperlight_guest/src/guest_handle/host_comm.rs:38-59`, `src/hyperlight_guest_bin/src/host_comm.rs:69-72`, `src/hyperlight_guest_bin/src/memory.rs:84-87`).
- Not found: a docs site navigation/config/build system beyond plain Markdown and Rust doc tests. Searches for `mkdocs`, `docusaurus`, `sphinx`, `mdbook`, docs build, and docs serve commands found no static-site config or recipe; docs are linked through Markdown (`docs/README.md:25-46`, `Justfile:276-277`).
