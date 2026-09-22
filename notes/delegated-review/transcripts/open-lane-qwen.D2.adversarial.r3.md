1. **kernel/src/user/install_service.rs**: The new `format_disk_description` function returns a `'static str`, but some of its branches return string literals (e.g., `" (already has nife installed)"`) while others do not. However, all returned values are indeed string literals with static lifetime, so this is actually okay. Still, it's brittle and could break if any branch were changed to return a dynamically constructed string.

2. **kernel/src/user/install_service.rs**: In `spawn_surveyor`, the code waits for two messages from the surveyor using `ipc_recv(report)` twice, but there is no timeout or error handling around these receives. If the surveyor hangs or crashes, the kernel will hang here indefinitely. This defeats the purpose of bounded waiting mentioned in the original design.

3. **kernel/src/user/install_service.rs**: The constant `SURVEYOR_BUDGET_PAGES` is set to 8, but there’s no explanation or justification for this value. Given that page table allocation can be critical for correct operation, this magic number should either be justified in comments or derived from actual requirements.

4. **kernel/src/user/install_service.rs**: The `create_roster_page` function fills a newly allocated page with zeros and then writes device info into it, but there's no verification that `block_roster::write` actually fits within the page bounds. While unlikely, if `block_devices()` returns more devices than fit in one page, this could lead to memory corruption.

5. **kernel/src/user/install_service.rs**: In `spawn_surveyor`, the surveyor is spawned with `SURVEYOR_ROLE_SURVEY` as `arg0`, but there's no explicit check that the surveyor binary actually supports this role. If the wrong program is passed as the surveyor, it might misbehave silently or crash.

6. **kernel/src/user/install_service.rs**: The `asked` function now takes a `disk_description: &str` parameter and prints it, but previously it had no such behavior. This changes the user interface subtly—users now see descriptions like "(already has nife installed)" even though logically, if nife is already installed, we shouldn’t reach this point due to earlier checks. The logic seems inconsistent with the documented behavior.

7. **kernel/src/user/install_service.rs**: Constants like `SURVEYOR_ROLE_SURVEY`, `SURVEYOR_SLOT_*`, etc., are defined locally in this file but must match those in `components/src/disk_surveyor.rs`. There's a comment acknowledging this, but no mechanism ensures consistency. A mismatch would cause runtime failures that are hard to debug.

NO CONCERNS