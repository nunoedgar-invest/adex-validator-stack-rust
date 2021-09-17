use primitives::{spender::Spendable, Address, ChannelId};

use super::{DbPool, PoolError};

/// ```text
/// INSERT INTO spendable (spender, channel_id, total, still_on_create2)
/// values ('0xce07CbB7e054514D590a0262C93070D838bFBA2e', '0x061d5e2a67d0a9a10f1c732bca12a676d83f79663a396f7d87b3e30b9b411088', 10.00000000, 2.00000000);
/// ```
pub async fn insert_spendable(pool: DbPool, spendable: &Spendable) -> Result<bool, PoolError> {
    let client = pool.get().await?;
    let stmt = client.prepare("INSERT INTO spendable (spender, channel_id, total, still_on_create2) values ($1, $2, $3, $4)").await?;

    let row = client
        .execute(
            &stmt,
            &[
                &spendable.spender,
                &spendable.channel.id(),
                &spendable.deposit.total,
                &spendable.deposit.still_on_create2,
            ],
        )
        .await?;

    let is_inserted = row == 1;
    Ok(is_inserted)
}

/// ```text
/// SELECT spender, total, still_on_create2, channels.leader, channels.follower, channels.guardian, channels.token, channels.nonce FROM spendable INNER JOIN channels ON channels.id = spendable.channel_id WHERE spender = $1 AND channel_id = $2
/// ```
pub async fn fetch_spendable(
    pool: DbPool,
    spender: &Address,
    channel_id: &ChannelId,
) -> Result<Option<Spendable>, PoolError> {
    let client = pool.get().await?;
    let statement = client.prepare("SELECT spender, total, still_on_create2, channels.leader, channels.follower, channels.guardian, channels.token, channels.nonce FROM spendable INNER JOIN channels ON channels.id = spendable.channel_id WHERE spender = $1 AND channel_id = $2").await?;
    
    let row = client.query_opt(&statement, &[spender, channel_id]).await?;

    Ok(row.as_ref().map(Spendable::from))
}

static UPDATE_SPENDABLE_STATEMENT: &str = "WITH inserted_spendable AS (INSERT INTO spendable(spender, channel_id, total, still_on_create2) VALUES($1, $2, $3, $4) ON CONFLICT ON CONSTRAINT spendable_pkey DO UPDATE SET total = $3, still_on_create2 = $4 WHERE spendable.spender = $1 AND spendable.channel_id = $2 RETURNING *) SELECT inserted_spendable.*, channels.leader, channels.follower, channels.guardian, channels.token, channels.nonce FROM inserted_spendable INNER JOIN channels ON inserted_spendable.channel_id = channels.id";

// Updates spendable entry deposit or inserts a new spendable entry if it doesn't exist
pub async fn update_spendable(pool: DbPool, spendable: &Spendable) -> Result<Spendable, PoolError> {
    let client = pool.get().await?;
    let statement = client.prepare(UPDATE_SPENDABLE_STATEMENT).await?;

    let row = client
        .query_one(
            &statement,
            &[
                &spendable.spender,
                &spendable.channel.id(),
                &spendable.deposit.total,
                &spendable.deposit.still_on_create2,
            ],
        )
        .await?;


    Ok(Spendable::from(&row))
}

#[cfg(test)]
mod test {
    use primitives::{
        Deposit,
        spender::Spendable,
        util::tests::prep_db::{ADDRESSES, DUMMY_CAMPAIGN},
        UnifiedNum,
    };

    use crate::db::{insert_channel, tests_postgres::{setup_test_migrations, DATABASE_POOL}};

    use super::*;

    #[tokio::test]
    async fn it_inserts_and_fetches_and_updates_spendable() {
        let database = DATABASE_POOL.get().await.expect("Should get a DB pool");

        setup_test_migrations(database.pool.clone())
            .await
            .expect("Migrations should succeed");

        let spendable = Spendable {
            spender: ADDRESSES["user"],
            channel: DUMMY_CAMPAIGN.channel.clone(),
            deposit: Deposit {
                total: UnifiedNum::from(100_000_000),
                still_on_create2: UnifiedNum::from(500_000),
            },
        };

        insert_channel(&database.pool, spendable.channel).await.expect("Should insert Channel before creating spendable");

        let is_inserted = insert_spendable(database.pool.clone(), &spendable)
            .await
            .expect("Should succeed");

        assert!(is_inserted);

        let fetched_spendable = fetch_spendable(
            database.pool.clone(),
            &spendable.spender,
            &spendable.channel.id(),
        )
        .await
        .expect("Should fetch successfully");

        assert_eq!(Some(spendable), fetched_spendable);

        // TODO: Update spendable
    }
}
