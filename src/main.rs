// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// #![deny(warnings)]

mod grpc;
mod sm;
mod util;

use crate::grpc::Client;
use clap::{App, Arg};
use hyper::{
    http::{Method, Request, Response, StatusCode},
    service::{make_service_fn, service_fn},
    Body, Server,
};
use std::mem;
use tokio::time::{self, Duration};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

static NOTFOUND: &[u8] = b"Not Found";

async fn response(req: Request<Body>, mut client: Client) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/block_number") => {
            let height = client.block_number().await;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(format!("height: {}", height).into())
                .unwrap())
        }
        (&Method::GET, "/auto_send_store_transaction") => {
            let hash = client.auto_send_store_transaction().await;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(hex::encode(hash).into())
                .unwrap())
        }
        _ => {
            // Return 404 not found response.
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(NOTFOUND.into())
                .unwrap())
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let matches = App::new("CITA-CLOUD RELAY SERVER")
        .author("Yieazy, Rivtower Technologies.")
        .version("0.1.0")
        .about("relay http request to grpc")
        .arg(
            Arg::new("listen-port")
                .short('p')
                .long("port")
                .takes_value(true)
                .about("Relay server listen port, default 1337")
        )
        .arg(
            Arg::new("controller-addr")
                .short('c')
                .long("controller")
                .takes_value(true)
                .about("Set controller rpc address, default http://127.0.0.1:50004")
        )
        .arg(
            Arg::new("evm-addr")
                .short('e')
                .long("evm")
                .takes_value(true)
                .about("Set evm rpc address, default http://127.0.0.1:50002")
        )
        .arg(
            Arg::new("private-key")
                .short('k')
                .long("key")
                .takes_value(true)
                .about("Set private key, default 3ef2627393529fed043c7dbfd9358a4ae47a88a59949b07e7631722fd6959002")
        )
        .get_matches();

    let addr = {
        let port = matches
            .value_of("listen-port")
            .unwrap_or("1337")
            .to_string();
        format!("0.0.0.0:{}", port).parse().unwrap()
    };

    let controller_addr = matches
        .value_of("controller-addr")
        .unwrap_or("http://127.0.0.1:50004")
        .to_string();
    let evm_addr = matches
        .value_of("evm-addr")
        .unwrap_or("http://127.0.0.1:50002")
        .to_string();
    let priv_key = matches
        .value_of("private-key")
        .unwrap_or("3ef2627393529fed043c7dbfd9358a4ae47a88a59949b07e7631722fd6959002")
        .to_string();

    let client = Client::new(controller_addr, evm_addr, priv_key).await;

    let mut work_client = client.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            {
                *work_client.chain_height.write().await = work_client.block_number().await;
                let raw_tx = {
                    let mut wr = work_client.raw_txs.write().await;
                    mem::take(&mut *wr)
                };
                if !raw_tx.body.is_empty() {
                    work_client.send_raw_transactions(raw_tx).await;
                }
            }
        }
    });

    let new_service = make_service_fn(move |_| {
        let client = client.clone();
        async {
            Ok::<_, GenericError>(service_fn(move |req| {
                // Clone again to ensure that client outlives this closure.
                response(req, client.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(new_service);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
