use super::*;
use crate::cap::{Rights, rendezvous_cap};
use crate::sched::RendezvousId;

const ROLE_SERVER: u64 = 14;
const ROLE_CLIENT: u64 = 15;

/// Spawn the pair, sharing one request endpoint. Returns `(client reply report, server one-shot
/// report)`: the client publishes the reply it got, the server publishes whether a second reply
/// was refused.
pub fn wire(image: &'static [u8]) -> (RendezvousId, RendezvousId) {
    let ep = crate::sched::create_rendezvous(); // client CALL <-> server RECV_CAP
    let call_report = crate::sched::create_rendezvous();
    let oneshot_report = crate::sched::create_rendezvous();

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: ROLE_SERVER,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(ep, Rights::READ),              // slot 0: RECV calls
                    rendezvous_cap(oneshot_report, Rights::WRITE), // slot 1: report the verdict
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the call server");

    crate::sched::spawn(move || {
        run(
            image,
            Spawn {
                arg0: ROLE_CLIENT,
                arg1: 0,
                arg2: 0,
                grants: &[
                    rendezvous_cap(ep, Rights::WRITE),          // slot 0: CALL
                    rendezvous_cap(call_report, Rights::WRITE), // slot 1: report the reply
                ],
                maps: &[],
            },
        )
    })
    .expect("could not spawn the call client");

    (call_report, oneshot_report)
}
