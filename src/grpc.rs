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

use crate::sm::{sm2_sign, sm3_hash};
use crate::util::pk2address;
use cita_cloud_proto::{
    blockchain::{
        raw_transaction::Tx, RawTransaction, Transaction, UnverifiedTransaction, Witness,
    },
    common::Empty,
    controller::{rpc_service_client::RpcServiceClient as Controller, Flag},
    evm::rpc_service_client::RpcServiceClient as Evm,
};
use efficient_sm2::KeyPair;
use prost::Message;
use rand::{thread_rng, Rng};
use tonic::transport::{Channel, Endpoint};

pub const STORE_ADDRESS: &str = "ffffffffffffffffffffffffffffffffff010000";

#[derive(Clone)]
pub struct Client {
    controller: Controller<Channel>,
    evm: Evm<Channel>,

    key_pair: KeyPair,
}

impl Client {
    pub fn new(controller_addr: String, evm_addr: String, priv_kay: String) -> Self {
        let controller = {
            let channel = Endpoint::from_shared(controller_addr)
                .unwrap()
                .connect_lazy()
                .unwrap();
            Controller::new(channel)
        };

        let evm = {
            // use the same addr as executor
            let channel = Endpoint::from_shared(evm_addr)
                .unwrap()
                .connect_lazy()
                .unwrap();
            Evm::new(channel)
        };

        let key_pair = KeyPair::new(&hex::decode(priv_kay).unwrap()).unwrap();

        Self {
            controller,
            evm,

            key_pair,
        }
    }

    async fn build_transaction(
        &mut self,
        to: Vec<u8>,
        data: Vec<u8>,
        value: Vec<u8>,
    ) -> Transaction {
        // get start block number
        let start_block_number = self.block_number().await;
        let sys_config = self
            .controller
            .get_system_config(Empty {})
            .await
            .unwrap()
            .into_inner();
        let nonce = rand::random::<u64>().to_string();
        Transaction {
            version: sys_config.version,
            to,
            nonce,
            quota: 3_000_000,
            valid_until_block: start_block_number + 99,
            data,
            value,
            chain_id: sys_config.chain_id.to_vec(),
        }
    }

    fn build_raw_transaction(&self, tx: Transaction) -> RawTransaction {
        // calc tx hash
        let tx_hash = {
            // build tx bytes
            let tx_bytes = {
                let mut buf = Vec::with_capacity(tx.encoded_len());
                tx.encode(&mut buf).unwrap();
                buf
            };
            sm3_hash(tx_bytes.as_slice())
        };

        let signature = sm2_sign(&self.key_pair, &tx_hash).to_vec();

        // build raw tx
        let raw_tx = {
            let witness = Witness {
                signature,
                sender: pk2address(&self.key_pair.public_key().bytes_less_safe()[1..]),
            };

            let unverified_tx = UnverifiedTransaction {
                transaction: Some(tx),
                transaction_hash: tx_hash.to_vec(),
                witness: Some(witness),
            };

            RawTransaction {
                tx: Some(Tx::NormalTx(unverified_tx)),
            }
        };

        raw_tx
    }

    async fn send_raw_transaction(&self, raw: RawTransaction) -> Vec<u8> {
        self.controller
            .clone()
            .send_raw_transaction(raw)
            .await
            .unwrap()
            .into_inner()
            .hash
    }

    pub async fn auto_send_store_transaction(&mut self) -> Vec<u8> {
        let data: [u8; 32] = {
            let mut rng = thread_rng();
            rng.gen()
        };

        let value = [0; 32];

        let tx = self
            .build_transaction(
                hex::decode(STORE_ADDRESS).unwrap().to_vec(),
                data.to_vec(),
                value.to_vec(),
            )
            .await;
        let raw_tx = self.build_raw_transaction(tx);

        self.send_raw_transaction(raw_tx).await
    }

    pub async fn block_number(&mut self) -> u64 {
        self.controller
            .get_block_number(Flag { flag: true })
            .await
            .unwrap()
            .into_inner()
            .block_number
    }
}