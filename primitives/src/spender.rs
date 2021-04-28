use crate::{channel_v5::Channel, Address, BalancesMap, UnifiedNum};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Deposit {
    pub total: UnifiedNum,
    pub still_on_create2: UnifiedNum,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Spendable {
    pub spender: Address,
    pub channel: Channel,
    #[serde(flatten)]
    pub deposit: Deposit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Aggregate {
    pub spender: Address,
    pub channel: Channel,
    pub balances: BalancesMap,
    pub created: DateTime<Utc>,
}
#[cfg(feature = "postgres")]
mod postgres {
    use std::convert::TryFrom;
    use tokio_postgres::{Error, Row};

    use super::*;

    impl TryFrom<Row> for Spendable {
        type Error = Error;

        fn try_from(row: Row) -> Result<Self, Self::Error> {
            Ok(Spendable {
                spender: row.try_get("spender")?,
                channel: row.try_get("channel")?,
                deposit: Deposit {
                    total: row.try_get("total")?,
                    still_on_create2: row.try_get("still_on_create2")?,
                },
            })
        }
    }
}