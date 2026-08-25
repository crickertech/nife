use super::*;
use crate::cap::{Rights, memory_region_cap, rendezvous_cap};
use crate::sched::RendezvousId;

const ROLE_MAKER: u64 = 17;
const ROLE_USER: u64 = 18;

/// Spawn the pair; returns the report endpoint carrying the word that crossed the minted
/// endpoint.
pub fn wire(image: &'static [u8]) -> RendezvousId {
    let channel = crate::sched::create_rendezvous();
    let report = crate::sched::create_rendezvous();
    let region = crate::memory_region::create(4).expect("no region for the maker's budget");

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: ROLE_MAKER,
                arg1: 0,
                arg2: 0,
                grants: &[
                    memory_region_cap(region),              // slot 0: the budget to mint from
                    rendezvous_cap(channel, Rights::WRITE), // slot 1: delegate the mint here
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the endpoint maker");

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: ROLE_USER,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(channel, Rights::READ), // slot 0: receive the delegation
                    rendezvous_cap(report, Rights::WRITE), // slot 1: report the word
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the endpoint user");

    report
}
