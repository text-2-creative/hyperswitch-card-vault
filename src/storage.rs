use crate::{
    config::Database,
    crypto::Encryption,
    error::{self, ContainerError},
};

use std::sync::Arc;

use diesel_async::{
    pooled_connection::{
        self,
        deadpool::{Object, Pool},
    },
    AsyncPgConnection,
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};

pub mod db;
pub mod schema;
pub mod types;

pub trait State {}

/// Storage State that is to be passed though the application
#[derive(Clone)]
pub struct Storage {
    pg_pool: Arc<Pool<AsyncPgConnection>>,
}

type DeadPoolConnType = Object<AsyncPgConnection>;

impl Storage {
    /// Create a new storage interface from configuration
    pub async fn new(database: &Database) -> error_stack::Result<Self, error::StorageError> {
        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            database.username,
            database.password.peek(),
            database.host,
            database.port,
            database.dbname
        );
        let config =
            pooled_connection::AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
        let pool = Pool::builder(config);

        let pool = match database.pool_size {
            Some(value) => pool.max_size(value),
            None => pool,
        };

        let pool = pool
            .build()
            .change_context(error::StorageError::DBPoolError)?;
        Ok(Self {
            pg_pool: Arc::new(pool),
        })
    }

    /// Get connection from database pool for accessing data
    pub async fn get_conn(&self) -> Result<DeadPoolConnType, ContainerError<error::StorageError>> {
        Ok(self
            .pg_pool
            .get()
            .await
            .change_context(error::StorageError::PoolClientFailure)?)
    }
}

///
/// MerchantInterface:
///
/// Interface providing functional to interface with the merchant table in database
#[async_trait::async_trait]
pub trait MerchantInterface {
    type Algorithm: Encryption<Vec<u8>, Vec<u8>>;
    type Error;

    /// find merchant from merchant table with `merchant_id` and `tenant_id` with key as master key
    async fn find_by_merchant_id(
        &self,
        merchant_id: &str,
        tenant_id: &str,
        key: &Self::Algorithm,
    ) -> Result<types::Merchant, ContainerError<Self::Error>>;

    /// find merchant from merchant table with `merchant_id` and `tenant_id` with key as master key
    /// and if not found create a new merchant
    async fn find_or_create_by_merchant_id(
        &self,
        merchant_id: &str,
        tenant_id: &str,
        key: &Self::Algorithm,
    ) -> Result<types::Merchant, ContainerError<Self::Error>>;

    /// Insert a new merchant in the database by encrypting the dek with `master_key`
    async fn insert_merchant(
        &self,
        new: types::MerchantNew<'_>,
        key: &Self::Algorithm,
    ) -> Result<types::Merchant, ContainerError<Self::Error>>;
}

///
/// LockerInterface:
///
/// Interface for interacting with the locker database table
#[async_trait::async_trait]
pub trait LockerInterface {
    type Algorithm: Encryption<Vec<u8>, Vec<u8>>;
    type Error;

    /// Fetch payment data from locker table by decrypting with `dek`
    async fn find_by_locker_id_merchant_id_customer_id(
        &self,
        locker_id: Secret<String>,
        tenant_id: &str,
        merchant_id: &str,
        customer_id: &str,
        key: &Self::Algorithm,
    ) -> Result<types::Locker, ContainerError<Self::Error>>;

    /// Insert payment data from locker table by decrypting with `dek`
    async fn insert_or_get_from_locker(
        &self,
        new: types::LockerNew<'_>,
        key: &Self::Algorithm,
    ) -> Result<types::Locker, ContainerError<Self::Error>>;

    /// Delete card from the locker, without access to the `dek`
    async fn delete_from_locker(
        &self,
        locker_id: Secret<String>,
        tenant_id: &str,
        merchant_id: &str,
        customer_id: &str,
    ) -> Result<usize, ContainerError<Self::Error>>;

    async fn find_by_hash_id_merchant_id_customer_id(
        &self,
        hash_id: &str,
        tenant_id: &str,
        merchant_id: &str,
        customer_id: &str,
        key: &Self::Algorithm,
    ) -> Result<Option<types::Locker>, ContainerError<Self::Error>>;
}

/// Trait defining behaviour of the application with the hash table, providing APIs to interact
/// with it
#[async_trait::async_trait]
pub trait HashInterface {
    type Error;

    async fn find_by_data_hash(
        &self,
        data_hash: &[u8],
    ) -> Result<Option<types::HashTable>, ContainerError<Self::Error>>;
    async fn insert_hash(
        &self,
        data_hash: Vec<u8>,
    ) -> Result<types::HashTable, ContainerError<Self::Error>>;
}
