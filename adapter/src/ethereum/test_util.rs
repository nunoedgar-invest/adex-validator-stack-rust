use once_cell::sync::Lazy;
use std::{collections::HashMap, convert::TryFrom, num::NonZeroU8};
use web3::{
    contract::{Contract, Options},
    ethabi::Token,
    transports::Http,
    types::{H160, H256, U256},
    Web3,
};

use primitives::{
    adapter::KeystoreOptions,
    channel::{Channel, Nonce},
    config::TokenInfo,
    Address, BigNum, Config, ValidatorId,
};

use crate::EthereumAdapter;

use super::{EthereumChannel, OUTPACE_ABI, SWEEPER_ABI};

// See `adex-eth-protocol` `contracts/mocks/Token.sol`
/// Mocked Token ABI
pub static MOCK_TOKEN_ABI: Lazy<&'static [u8]> =
    Lazy::new(|| include_bytes!("../../test/resources/mock_token_abi.json"));
/// Mocked Token bytecode in JSON
pub static MOCK_TOKEN_BYTECODE: Lazy<&'static str> =
    Lazy::new(|| include_str!("../../test/resources/mock_token_bytecode.bin"));
/// Sweeper bytecode
pub static SWEEPER_BYTECODE: Lazy<&'static str> =
    Lazy::new(|| include_str!("../../../lib/protocol-eth/resources/bytecode/Sweeper.bin"));
/// Outpace bytecode
pub static OUTPACE_BYTECODE: Lazy<&'static str> =
    Lazy::new(|| include_str!("../../../lib/protocol-eth/resources/bytecode/OUTPACE.bin"));

pub static KEYSTORE_IDENTITY: Lazy<(Address, KeystoreOptions)> = Lazy::new(|| {
    (
        // The address of the keystore file in `adapter/test/resources/keystore.json`
        Address::try_from("0x2bDeAFAE53940669DaA6F519373f686c1f3d3393")
            .expect("failed to parse id"),
        KeystoreOptions {
            keystore_file: "./test/resources/keystore.json".to_string(),
            keystore_pwd: "adexvalidator".to_string(),
        },
    )
});

/// Addresses generated on local running `ganache` for testing purposes.
/// see the `ganache-cli.sh` script in the repository
pub static GANACHE_ADDRESSES: Lazy<HashMap<String, Address>> = Lazy::new(|| {
    vec![
        (
            "leader".to_string(),
            "0x5a04A8fB90242fB7E1db7d1F51e268A03b7f93A5"
                .parse()
                .expect("Valid Address"),
        ),
        (
            "follower".to_string(),
            "0xe3896ebd3F32092AFC7D27e9ef7b67E26C49fB02"
                .parse()
                .expect("Valid Address"),
        ),
        (
            "creator".to_string(),
            "0x0E45891a570Af9e5A962F181C219468A6C9EB4e1"
                .parse()
                .expect("Valid Address"),
        ),
        (
            "advertiser".to_string(),
            "0x8c4B95383a46D30F056aCe085D8f453fCF4Ed66d"
                .parse()
                .expect("Valid Address"),
        ),
    ]
    .into_iter()
    .collect()
});
/// Local `ganache` is running at:
pub const GANACHE_URL: &'static str = "http://localhost:8545";

pub fn get_test_channel(token_address: Address) -> Channel {
    Channel {
        leader: ValidatorId::from(&GANACHE_ADDRESSES["leader"]),
        follower: ValidatorId::from(&GANACHE_ADDRESSES["follower"]),
        guardian: GANACHE_ADDRESSES["advertiser"],
        token: token_address,
        nonce: Nonce::from(12345_u32),
    }
}

pub fn setup_eth_adapter(config: Config) -> EthereumAdapter {
    let keystore_options = KeystoreOptions {
        keystore_file: "./test/resources/keystore.json".to_string(),
        keystore_pwd: "adexvalidator".to_string(),
    };

    EthereumAdapter::init(keystore_options, &config).expect("should init ethereum adapter")
}

pub async fn mock_set_balance(
    token_contract: &Contract<Http>,
    from: [u8; 20],
    address: [u8; 20],
    amount: u64,
) -> web3::contract::Result<H256> {
    token_contract
        .call(
            "setBalanceTo",
            (H160(address), U256::from(amount)),
            H160(from),
            Options::default(),
        )
        .await
}

pub async fn outpace_deposit(
    outpace_contract: &Contract<Http>,
    channel: &Channel,
    to: [u8; 20],
    amount: u64,
) -> web3::contract::Result<H256> {
    outpace_contract
        .call(
            "deposit",
            (channel.tokenize(), H160(to), U256::from(amount)),
            H160(to),
            Options::with(|opt| {
                opt.gas_price = Some(1.into());
                opt.gas = Some(6_721_975.into());
            }),
        )
        .await
}

pub async fn sweeper_sweep(
    sweeper_contract: &Contract<Http>,
    outpace_address: [u8; 20],
    channel: &Channel,
    depositor: [u8; 20],
) -> web3::contract::Result<H256> {
    let from_leader_account = H160(*GANACHE_ADDRESSES["leader"].as_bytes());

    sweeper_contract
        .call(
            "sweep",
            (
                Token::Address(H160(outpace_address)),
                channel.tokenize(),
                Token::Array(vec![Token::Address(H160(depositor))]),
            ),
            from_leader_account,
            Options::with(|opt| {
                opt.gas_price = Some(1.into());
                opt.gas = Some(6_721_975.into());
            }),
        )
        .await
}

/// Deploys the Sweeper contract from `GANACHE_ADDRESS['leader']`
pub async fn deploy_sweeper_contract(
    web3: &Web3<Http>,
) -> web3::contract::Result<(H160, Contract<Http>)> {
    let from_leader_account = H160(*GANACHE_ADDRESSES["leader"].as_bytes());

    let sweeper_contract = Contract::deploy(web3.eth(), &SWEEPER_ABI)
        .expect("Invalid ABI of Sweeper contract")
        .confirmations(0)
        .options(Options::with(|opt| {
            opt.gas_price = Some(1.into());
            opt.gas = Some(6_721_975.into());
        }))
        .execute(*SWEEPER_BYTECODE, (), from_leader_account)
        .await?;

    Ok((sweeper_contract.address(), sweeper_contract))
}

/// Deploys the Outpace contract from `GANACHE_ADDRESS['leader']`
pub async fn deploy_outpace_contract(
    web3: &Web3<Http>,
) -> web3::contract::Result<(H160, Contract<Http>)> {
    let from_leader_account = H160(*GANACHE_ADDRESSES["leader"].as_bytes());

    let outpace_contract = Contract::deploy(web3.eth(), &OUTPACE_ABI)
        .expect("Invalid ABI of Sweeper contract")
        .confirmations(0)
        .options(Options::with(|opt| {
            opt.gas_price = Some(1.into());
            opt.gas = Some(6_721_975.into());
        }))
        .execute(*OUTPACE_BYTECODE, (), from_leader_account)
        .await?;

    Ok((outpace_contract.address(), outpace_contract))
}

/// Deploys the Mock Token contract from `GANACHE_ADDRESS['leader']`
pub async fn deploy_token_contract(
    web3: &Web3<Http>,
    min_token_units: u64,
) -> web3::contract::Result<(TokenInfo, Address, Contract<Http>)> {
    let from_leader_account = H160(*GANACHE_ADDRESSES["leader"].as_bytes());

    let token_contract = Contract::deploy(web3.eth(), &MOCK_TOKEN_ABI)
        .expect("Invalid ABI of Mock Token contract")
        .confirmations(0)
        .options(Options::with(|opt| {
            opt.gas_price = Some(1.into());
            opt.gas = Some(6_721_975.into());
        }))
        .execute(*MOCK_TOKEN_BYTECODE, (), from_leader_account)
        .await?;

    let token_info = TokenInfo {
        min_token_units_for_deposit: BigNum::from(min_token_units),
        precision: NonZeroU8::new(18).expect("should create NonZeroU8"),
        // 0.000_1
        min_validator_fee: BigNum::from(100_000_000_000_000),
    };

    Ok((
        token_info,
        Address::from(token_contract.address().as_fixed_bytes()),
        token_contract,
    ))
}
