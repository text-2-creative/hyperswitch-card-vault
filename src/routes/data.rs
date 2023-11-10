use axum::{extract, routing::post, Json};

#[cfg(feature = "middleware")]
use axum::middleware;

use error_stack::ResultExt;
use masking::ExposeInterface;

use crate::{
    app::AppState,
    crypto::{aes::GcmAes256, sha::Sha512, Encode},
    error::{self, LogReport},
    storage::{HashInterface, LockerInterface, MerchantInterface},
};

#[cfg(feature = "middleware")]
use crate::middleware as custom_middleware;

use self::types::Validation;

mod transformers;
pub mod types;

///
/// Function for creating the server that is specifically handling the cards api
///
#[allow(clippy::let_and_return)]
pub fn serve(#[cfg(feature = "middleware")] state: AppState) -> axum::Router<AppState> {
    let router = axum::Router::new()
        .route("/add", post(add_card))
        .route("/delete", post(delete_card))
        .route("/retrieve", post(retrieve_card));

    #[cfg(feature = "middleware")]
    {
        router.layer(middleware::from_fn_with_state(
            state,
            custom_middleware::middleware,
        ))
    }
    #[cfg(not(feature = "middleware"))]
    router
}

/// `/data/add` handling the requirement of storing cards
pub async fn add_card(
    extract::State(state): extract::State<AppState>,
    Json(request): Json<types::StoreCardRequest>,
) -> Result<Json<types::StoreCardResponse>, error::ApiError> {
    request.validate()?;

    let master_encryption = GcmAes256::new(state.config.secrets.master_key);
    let merchant = state
        .db
        .find_or_create_by_merchant_id(
            &request.merchant_id,
            &state.config.secrets.tenant,
            &master_encryption,
        )
        .await
        .change_context(error::ApiError::RetrieveDataFailed("merchant"))
        .report_unwrap()?;

    let merchant_dek = GcmAes256::new(merchant.enc_key.expose());

    let hash_data = serde_json::to_vec(&request.data)
        .change_context(error::ApiError::EncodingError)
        .and_then(|data| {
            (Sha512)
                .encode(data)
                .change_context(error::ApiError::EncodingError)
        })
        .report_unwrap()?;

    let optional_hash_table = state
        .db
        .find_by_data_hash(&hash_data)
        .await
        .change_context(error::ApiError::DatabaseRetrieveFailed("hash_table"))
        .report_unwrap()?;

    let output = match optional_hash_table {
        Some(hash_table) => {
            let stored_data = state
                .db
                .find_by_hash_id_merchant_id_customer_id(
                    &hash_table.hash_id,
                    &state.config.secrets.tenant,
                    &request.merchant_id,
                    &request.merchant_customer_id,
                    &merchant_dek,
                )
                .await
                .change_context(error::ApiError::DatabaseRetrieveFailed("locker"))
                .report_unwrap()?;

            match stored_data {
                Some(data) => data,
                None => state
                    .db
                    .insert_or_get_from_locker(
                        (
                            request,
                            state.config.secrets.tenant.as_str(),
                            hash_table.hash_id.as_str(),
                        )
                            .try_into()?,
                        &merchant_dek,
                    )
                    .await
                    .change_context(error::ApiError::DatabaseInsertFailed("locker"))
                    .report_unwrap()?,
            }
        }
        None => {
            let hash_table = state
                .db
                .insert_hash(hash_data)
                .await
                .change_context(error::ApiError::DatabaseInsertFailed("hash_table"))
                .report_unwrap()?;

            state
                .db
                .insert_or_get_from_locker(
                    (
                        request,
                        state.config.secrets.tenant.as_str(),
                        hash_table.hash_id.as_str(),
                    )
                        .try_into()?,
                    &merchant_dek,
                )
                .await
                .change_context(error::ApiError::DatabaseInsertFailed("locker"))
                .report_unwrap()?
        }
    };

    Ok(Json(output.into()))
}

/// `/data/delete` handling the requirement of deleting cards
pub async fn delete_card(
    extract::State(state): extract::State<AppState>,
    Json(request): Json<types::DeleteCardRequest>,
) -> Result<Json<types::DeleteCardResponse>, error::ApiError> {
    let master_key = GcmAes256::new(state.config.secrets.master_key.clone());

    let _merchant = state
        .db
        .find_by_merchant_id(
            &request.merchant_id,
            &state.config.secrets.tenant,
            &master_key,
        )
        .await
        .change_context(error::ApiError::DatabaseRetrieveFailed("merchant"))
        .report_unwrap()?;

    let _delete_status = state
        .db
        .delete_from_locker(
            request.card_reference.into(),
            &state.config.secrets.tenant,
            &request.merchant_id,
            &request.merchant_customer_id,
        )
        .await
        .change_context(error::ApiError::DatabaseDeleteFailed("locker"))
        .report_unwrap()?;

    Ok(Json(types::DeleteCardResponse {
        status: types::Status::Ok,
    }))
}

/// `/data/retrieve` handling the requirement of retrieving cards
pub async fn retrieve_card(
    extract::State(state): extract::State<AppState>,
    Json(request): Json<types::RetrieveCardRequest>,
) -> Result<Json<types::RetrieveCardResponse>, error::ApiError> {
    let master_key = GcmAes256::new(state.config.secrets.master_key.clone());

    let merchant = state
        .db
        .find_by_merchant_id(
            &request.merchant_id,
            &state.config.secrets.tenant,
            &master_key,
        )
        .await
        .change_context(error::ApiError::DatabaseDeleteFailed("locker"))
        .report_unwrap()?;

    let merchant_dek = GcmAes256::new(merchant.enc_key.expose());

    let card = state
        .db
        .find_by_locker_id_merchant_id_customer_id(
            request.card_reference.into(),
            &state.config.secrets.tenant,
            &request.merchant_id,
            &request.merchant_customer_id,
            &merchant_dek,
        )
        .await
        .change_context(error::ApiError::DatabaseDeleteFailed("locker"))
        .report_unwrap()?;

    Ok(Json(card.try_into()?))
}
