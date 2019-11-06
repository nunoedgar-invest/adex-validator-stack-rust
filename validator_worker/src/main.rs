#![deny(rust_2018_idioms)]
#![deny(clippy::all)]

use clap::{App, Arg};

use adapter::{AdapterTypes, DummyAdapter, EthereumAdapter};
use futures::compat::Future01CompatExt;
use futures::future::try_join_all;
use futures::future::{join, FutureExt, TryFutureExt};
use primitives::adapter::{Adapter, DummyAdapterOptions, KeystoreOptions};
use primitives::config::{configuration, Config};
use primitives::util::tests::prep_db::{AUTH, IDS};
use primitives::{Channel, SpecValidator, ValidatorId};
use std::convert::TryFrom;
use std::error::Error;
use std::ops::Add;
use std::time::{Duration, Instant};
use tokio::timer::Delay;
use tokio::util::FutureExt as TokioFutureExt;
use validator_worker::error::ValidatorWorker as ValidatorWorkerError;
use validator_worker::{all_channels, follower, leader, SentryApi};

#[derive(Debug, Clone)]
struct Args<A: Adapter> {
    sentry_url: String,
    config: Config,
    adapter: A,
    whoami: ValidatorId,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = App::new("Validator worker")
        .version("0.1")
        .arg(
            Arg::with_name("config")
                .help("the config file for the validator worker")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("adapter")
                .short("a")
                .help("the adapter for authentication and signing")
                .required(true)
                .default_value("ethereum")
                .possible_values(&["ethereum", "dummy"])
                .takes_value(true),
        )
        .arg(
            Arg::with_name("keystoreFile")
                .short("k")
                .help("path to the JSON Ethereum Keystore file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("dummyIdentity")
                .short("i")
                .help("the identity to use with the dummy adapter")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("sentryUrl")
                .short("u")
                .help("the URL to the sentry used for listing channels")
                .default_value("http://127.0.0.1:8005")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("singleTick")
                .short("t")
                .takes_value(false)
                .help("runs the validator in single-tick mode and exis"),
        )
        .get_matches();

    let environment = std::env::var("ENV").unwrap_or_else(|_| "development".into());
    let config_file = cli.value_of("config");
    let config = configuration(&environment, config_file).expect("failed to parse configuration");
    let sentry_url = cli.value_of("sentryUrl").expect("sentry url missing");
    let is_single_tick = cli.is_present("singleTick");

    let adapter = match cli.value_of("adapter").unwrap() {
        "ethereum" => {
            let keystore_file = cli
                .value_of("keystoreFile")
                .expect("unable to get keystore file");
            let keystore_pwd = std::env::var("KEYSTORE_PWD").expect("unable to get keystore pwd");
            let keystore_options = KeystoreOptions {
                keystore_file: keystore_file.to_string(),
                keystore_pwd,
            };
            AdapterTypes::EthereumAdapter(Box::new(
                EthereumAdapter::init(keystore_options, &config).expect("failed to init adapter"),
            ))
        }
        "dummy" => {
            let dummy_identity = cli
                .value_of("dummyIdentity")
                .expect("unable to get dummyIdentity");
            let options = DummyAdapterOptions {
                dummy_identity: ValidatorId::try_from(dummy_identity)?,
                dummy_auth: IDS.clone(),
                dummy_auth_tokens: AUTH.clone(),
            };
            AdapterTypes::DummyAdapter(Box::new(DummyAdapter::init(options, &config)))
        }
        // @TODO exit gracefully
        _ => panic!("We don't have any other adapters implemented yet!"),
    };

    match adapter {
        AdapterTypes::EthereumAdapter(ethadapter) => {
            run(is_single_tick, &sentry_url, &config, *ethadapter)
        }
        AdapterTypes::DummyAdapter(dummyadapter) => {
            run(is_single_tick, &sentry_url, &config, *dummyadapter)
        }
    }
}

fn run<A: Adapter + 'static>(
    is_single_tick: bool,
    sentry_url: &str,
    config: &Config,
    adapter: A,
) -> Result<(), Box<dyn Error>> {
    let mut sentry_adapter = adapter.clone();
    // unlock adapter
    sentry_adapter.unlock()?;
    let whoami = adapter.whoami().to_owned();

    let args = Args {
        sentry_url: sentry_url.to_owned(),
        config: config.to_owned(),
        adapter: sentry_adapter,
        whoami,
    };

    if is_single_tick {
        tokio::run(iterate_channels(args).boxed().compat());
    } else {
        tokio::run(infinite(args).boxed().compat());
    }

    Ok(())
}

async fn infinite<A: Adapter + 'static>(args: Args<A>) -> Result<(), ()> {
    loop {
        let arg = args.clone();
        let delay_future =
            Delay::new(Instant::now().add(Duration::from_secs(arg.config.wait_time as u64)));
        let joined = join(iterate_channels(arg), delay_future.compat());
        if let (_, Err(e)) = joined.await {
            eprintln!("{}", e);
        }
    }
}

async fn iterate_channels<A: Adapter + 'static>(args: Args<A>) -> Result<(), ()> {
    let result = all_channels(&args.sentry_url, args.whoami.to_string()).await;

    if let Err(e) = result {
        eprintln!("Failed to get channels {}", e);
        return Ok(());
    }

    let channels = result.unwrap();
    let channels_size = channels.len();

    let tick =
        try_join_all(channels.into_iter().map(|channel| {
            validator_tick(args.adapter.clone(), channel, &args.config, &args.whoami)
        }))
        .await;

    if let Err(e) = tick {
        eprintln!("An occurred while processing channels {}", e);
    }

    if channels_size >= args.config.max_channels as usize {
        eprintln!(
            "WARNING: channel limit cfg.MAX_CHANNELS={} reached",
            args.config.max_channels
        )
    }
    Ok(())
}

async fn validator_tick<A: Adapter + 'static>(
    adapter: A,
    channel: Channel,
    config: &Config,
    whoami: &ValidatorId,
) -> Result<(), ValidatorWorkerError> {
    let sentry = SentryApi::init(adapter, &channel, &config, true, whoami)?;
    let duration = Duration::from_secs(config.validator_tick_timeout as u64);

    match channel.spec.validators.find(&whoami) {
        SpecValidator::Leader(_) => {
            if let Err(e) = leader::tick(&sentry)
                .boxed()
                .compat()
                .timeout(duration)
                .compat()
                .await
            {
                return Err(ValidatorWorkerError::Failed(e.to_string()));
            }
        }
        SpecValidator::Follower(_) => {
            if let Err(e) = follower::tick(&sentry)
                .boxed()
                .compat()
                .timeout(duration)
                .compat()
                .await
            {
                return Err(ValidatorWorkerError::Failed(e.to_string()));
            }
        }
        SpecValidator::None => {
            return Err(ValidatorWorkerError::Failed(
                "validatorTick: processing a channel where we are not validating".to_string(),
            ))
        }
    };
    Ok(())
}
