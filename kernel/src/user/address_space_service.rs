use super::*;
use crate::cap::{Rights, memory_region_cap, rendezvous_cap};
use crate::sched::RendezvousId;

const ROLE_BUILDER: u64 = 19;

/// Spawn the builder; returns the report endpoint carrying its verdict bits.
pub fn wire(image: &'static [u8]) -> RendezvousId {
    let report = crate::sched::create_rendezvous();
    let region = crate::memory_region::create(8).expect("no region for the builder");

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: ROLE_BUILDER,
                arg1: 0,
                arg2: 0,
                grants: &[
                    memory_region_cap(region),             // slot 0: the budget
                    rendezvous_cap(report, Rights::WRITE), // slot 1: the verdict
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the address space builder");

    report
}
