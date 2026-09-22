1. **kernel/src/user/install_service.rs: Duplicated constants**  
The surveyor constants (like `SURVEYOR_ROLE_SURVEY`, `SURVEYOR_SLOT_*`, etc.) are manually defined in this file, duplicating values that should ideally be imported or shared from `components/src/disk_surveyor.rs`. This introduces a maintenance hazard and potential inconsistency if the source of truth changes.

2. **kernel/src/user/install_service.rs: Hardcoded stack/budget sizes**  
Constants like `SURVEYOR_STACK_PAGES` and `SURVEYOR_BUDGET_PAGES` are hardcoded without explanation or reference to how these values were determined. If these are insufficient or excessive under certain conditions, it could lead to runtime failures or wasted resources.

3. **kernel/src/user/install_service.rs: Unhandled timeout case in spawn_surveyor**  
In `spawn_surveyor`, the code assumes that waiting for IPC messages via `ipc_recv` will always succeed. However, there’s no handling for timeouts or message delivery failure, which could cause indefinite blocking or panic if the surveyor misbehaves or crashes.

4. **kernel/src/user/install_service.rs: Silent fallback on surveyor failure**  
If spawning the `disk_surveyor` fails or times out, the user sees a vague message like "(survey timed out or failed)" but doesn't get more diagnostic detail. This reduces debuggability during install issues.

5. **kernel/src/user/install_service.rs: Lack of error propagation from surveyor**  
The `spawn_surveyor` function returns an `Option<(...)>` but does not propagate errors that might help diagnose why the surveyor failed. This makes debugging harder since internal faults are masked as generic failures.

6. **kernel/src/user/install_service.rs: Unsafe usage without sufficient guards**  
Usage of `unsafe` in `create_roster_page()` relies on assumptions about memory ownership and mapping correctness. While documented with a safety comment, such critical unsafe blocks require strong justification and ideally additional assertions or compile-time checks to prevent UB.

7. **kernel/src/user/install_service.rs: Inconsistent naming/convention with legacy survey logic**  
This new implementation replaces a prior method using a ROLE_SURVEY-based installer call but adds complexity by introducing a separate `disk_surveyor` component. It's unclear if this improves modularity or increases dependencies unnecessarily, especially given existing bugs mentioning lack of reinstall support.

8. **kernel/src/user/install_service.rs: Potential resource leak in spawn_surveyor**  
Allocated frames for stack and roster page are not explicitly reclaimed even though they’re only needed temporarily for the surveyor execution. Given the kernel context, this may represent a minor resource leak depending on allocator behavior post-spawn.

9. **kernel/src/user/install_service.rs: Missing retry or recovery mechanism for surveyor**  
There is no attempt to retry or recover if the surveyor fails, potentially leaving users unable to proceed with installation unless they understand to reboot or take manual action.

10. **kernel/src/user/install_service.rs: Redundant block device listing logic**  
The `block_devices()` function duplicates knowledge about supported transports (`TRANSPORT_MMIO`, `TRANSPORT_PCI`). This logic appears tightly coupled with lower-level drivers and risks becoming inconsistent if those evolve independently.