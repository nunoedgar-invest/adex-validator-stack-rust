#![feature(async_await, await_macro)]
#![deny(rust_2018_idioms)]
#![deny(clippy::all)]
use reqwest::r#async::Client;

use futures::future::{FutureExt, TryFutureExt};
use validator::domain::channel::ChannelRepository;

fn main() {
    let future = async {
        let repo = validator::infrastructure::persistence::channel::api::ApiChannelRepository {
            client: Client::new(),
        };
        println!(
            "{:#?}",
            await!(repo.all("0x2892f6C41E0718eeeDd49D98D648C789668cA67d"))
        );
    };

    tokio::run(future.unit_error().boxed().compat());
}