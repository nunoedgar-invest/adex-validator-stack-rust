use std::error::Error;

use primitives::adapter::{Adapter, AdapterErrorKind};
use primitives::balances::UncheckedState;
use primitives::{
    sentry::AccountingResponse,
    validator::{MessageTypes, NewState},
    BalancesMap,
};

use crate::get_state_root_hash;
use crate::heartbeat::{heartbeat, HeartbeatStatus};
use crate::sentry_interface::{PropagationResult, SentryApi};

#[derive(Debug)]
pub struct TickStatus<AE: AdapterErrorKind + 'static> {
    pub heartbeat: HeartbeatStatus<AE>,
    /// If None, then the conditions for handling a new state haven't been met
    pub new_state: Option<Vec<PropagationResult>>,
}

pub async fn tick<A: Adapter + 'static>(
    iface: &SentryApi<A>,
) -> Result<TickStatus<A::AdapterError>, Box<dyn Error>> {
    // 1. Get Accounting
    // 2. Check if Accounting != than latest NewState
    // 3. create a NewState
    let new_state = None;

    Ok(TickStatus {
        heartbeat: heartbeat(iface).await?,
        new_state,
    })
}

async fn _on_new_accounting<A: Adapter + 'static>(
    iface: &SentryApi<A>,
    new_accounting: &AccountingResponse<UncheckedState>,
) -> Result<Vec<PropagationResult>, Box<dyn Error>> {
    let state_root_raw = get_state_root_hash(iface, &BalancesMap::default())?;
    let state_root = hex::encode(state_root_raw);

    let signature = iface.adapter.sign(&state_root)?;

    let propagation_results = iface
        .propagate(&[&MessageTypes::NewState(NewState {
            state_root,
            signature,
            balances: new_accounting.balances.clone(),
        })])
        .await;

    Ok(propagation_results)
}
