// Copyright (c), Mysten Labs, Inc.
// Copyright (c), The Social Proof Foundation, LLC.
// SPDX-License-Identifier: Apache-2.0

use crate::cache::default_lru_cache;
use crate::errors::InternalError;
use crate::key_server_options::KeyServerOptions;
use crate::mys_rpc_client::MysRpcClient;
use crate::{mvr_forward_resolution, Timestamp};
use moka::sync::Cache;
use once_cell::sync::Lazy;
use mys_sdk::error::MysRpcResult;
use mys_sdk::rpc_types::{CheckpointId, MysData, MysObjectDataOptions};
use mys_types::base_types::ObjectID;
use tap::TapFallible;
use tracing::{debug, warn};

static CACHE: Lazy<Cache<ObjectID, ObjectID>> = Lazy::new(default_lru_cache);
static MVR_CACHE: Lazy<Cache<String, ObjectID>> = Lazy::new(default_lru_cache);

#[cfg(test)]
pub(crate) fn add_package(pkg_id: ObjectID) {
    CACHE.insert(pkg_id, pkg_id);
}

#[cfg(test)]
pub(crate) fn add_upgraded_package(pkg_id: ObjectID, new_pkg_id: ObjectID) {
    CACHE.insert(new_pkg_id, pkg_id);
}

pub(crate) async fn check_mvr_package_id(
    mvr_name: &Option<String>,
    mys_rpc_client: &MysRpcClient,
    key_server_options: &KeyServerOptions,
    first_pkg_id: ObjectID,
    req_id: Option<&str>,
) -> Result<(), InternalError> {
    // If an MVR name is provided, get it from cache or resolve it to the package
    // id. Then check that it points to the first package ID.
    if let Some(mvr_name) = &mvr_name {
        let mvr_package_id = match get_mvr_cache(mvr_name) {
            None => {
                let mvr_package_id =
                    mvr_forward_resolution(mys_rpc_client, mvr_name, key_server_options).await?;
                insert_mvr_cache(mvr_name, mvr_package_id);
                mvr_package_id
            }
            Some(mvr_package_id) => {
                debug!(
                    "MVR name {} is already in cache (req_id: {:?})",
                    mvr_name, req_id
                );
                mvr_package_id
            }
        };
        if mvr_package_id != first_pkg_id {
            debug!(
                "MVR name {} points to package ID {:?} while the first package ID is {:?} (req_id: {:?})",
                mvr_name, mvr_package_id, first_pkg_id, req_id
            );
            return Err(InternalError::InvalidMVRName);
        }
    }
    Ok(())
}

pub(crate) async fn fetch_first_pkg_id(
    pkg_id: &ObjectID,
    mys_rpc_client: &MysRpcClient,
) -> Result<ObjectID, InternalError> {
    match CACHE.get(pkg_id) {
        Some(first) => Ok(first),
        None => {
            let object = mys_rpc_client
                .get_object_with_options(*pkg_id, MysObjectDataOptions::default().with_bcs())
                .await
                .map_err(|_| InternalError::Failure("FN failed to respond".to_string()))? // internal error that fullnode fails to respond, check fullnode.
                .into_object()
                .map_err(|_| InternalError::InvalidPackage)?; // user error that object does not exist or deleted.

            let package = object
                .bcs
                .ok_or(InternalError::Failure(
                    "No BCS object in response".to_string(),
                ))? // internal error that fullnode does not respond with bcs even though request includes the bcs option.
                .try_as_package()
                .ok_or(InternalError::InvalidPackage)?
                .to_move_package(u64::MAX)
                .map_err(|_| InternalError::InvalidPackage)?; // user error if the provided package throw MovePackageTooBig.

            let first = package.original_package_id();
            CACHE.insert(*pkg_id, first);
            Ok(first)
        }
    }
}

pub(crate) fn insert_mvr_cache(mvr_name: &str, package_id: ObjectID) {
    MVR_CACHE.insert(mvr_name.to_string(), package_id);
}

pub(crate) fn get_mvr_cache(mvr_name: &str) -> Option<ObjectID> {
    MVR_CACHE.get(&mvr_name.to_string())
}

/// Returns the timestamp for the latest checkpoint.
pub(crate) async fn get_latest_checkpoint_timestamp(
    mys_rpc_client: MysRpcClient,
) -> MysRpcResult<Timestamp> {
    let latest_checkpoint_sequence_number = mys_rpc_client
        .get_latest_checkpoint_sequence_number()
        .await?;
    let checkpoint = mys_rpc_client
        .get_checkpoint(CheckpointId::SequenceNumber(
            latest_checkpoint_sequence_number,
        ))
        .await?;
    Ok(checkpoint.timestamp_ms)
}

pub(crate) async fn get_reference_gas_price(mys_rpc_client: MysRpcClient) -> MysRpcResult<u64> {
    let rgp = mys_rpc_client
        .get_reference_gas_price()
        .await
        .tap_err(|e| {
            warn!("Failed retrieving RGP ({:?})", e);
        })?;
    Ok(rgp)
}

#[cfg(test)]
mod tests {
    use crate::externals::fetch_first_pkg_id;
    use crate::key_server_options::RetryConfig;
    use crate::mys_rpc_client::MysRpcClient;
    use crate::types::Network;
    use crate::InternalError;
    use fastcrypto::ed25519::Ed25519KeyPair;
    use fastcrypto::secp256k1::Secp256k1KeyPair;
    use fastcrypto::secp256r1::Secp256r1KeyPair;
    use shared_crypto::intent::{Intent, IntentMessage, PersonalMessage};
    use std::str::FromStr;
    use mys_sdk::types::crypto::{get_key_pair, Signature};
    use mys_sdk::types::signature::GenericSignature;
    use mys_sdk::verify_personal_message_signature::verify_personal_message_signature;
    use mys_sdk::MysClientBuilder;
    use mys_types::base_types::ObjectID;

    #[tokio::test]
    async fn test_fetch_first_pkg_id() {
        let address = ObjectID::from_str(
            "0xac7890f847ac6973ca615af9d7bbb642541f175e35e340e5d1241d0ffda9ed04",
        )
        .unwrap();
        let mys_rpc_client = MysRpcClient::new(
            MysClientBuilder::default()
                .build(&Network::Testnet.node_url())
                .await
                .expect(
                    "MysClientBuilder should not failed unless provided with invalid network url",
                ),
            RetryConfig::default(),
            None,
        );
        match fetch_first_pkg_id(&address, &mys_rpc_client).await {
            Ok(first) => {
                assert_eq!(
                    first.to_hex_literal(),
                    "0x717d42d8205adeb14b440d6b46c8524d7479952099435261defa1b57f151bf16"
                        .to_string()
                );
                println!("First address: {:?}", first);
            }
            Err(e) => panic!("Test failed with error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_fetch_first_pkg_id_with_invalid_id() {
        let invalid_address = ObjectID::ZERO;
        let mys_rpc_client = MysRpcClient::new(
            MysClientBuilder::default()
                .build(&Network::Mainnet.node_url())
                .await
                .expect(
                    "MysClientBuilder should not failed unless provided with invalid network url",
                ),
            RetryConfig::default(),
            None,
        );
        let result = fetch_first_pkg_id(&invalid_address, &mys_rpc_client).await;
        assert!(matches!(result, Err(InternalError::InvalidPackage)));
    }

    #[tokio::test]
    async fn test_simple_sigs() {
        let personal_msg = PersonalMessage {
            message: "hello".as_bytes().to_vec(),
        };
        let msg_with_intent = IntentMessage::new(Intent::personal_message(), personal_msg.clone());

        // simple sigs
        {
            let (addr, sk): (_, Ed25519KeyPair) = get_key_pair();
            let sig = GenericSignature::Signature(Signature::new_secure(&msg_with_intent, &sk));
            assert!(verify_personal_message_signature(
                sig.clone(),
                &personal_msg.message,
                addr,
                None
            )
            .await
            .is_ok());

            let (wrong_addr, _): (_, Ed25519KeyPair) = get_key_pair();
            assert!(verify_personal_message_signature(
                sig.clone(),
                &personal_msg.message,
                wrong_addr,
                None
            )
            .await
            .is_err());

            let wrong_msg = PersonalMessage {
                message: "wrong".as_bytes().to_vec(),
            };
            assert!(
                verify_personal_message_signature(sig.clone(), &wrong_msg.message, addr, None)
                    .await
                    .is_err()
            );
        }
        {
            let (addr, sk): (_, Secp256k1KeyPair) = get_key_pair();
            let sig = GenericSignature::Signature(Signature::new_secure(&msg_with_intent, &sk));
            assert!(verify_personal_message_signature(
                sig.clone(),
                &personal_msg.message,
                addr,
                None
            )
            .await
            .is_ok());
            let (wrong_addr, _): (_, Secp256k1KeyPair) = get_key_pair();
            assert!(verify_personal_message_signature(
                sig.clone(),
                &personal_msg.message,
                wrong_addr,
                None
            )
            .await
            .is_err());
            let wrong_msg = PersonalMessage {
                message: "wrong".as_bytes().to_vec(),
            };
            assert!(
                verify_personal_message_signature(sig.clone(), &wrong_msg.message, addr, None)
                    .await
                    .is_err()
            );
        }
        {
            let (addr, sk): (_, Secp256r1KeyPair) = get_key_pair();
            let sig = GenericSignature::Signature(Signature::new_secure(&msg_with_intent, &sk));
            assert!(verify_personal_message_signature(
                sig.clone(),
                &personal_msg.message,
                addr,
                None
            )
            .await
            .is_ok());

            let (wrong_addr, _): (_, Secp256r1KeyPair) = get_key_pair();
            assert!(verify_personal_message_signature(
                sig.clone(),
                &personal_msg.message,
                wrong_addr,
                None
            )
            .await
            .is_err());
            let wrong_msg = PersonalMessage {
                message: "wrong".as_bytes().to_vec(),
            };
            assert!(
                verify_personal_message_signature(sig.clone(), &wrong_msg.message, addr, None)
                    .await
                    .is_err()
            );
        }
    }
}
