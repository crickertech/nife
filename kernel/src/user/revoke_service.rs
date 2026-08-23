use super::*;
use crate::cap::{Rights, rendezvous_cap, untyped_cap};
use crate::sched::RendezvousId;

const ROLE_REVOKE_DEMO: u64 = 16;

/// Spawn the demo with an 8-page untyped budget; returns the endpoint it reports its verdict on.
pub fn wire(image: &'static [u8]) -> RendezvousId {
    let region = crate::untyped::create(8).expect("no untyped for the revoke demo");
    let report = crate::sched::create_rendezvous();
    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: ROLE_REVOKE_DEMO,
                arg1: 0,
                arg2: 0,
                grants: &[
                    untyped_cap(region),                   // slot 0: retype + page tables
                    rendezvous_cap(report, Rights::WRITE), // slot 1: report the verdict
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the revoke demo");
    report
}
