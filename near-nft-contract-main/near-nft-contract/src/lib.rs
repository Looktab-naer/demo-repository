use near_contract_standards::non_fungible_token::core::{
    NonFungibleTokenCore, NonFungibleTokenResolver,
};
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::NonFungibleToken;
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, UnorderedMap, UnorderedSet};
use near_sdk::env::is_valid_account_id;
use near_sdk::json_types::{ValidAccountId, U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, ext_contract, near_bindgen, serde_json::json,
    serde_json::value::Value::Null, AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault,
    Promise, PromiseOrValue, Timestamp,
};
use std::collections::HashMap;

pub mod event;
pub use event::NearEvent;

/// between token_series_id and edition number e.g. 42:2 where 42 is series and 2 is edition
pub const TOKEN_DELIMETER: char = ':';
/// TokenMetadata.title returned for individual token e.g. "Title — 2/10" where 10 is max copies
pub const TITLE_DELIMETER: &str = " #";
/// e.g. "Title — 2/10" where 10 is max copies
pub const EDITION_DELIMETER: &str = "/";

const GAS_FOR_RESOLVE_TRANSFER: Gas = 10_000_000_000_000;
const GAS_FOR_NFT_TRANSFER_CALL: Gas = 30_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER;
const GAS_FOR_NFT_APPROVE: Gas = 10_000_000_000_000;
const GAS_FOR_MINT: Gas = 90_000_000_000_000;
const NO_DEPOSIT: Balance = 0;
const MAX_PRICE: Balance = 1_000_000_000 * 10u128.pow(24);

pub type TokenSeriesId = String;
pub type TimestampSec = u32;
pub type ContractAndTokenId = String;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Payout {
    pub payout: HashMap<AccountId, U128>,
}

#[ext_contract(ext_non_fungible_token_receiver)]
trait NonFungibleTokenReceiver {
    /// Returns `true` if the token should be returned back to the sender.
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> Promise;
}

#[ext_contract(ext_approval_receiver)]
pub trait NonFungibleTokenReceiver {
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: u64,
        msg: String,
    );
}

#[ext_contract(ext_self)]
trait NonFungibleTokenResolver {
    fn nft_resolve_transfer(
        &mut self,
        previous_owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        approved_account_ids: Option<HashMap<AccountId, u64>>,
    ) -> bool;
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct TokenSeries {
    metadata: TokenMetadata,
    creator_id: AccountId,
    tokens: UnorderedSet<TokenId>,
    price: Option<Balance>,
    is_mintable: bool,
    royalty: HashMap<AccountId, u32>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenSeriesJson {
    token_series_id: TokenSeriesId,
    metadata: TokenMetadata,
    creator_id: AccountId,
    royalty: HashMap<AccountId, u32>,
    transaction_fee: U128,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TransactionFee {
    pub next_fee: Option<u16>,
    pub start_time: Option<TimestampSec>,
    pub current_fee: u16,
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct MarketDataTransactionFee {
    pub transaction_fee: UnorderedMap<TokenSeriesId, u128>,
}

near_sdk::setup_alloc!();

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ContractV1 {
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
    // CUSTOM
    token_series_by_id: UnorderedMap<TokenSeriesId, TokenSeries>,
    treasury_id: AccountId,
    transaction_fee: TransactionFee,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
    // CUSTOM
    token_series_by_id: UnorderedMap<TokenSeriesId, TokenSeries>,
    treasury_id: AccountId,
    transaction_fee: TransactionFee,
    market_data_transaction_fee: MarketDataTransactionFee,
}

const DATA_IMAGE_SVG_ICON: &str = "data:image/svg+xml,%3Csvg width='383' height='383' viewBox='0 0 383 383' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Ccircle cx='191.5' cy='191.5' r='191.5' fill='black'/%3E%3Cmask id='mask0_961_8519' style='mask-type:alpha' maskUnits='userSpaceOnUse' x='75' y='76' width='235' height='232'%3E%3Cpath d='M105.742 76.6C98.4934 77.4237 92.7276 79.6476 88.2386 83.2306C81.4844 88.6669 77.4895 95.6682 76.3364 104.029C75.7186 108.476 75.7186 280.42 76.3364 284.991C77.4483 293.475 82.1845 300.229 89.4741 303.771C96.9696 307.437 105.618 307.478 112.99 303.895C117.15 301.836 118.591 300.147 129.67 283.879C131.152 281.738 133.829 277.825 135.641 275.231C137.453 272.636 139.636 269.465 140.501 268.229C141.325 266.994 142.684 265.017 143.466 263.905C146.432 259.704 148.985 255.998 150.632 253.609C151.58 252.25 153.186 249.944 154.215 248.461C156.604 245.002 161.587 237.836 165.17 232.605C166.735 230.34 168.877 227.169 169.989 225.604C171.595 223.256 171.966 222.392 171.966 220.95C171.966 218.479 170.112 216.543 167.724 216.543C165.747 216.543 165.294 216.873 153.721 226.222C152.939 226.839 147.461 231.205 141.572 235.9C135.683 240.595 130.205 244.96 129.423 245.578C128.64 246.196 125.304 248.873 122.009 251.509C118.715 254.144 115.914 256.286 115.791 256.286C115.626 256.286 115.42 255.709 115.255 255.009C114.926 253.403 114.926 131.745 115.255 130.881C115.379 130.551 115.667 130.263 115.832 130.263C116.038 130.263 118.097 132.528 120.362 135.287C122.668 138.088 124.975 140.806 125.469 141.383C125.922 141.959 127.487 143.812 128.887 145.501C130.287 147.189 135.435 153.408 140.336 159.298C145.237 165.187 150.385 171.406 151.786 173.094C153.186 174.783 154.751 176.636 155.204 177.213C155.698 177.789 158.622 181.29 161.752 185.038C164.841 188.785 167.683 192.204 168.053 192.657C168.424 193.11 170.607 195.704 172.831 198.422C175.096 201.141 177.773 204.394 178.843 205.63C179.914 206.865 182.303 209.748 184.156 212.013C186.051 214.278 188.275 216.955 189.139 217.985C190.004 219.014 194.04 223.915 198.2 228.899C202.318 233.882 206.107 238.412 206.601 238.989C207.055 239.565 208.619 241.419 210.02 243.107C211.42 244.796 216.527 250.932 221.345 256.739C226.164 262.546 233.453 271.359 237.531 276.26C241.608 281.202 245.232 285.568 245.562 285.897C245.891 286.268 246.921 287.503 247.827 288.615C252.233 294.093 256.558 298.623 259.564 300.971C262.612 303.318 268.13 305.913 271.919 306.778C278.509 308.301 284.645 307.519 291.07 304.43C300.707 299.776 307.049 290.716 308.532 279.39C309.15 274.819 309.15 102.175 308.573 97.9333C307.255 88.6257 301.943 81.9127 293.17 78.4945C290.452 77.4237 289.834 77.3413 284.275 77.3413C278.426 77.3413 278.221 77.3825 275.091 78.7416C269.407 81.1714 266.771 83.6013 261.417 91.3439C260.305 92.9089 258.74 95.1328 257.958 96.286C257.175 97.3979 255.734 99.4983 254.746 100.94C253.757 102.34 252.316 104.44 251.533 105.552C250.71 106.706 248.733 109.63 247.085 112.059C245.397 114.489 243.091 117.866 241.896 119.555C240.702 121.243 238.56 124.415 237.078 126.556C235.636 128.698 232.918 132.61 231.106 135.205C229.294 137.799 227.111 140.971 226.246 142.206C225.423 143.442 223.816 145.748 222.704 147.354C221.592 148.919 219.533 152.008 218.051 154.15C216.609 156.291 215.168 158.35 214.879 158.68C214.591 159.009 214.138 160.039 213.85 160.945C213.438 162.345 213.479 162.881 213.973 164.157C214.756 165.969 216.815 167.328 218.792 167.328C220.522 167.287 221.757 166.464 231.435 158.639C235.224 155.55 240.043 151.679 242.061 150.031C244.12 148.343 249.021 144.389 252.975 141.177C266.565 130.18 268.542 128.615 268.995 128.615C269.242 128.615 269.531 129.027 269.654 129.521C269.778 130.057 269.819 157.897 269.778 191.38C269.695 236.682 269.531 252.332 269.201 252.456C268.789 252.579 263.847 246.855 251.204 231.576C249.804 229.887 248.239 228.034 247.786 227.457C247.291 226.881 244.367 223.38 241.237 219.632C238.148 215.884 235.307 212.466 234.936 212.013C234.565 211.56 233.412 210.16 232.383 208.924C231.353 207.689 230.118 206.206 229.665 205.63C229.17 205.053 226.246 201.552 223.116 197.805C220.027 194.057 217.186 190.639 216.815 190.186C216.444 189.733 215.291 188.332 214.262 187.097C212.038 184.42 212.038 184.42 207.26 178.654C202.401 172.847 202.442 172.888 200.259 170.211C199.23 168.976 197.994 167.493 197.541 166.917C197.047 166.34 194.123 162.839 190.993 159.092C187.904 155.344 185.062 151.926 184.692 151.473C184.321 151.02 183.168 149.619 182.138 148.384C181.109 147.148 179.873 145.666 179.42 145.089C178.72 144.265 172.501 136.811 166.571 129.645C161.052 123.014 150.303 110.041 147.955 107.2C147.091 106.17 144.867 103.493 142.972 101.228C129.134 84.5485 127.858 83.2306 122.71 80.4713C117.809 77.8355 110.107 76.0646 105.742 76.6Z' fill='black'/%3E%3C/mask%3E%3Cg mask='url(%23mask0_961_8519)'%3E%3Crect x='75.873' y='76.5044' width='233.222' height='231.001' fill='url(%23paint0_linear_961_8519)'/%3E%3C/g%3E%3Cdefs%3E%3ClinearGradient id='paint0_linear_961_8519' x1='309.095' y1='192.005' x2='75.873' y2='192.005' gradientUnits='userSpaceOnUse'%3E%3Cstop stop-color='%23FFFC57'/%3E%3Cstop offset='1' stop-color='%2379E6EE'/%3E%3C/linearGradient%3E%3C/defs%3E%3C/svg%3E%0A";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
    // CUSTOM
    TokenSeriesById,
    TokensBySeriesInner { token_series: String },
    TokensPerOwner { account_hash: Vec<u8> },
    MarketDataTransactionFee,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new_default_meta(owner_id: ValidAccountId, treasury_id: ValidAccountId) -> Self {
        Self::new(
            owner_id,
            treasury_id,
            NFTContractMetadata {
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Look Tab Collectibles".to_string(),
                symbol: "LTAB".to_string(),
                icon: Some(DATA_IMAGE_SVG_ICON.to_string()),
                base_uri: None,
                reference: None,
                reference_hash: None,
            },
            500,
        )
    }

    #[init]
    pub fn new(
        owner_id: ValidAccountId,
        treasury_id: ValidAccountId,
        metadata: NFTContractMetadata,
        current_fee: u16,
    ) -> Self {
        metadata.assert_valid();
        Self {
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            token_series_by_id: UnorderedMap::new(StorageKey::TokenSeriesById),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            treasury_id: treasury_id.to_string(),
            transaction_fee: TransactionFee {
                next_fee: None,
                start_time: None,
                current_fee,
            },
            market_data_transaction_fee: MarketDataTransactionFee {
                transaction_fee: UnorderedMap::new(StorageKey::MarketDataTransactionFee),
            },
        }
    }

    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let prev: ContractV1 = env::state_read().expect("ERR_NOT_INITIALIZED");
        assert_eq!(
            env::predecessor_account_id(),
            prev.tokens.owner_id,
            "Error: Only owner"
        );

        let this = Contract {
            tokens: prev.tokens,
            metadata: prev.metadata,
            token_series_by_id: prev.token_series_by_id,
            treasury_id: prev.treasury_id,
            transaction_fee: prev.transaction_fee,
            market_data_transaction_fee: MarketDataTransactionFee {
                transaction_fee: UnorderedMap::new(StorageKey::MarketDataTransactionFee),
            },
        };

        this
    }

    #[payable]
    pub fn set_transaction_fee(&mut self, next_fee: u16, start_time: Option<TimestampSec>) {
        assert_one_yocto();
        assert_eq!(
            env::predecessor_account_id(),
            self.tokens.owner_id,
            "Error: Owner only"
        );

        assert!(
            next_fee < 10_000,
            "Error: transaction fee is more than 10_000"
        );

        if start_time.is_none() {
            self.transaction_fee.current_fee = next_fee;
            self.transaction_fee.next_fee = None;
            self.transaction_fee.start_time = None;
            return;
        } else {
            let start_time: TimestampSec = start_time.unwrap();
            assert!(
                start_time > to_sec(env::block_timestamp()),
                "start_time is less than current block_timestamp"
            );
            self.transaction_fee.next_fee = Some(next_fee);
            self.transaction_fee.start_time = Some(start_time);
        }
    }

    pub fn calculate_new_market_data_transaction_fee(
        &mut self,
        token_series_id: &TokenSeriesId,
    ) -> u128 {
        if let Some(transaction_fee) = self
            .market_data_transaction_fee
            .transaction_fee
            .get(&token_series_id)
        {
            return transaction_fee;
        }

        self.calculate_current_transaction_fee()
    }

    pub fn calculate_current_transaction_fee(&mut self) -> u128 {
        let transaction_fee: &TransactionFee = &self.transaction_fee;
        if transaction_fee.next_fee.is_some() {
            if to_sec(env::block_timestamp()) >= transaction_fee.start_time.unwrap() {
                self.transaction_fee.current_fee = transaction_fee.next_fee.unwrap();
                self.transaction_fee.next_fee = None;
                self.transaction_fee.start_time = None;
            }
        }
        self.transaction_fee.current_fee as u128
    }

    pub fn get_transaction_fee(&self) -> &TransactionFee {
        &self.transaction_fee
    }

    pub fn get_market_data_transaction_fee(&self, token_series_id: &TokenSeriesId) -> u128 {
        if let Some(transaction_fee) = self
            .market_data_transaction_fee
            .transaction_fee
            .get(&token_series_id)
        {
            return transaction_fee;
        }
        // fallback to default transaction fee
        self.transaction_fee.current_fee as u128
    }

    // Treasury
    #[payable]
    pub fn set_treasury(&mut self, treasury_id: ValidAccountId) {
        assert_one_yocto();
        assert_eq!(
            env::predecessor_account_id(),
            self.tokens.owner_id,
            "Error: Owner only"
        );
        self.treasury_id = treasury_id.to_string();
    }

    // CUSTOM

    #[payable]
    pub fn nft_create_series(
        &mut self,
        token_metadata: TokenMetadata,
        price: Option<U128>,
        royalty: Option<HashMap<AccountId, u32>>,
        token_series_id: String,
    ) -> TokenSeriesJson {
        let initial_storage_usage = env::storage_usage();
        let caller_id = env::predecessor_account_id();

        // let token_series_id = format!("{}", (self.token_series_by_id.len() + 1));
        assert!(token_series_id.chars().count()>0, "Error: token_series_id is required");

        assert!(
            self.token_series_by_id.get(&token_series_id).is_none(),
            "Error: duplicate token_series_id"
        );
        

        let title = token_metadata.title.clone();
        assert!(title.is_some(), "Error: token_metadata.title is required");

        let mut total_perpetual = 0;
        let mut total_accounts = 0;
        let royalty_res: HashMap<AccountId, u32> = if let Some(royalty) = royalty {
            for (k, v) in royalty.iter() {
                if !is_valid_account_id(k.as_bytes()) {
                    env::panic("Not valid account_id for royalty".as_bytes());
                };
                total_perpetual += *v;
                total_accounts += 1;
            }
            royalty
        } else {
            HashMap::new()
        };

        assert!(total_accounts <= 50, "Error: royalty exceeds 50 accounts");

        assert!(
            total_perpetual <= 9000,
            "Error Exceeds maximum royalty -> 9000",
        );

        let price_res: Option<u128> = if price.is_some() {
            assert!(
                price.unwrap().0 < MAX_PRICE,
                "Error: price higher than {}",
                MAX_PRICE
            );
            Some(price.unwrap().0)
        } else {
            None
        };

        self.token_series_by_id.insert(
            &token_series_id,
            &TokenSeries {
                metadata: token_metadata.clone(),
                creator_id: caller_id.to_string(),
                tokens: UnorderedSet::new(
                    StorageKey::TokensBySeriesInner {
                        token_series: token_series_id.clone(),
                    }
                    .try_to_vec()
                    .unwrap(),
                ),
                price: price_res,
                is_mintable: true,
                royalty: royalty_res.clone(),
            },
        );

        // set market data transaction fee
        let current_transaction_fee = self.calculate_current_transaction_fee();
        self.market_data_transaction_fee
            .transaction_fee
            .insert(&token_series_id, &current_transaction_fee);

        env::log(
            json!({
                "type": "nft_create_series",
                "params": {
                    "token_series_id": token_series_id,
                    "token_metadata": token_metadata,
                    "creator_id": caller_id,
                    "price": price,
                    "royalty": royalty_res,
                    "transaction_fee": &current_transaction_fee.to_string()
                }
            })
            .to_string()
            .as_bytes(),
        );

        refund_deposit(env::storage_usage() - initial_storage_usage, 0);

        TokenSeriesJson {
            token_series_id,
            metadata: token_metadata,
            creator_id: caller_id.into(),
            royalty: royalty_res,
            transaction_fee: current_transaction_fee.into(),
        }
    }

    #[payable]
    pub fn nft_mint(
        &mut self,
        token_series_id: TokenSeriesId,
        receiver_id: ValidAccountId,
    ) -> TokenId {
        let initial_storage_usage = env::storage_usage();

        let token_series = self
            .token_series_by_id
            .get(&token_series_id)
            .expect("Error: Token series not exist");
        assert_eq!(
            env::predecessor_account_id(),
            token_series.creator_id,
            "Error: not creator"
        );
        let token_id: TokenId = self._nft_mint_series(token_series_id, receiver_id.to_string());

        refund_deposit(env::storage_usage() - initial_storage_usage, 0);

        NearEvent::log_nft_mint(receiver_id.to_string(), vec![token_id.clone()], None);

        token_id
    }

    #[payable]
    pub fn nft_mint_and_approve(
        &mut self,
        token_series_id: TokenSeriesId,
        account_id: ValidAccountId,
        msg: Option<String>,
    ) -> Option<Promise> {
        let initial_storage_usage = env::storage_usage();

        let token_series = self
            .token_series_by_id
            .get(&token_series_id)
            .expect("Error: Token series not exist");
        assert_eq!(
            env::predecessor_account_id(),
            token_series.creator_id,
            "Error: not creator"
        );
        let token_id: TokenId =
            self._nft_mint_series(token_series_id, token_series.creator_id.clone());

        // Need to copy the nft_approve code here to solve the gas problem
        // get contract-level LookupMap of token_id to approvals HashMap
        let approvals_by_id = self.tokens.approvals_by_id.as_mut().unwrap();

        // update HashMap of approvals for this token
        let approved_account_ids = &mut approvals_by_id
            .get(&token_id)
            .unwrap_or_else(|| HashMap::new());
        let account_id: AccountId = account_id.into();
        let approval_id: u64 = self
            .tokens
            .next_approval_id_by_id
            .as_ref()
            .unwrap()
            .get(&token_id)
            .unwrap_or_else(|| 1u64);
        approved_account_ids.insert(account_id.clone(), approval_id);

        // save updated approvals HashMap to contract's LookupMap
        approvals_by_id.insert(&token_id, &approved_account_ids);

        // increment next_approval_id for this token
        self.tokens
            .next_approval_id_by_id
            .as_mut()
            .unwrap()
            .insert(&token_id, &(approval_id + 1));

        refund_deposit(env::storage_usage() - initial_storage_usage, 0);

        NearEvent::log_nft_mint(
            token_series.creator_id.clone(),
            vec![token_id.clone()],
            None,
        );

        if let Some(msg) = msg {
            Some(ext_approval_receiver::nft_on_approve(
                token_id,
                token_series.creator_id,
                approval_id,
                msg,
                &account_id,
                NO_DEPOSIT,
                env::prepaid_gas() - GAS_FOR_NFT_APPROVE - GAS_FOR_MINT,
            ))
        } else {
            None
        }
    }

    fn _nft_mint_series(
        &mut self,
        token_series_id: TokenSeriesId,
        receiver_id: AccountId,
    ) -> TokenId {
        let mut token_series = self
            .token_series_by_id
            .get(&token_series_id)
            .expect("Error: Token series not exist");
        assert!(
            token_series.is_mintable,
            "Error: Token series is not mintable"
        );

        let num_tokens = token_series.tokens.len();
        let max_copies = token_series.metadata.copies.unwrap_or(u64::MAX);
        assert!(num_tokens < max_copies, "Series supply maxed");

        if (num_tokens + 1) >= max_copies {
            token_series.is_mintable = false;
            token_series.price = None;
        }

        let token_id = format!("{}{}{}", &token_series_id, TOKEN_DELIMETER, num_tokens + 1);
        token_series.tokens.insert(&token_id);
        self.token_series_by_id
            .insert(&token_series_id, &token_series);

        // you can add custom metadata to each token here
        let metadata = Some(TokenMetadata {
            title: None,       // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
            description: None, // free-form description
            media: None, // URL to associated media, preferably to decentralized, content-addressed storage
            media_hash: None, // Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
            copies: None, // number of copies of this set of metadata in existence when token was minted.
            issued_at: Some(env::block_timestamp().to_string()), // ISO 8601 datetime when token was issued or minted
            expires_at: None,     // ISO 8601 datetime when token expires
            starts_at: None,      // ISO 8601 datetime when token starts being valid
            updated_at: None,     // ISO 8601 datetime when token was last updated
            extra: None, // anything extra the NFT wants to store on-chain. Can be stringified JSON.
            reference: None, // URL to an off-chain JSON file with more info.
            reference_hash: None, // Base64-encoded sha256 hash of JSON from reference field. Required if `reference` is included.
        });

        //let token = self.tokens.mint(token_id, receiver_id, metadata);
        // From : https://github.com/near/near-sdk-rs/blob/master/near-contract-standards/src/non_fungible_token/core/core_impl.rs#L359
        // This allows lazy minting

        let owner_id: AccountId = receiver_id;
        self.tokens.owner_by_id.insert(&token_id, &owner_id);

        self.tokens
            .token_metadata_by_id
            .as_mut()
            .and_then(|by_id| by_id.insert(&token_id, &metadata.as_ref().unwrap()));

        if let Some(tokens_per_owner) = &mut self.tokens.tokens_per_owner {
            let mut token_ids = tokens_per_owner.get(&owner_id).unwrap_or_else(|| {
                UnorderedSet::new(StorageKey::TokensPerOwner {
                    account_hash: env::sha256(&owner_id.as_bytes()),
                })
            });
            token_ids.insert(&token_id);
            tokens_per_owner.insert(&owner_id, &token_ids);
        }

        token_id
    }

    #[payable]
    pub fn nft_decrease_series_copies(
        &mut self,
        token_series_id: TokenSeriesId,
        decrease_copies: U64,
    ) -> U64 {
        assert_one_yocto();

        let mut token_series = self
            .token_series_by_id
            .get(&token_series_id)
            .expect("Token series not exist");
        assert_eq!(
            env::predecessor_account_id(),
            token_series.creator_id,
            "Error: Creator only"
        );

        let minted_copies = token_series.tokens.len();
        let copies = token_series.metadata.copies.unwrap();

        assert!(
            (copies - decrease_copies.0) >= minted_copies,
            "Error: cannot decrease supply, already minted : {}",
            minted_copies
        );

        let is_non_mintable = if (copies - decrease_copies.0) == minted_copies {
            token_series.is_mintable = false;
            token_series.price = None;

            env::log(
                json!({
                    "type": "nft_set_series_price",
                    "params": {
                        "token_series_id": token_series_id,
                        "price": Null,
                    }
                })
                .to_string()
                .as_bytes(),
            );

            true
        } else {
            false
        };

        token_series.metadata.copies = Some(copies - decrease_copies.0);

        self.token_series_by_id
            .insert(&token_series_id, &token_series);
        env::log(
            json!({
                "type": "nft_decrease_series_copies",
                "params": {
                    "token_series_id": token_series_id,
                    "copies": U64::from(token_series.metadata.copies.unwrap()),
                    "is_non_mintable": is_non_mintable,
                }
            })
            .to_string()
            .as_bytes(),
        );
        U64::from(token_series.metadata.copies.unwrap())
    }

    #[payable]
    pub fn nft_set_series_price(
        &mut self,
        token_series_id: TokenSeriesId,
        price: Option<U128>,
    ) -> Option<U128> {
        assert_one_yocto();

        let mut token_series = self
            .token_series_by_id
            .get(&token_series_id)
            .expect("Token series not exist");
        assert_eq!(
            env::predecessor_account_id(),
            token_series.creator_id,
            "Error: Creator only"
        );

        assert_eq!(
            token_series.is_mintable, true,
            "Error: token series is not mintable"
        );

        if price.is_none() {
            token_series.price = None;
        } else {
            assert!(
                price.unwrap().0 < MAX_PRICE,
                "Error: price higher than {}",
                MAX_PRICE
            );
            token_series.price = Some(price.unwrap().0);
        }

        self.token_series_by_id
            .insert(&token_series_id, &token_series);

        // set market data transaction fee
        let current_transaction_fee = self.calculate_current_transaction_fee();
        self.market_data_transaction_fee
            .transaction_fee
            .insert(&token_series_id, &current_transaction_fee);

        env::log(
            json!({
                "type": "nft_set_series_price",
                "params": {
                    "token_series_id": token_series_id,
                    "price": price,
                    "transaction_fee": current_transaction_fee.to_string()
                }
            })
            .to_string()
            .as_bytes(),
        );
        return price;
    }

    #[payable]
    pub fn nft_burn(&mut self, token_id: TokenId) {
        assert_one_yocto();

        let owner_id = self.tokens.owner_by_id.get(&token_id).unwrap();
        assert_eq!(owner_id, env::predecessor_account_id(), "Token owner only");

        if let Some(next_approval_id_by_id) = &mut self.tokens.next_approval_id_by_id {
            next_approval_id_by_id.remove(&token_id);
        }

        if let Some(approvals_by_id) = &mut self.tokens.approvals_by_id {
            approvals_by_id.remove(&token_id);
        }

        if let Some(tokens_per_owner) = &mut self.tokens.tokens_per_owner {
            let mut token_ids = tokens_per_owner.get(&owner_id).unwrap();
            token_ids.remove(&token_id);

            // remove the owner if there are no more tokens
            if token_ids.is_empty() {
                tokens_per_owner.remove(&owner_id);
            } else {
                tokens_per_owner.insert(&owner_id, &token_ids);
            }
        }

        if let Some(token_metadata_by_id) = &mut self.tokens.token_metadata_by_id {
            token_metadata_by_id.remove(&token_id);
        }

        self.tokens.owner_by_id.remove(&token_id);

        NearEvent::log_nft_burn(owner_id, vec![token_id], None, None);
    }

    // CUSTOM VIEWS

    pub fn nft_get_series_single(&self, token_series_id: TokenSeriesId) -> TokenSeriesJson {
        let token_series = self
            .token_series_by_id
            .get(&token_series_id)
            .expect("Series does not exist");
        let current_transaction_fee = self.get_market_data_transaction_fee(&token_series_id);
        TokenSeriesJson {
            token_series_id,
            metadata: token_series.metadata,
            creator_id: token_series.creator_id,
            royalty: token_series.royalty,
            transaction_fee: current_transaction_fee.into(),
        }
    }

    pub fn nft_get_series_format(self) -> (char, &'static str, &'static str) {
        (TOKEN_DELIMETER, TITLE_DELIMETER, EDITION_DELIMETER)
    }

    pub fn nft_get_series_price(self, token_series_id: TokenSeriesId) -> Option<U128> {
        let price = self.token_series_by_id.get(&token_series_id).unwrap().price;
        match price {
            Some(p) => return Some(U128::from(p)),
            None => return None,
        };
    }

    pub fn nft_get_series(
        &self,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<TokenSeriesJson> {
        let start_index: u128 = from_index.map(From::from).unwrap_or_default();
        assert!(
            (self.token_series_by_id.len() as u128) > start_index,
            "Out of bounds, please use a smaller from_index."
        );
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        assert_ne!(limit, 0, "Cannot provide limit of 0.");

        self.token_series_by_id
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|(token_series_id, token_series)| {
                let current_transaction_fee =
                    self.get_market_data_transaction_fee(&token_series_id);
                return TokenSeriesJson {
                    token_series_id,
                    metadata: token_series.metadata,
                    creator_id: token_series.creator_id,
                    royalty: token_series.royalty,
                    transaction_fee: current_transaction_fee.into(),
                };
            })
            .collect()
    }

    pub fn nft_supply_for_series(&self, token_series_id: TokenSeriesId) -> U64 {
        self.token_series_by_id
            .get(&token_series_id)
            .expect("Token series not exist")
            .tokens
            .len()
            .into()
    }

    pub fn nft_tokens_by_series(
        &self,
        token_series_id: TokenSeriesId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<Token> {
        let start_index: u128 = from_index.map(From::from).unwrap_or_default();
        let tokens = self
            .token_series_by_id
            .get(&token_series_id)
            .unwrap()
            .tokens;
        assert!(
            (tokens.len() as u128) > start_index,
            "Out of bounds, please use a smaller from_index."
        );
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        assert_ne!(limit, 0, "Cannot provide limit of 0.");

        tokens
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|token_id| self.nft_token(token_id).unwrap())
            .collect()
    }

    pub fn nft_token(&self, token_id: TokenId) -> Option<Token> {
        let owner_id = self.tokens.owner_by_id.get(&token_id)?;
        let approved_account_ids = self
            .tokens
            .approvals_by_id
            .as_ref()
            .and_then(|by_id| by_id.get(&token_id).or_else(|| Some(HashMap::new())));

        // CUSTOM (switch metadata for the token_series metadata)
        let mut token_id_iter = token_id.split(TOKEN_DELIMETER);
        let token_series_id = token_id_iter.next().unwrap().parse().unwrap();
        let series_metadata = self
            .token_series_by_id
            .get(&token_series_id)
            .unwrap()
            .metadata;

        let mut token_metadata = self
            .tokens
            .token_metadata_by_id
            .as_ref()
            .unwrap()
            .get(&token_id)
            .unwrap();

        token_metadata.title = Some(format!(
            "{}{}{}",
            series_metadata.title.unwrap(),
            TITLE_DELIMETER,
            token_id_iter.next().unwrap()
        ));

        token_metadata.reference = series_metadata.reference;
        token_metadata.media = series_metadata.media;
        token_metadata.copies = series_metadata.copies;
        token_metadata.extra = series_metadata.extra;
        token_metadata.description = series_metadata.description;

        Some(Token {
            token_id,
            owner_id,
            metadata: Some(token_metadata),
            approved_account_ids,
        })
    }

    #[payable]
    pub fn nft_transfer(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) {
        let sender_id = env::predecessor_account_id();
        let previous_owner_id = self
            .tokens
            .owner_by_id
            .get(&token_id)
            .expect("Token not found");
        let receiver_id_str = receiver_id.to_string();
        self.tokens
            .nft_transfer(receiver_id, token_id.clone(), approval_id, memo.clone());

        let authorized_id: Option<AccountId> = if sender_id != previous_owner_id {
            Some(sender_id)
        } else {
            None
        };

        NearEvent::log_nft_transfer(
            previous_owner_id,
            receiver_id_str,
            vec![token_id],
            memo,
            authorized_id,
        );
    }

    #[payable]
    pub fn nft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<bool> {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let (previous_owner_id, old_approvals) = self.tokens.internal_transfer(
            &sender_id,
            receiver_id.as_ref(),
            &token_id,
            approval_id,
            memo.clone(),
        );

        let authorized_id: Option<AccountId> = if sender_id != previous_owner_id {
            Some(sender_id.clone())
        } else {
            None
        };

        NearEvent::log_nft_transfer(
            previous_owner_id.clone(),
            receiver_id.to_string(),
            vec![token_id.clone()],
            memo,
            authorized_id,
        );

        // Initiating receiver's call and the callback
        ext_non_fungible_token_receiver::nft_on_transfer(
            sender_id,
            previous_owner_id.clone(),
            token_id.clone(),
            msg,
            receiver_id.as_ref(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_NFT_TRANSFER_CALL,
        )
        .then(ext_self::nft_resolve_transfer(
            previous_owner_id,
            receiver_id.into(),
            token_id,
            old_approvals,
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
        .into()
    }

    pub fn nft_total_supply(&self) -> U128 {
        (self.tokens.owner_by_id.len() as u128).into()
    }

    pub fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token> {
        // Get starting index, whether or not it was explicitly given.
        // Defaults to 0 based on the spec:
        let start_index: u128 = from_index.map(From::from).unwrap_or_default();
        assert!(
            (self.tokens.owner_by_id.len() as u128) > start_index,
            "Out of bounds, please use a smaller from_index."
        );
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        assert_ne!(limit, 0, "Cannot provide limit of 0.");
        self.tokens
            .owner_by_id
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|(token_id, _)| self.nft_token(token_id).unwrap())
            .collect()
    }

    pub fn nft_supply_for_owner(self, account_id: ValidAccountId) -> U128 {
        let tokens_per_owner = self.tokens.tokens_per_owner.expect(
            "Could not find tokens_per_owner when calling a method on the enumeration standard.",
        );
        tokens_per_owner
            .get(account_id.as_ref())
            .map(|account_tokens| U128::from(account_tokens.len() as u128))
            .unwrap_or(U128(0))
    }

    pub fn nft_tokens_for_owner(
        &self,
        account_id: ValidAccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<Token> {
        let tokens_per_owner = self.tokens.tokens_per_owner.as_ref().expect(
            "Could not find tokens_per_owner when calling a method on the enumeration standard.",
        );
        let token_set = if let Some(token_set) = tokens_per_owner.get(account_id.as_ref()) {
            token_set
        } else {
            return vec![];
        };
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        assert_ne!(limit, 0, "Cannot provide limit of 0.");
        let start_index: u128 = from_index.map(From::from).unwrap_or_default();
        assert!(
            token_set.len() as u128 > start_index,
            "Out of bounds, please use a smaller from_index."
        );
        token_set
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|token_id| self.nft_token(token_id).unwrap())
            .collect()
    }

    pub fn nft_payout(&self, token_id: TokenId, balance: U128, max_len_payout: u32) -> Payout {
        let owner_id = self.tokens.owner_by_id.get(&token_id).expect("No token id");
        let mut token_id_iter = token_id.split(TOKEN_DELIMETER);
        let token_series_id = token_id_iter.next().unwrap().parse().unwrap();
        let royalty = self
            .token_series_by_id
            .get(&token_series_id)
            .expect("no type")
            .royalty;

        assert!(
            royalty.len() as u32 <= max_len_payout,
            "Market cannot payout to that many receivers"
        );

        let balance_u128: u128 = balance.into();

        let mut payout: Payout = Payout {
            payout: HashMap::new(),
        };
        let mut total_perpetual = 0;

        for (k, v) in royalty.iter() {
            if *k != owner_id {
                let key = k.clone();
                payout
                    .payout
                    .insert(key, royalty_to_payout(*v, balance_u128));
                total_perpetual += *v;
            }
        }
        payout.payout.insert(
            owner_id,
            royalty_to_payout(10000 - total_perpetual, balance_u128),
        );
        payout
    }

    #[payable]
    pub fn nft_transfer_payout(
        &mut self,
        receiver_id: ValidAccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        balance: Option<U128>,
        max_len_payout: Option<u32>,
    ) -> Option<Payout> {
        assert_one_yocto();

        let sender_id = env::predecessor_account_id();
        // Transfer
        let previous_token = self.nft_token(token_id.clone()).expect("no token");
        self.tokens
            .nft_transfer(receiver_id.clone(), token_id.clone(), approval_id, None);

        // Payout calculation
        let previous_owner_id = previous_token.owner_id;
        let mut total_perpetual = 0;
        let payout = if let Some(balance) = balance {
            let balance_u128: u128 = u128::from(balance);
            let mut payout: Payout = Payout {
                payout: HashMap::new(),
            };

            let mut token_id_iter = token_id.split(TOKEN_DELIMETER);
            let token_series_id = token_id_iter.next().unwrap().parse().unwrap();
            let royalty = self
                .token_series_by_id
                .get(&token_series_id)
                .expect("no type")
                .royalty;

            assert!(
                royalty.len() as u32 <= max_len_payout.unwrap(),
                "Market cannot payout to that many receivers"
            );
            for (k, v) in royalty.iter() {
                let key = k.clone();
                if key != previous_owner_id {
                    payout
                        .payout
                        .insert(key, royalty_to_payout(*v, balance_u128));
                    total_perpetual += *v;
                }
            }

            assert!(total_perpetual <= 10000, "Total payout overflow");

            payout.payout.insert(
                previous_owner_id.clone(),
                royalty_to_payout(10000 - total_perpetual, balance_u128),
            );
            Some(payout)
        } else {
            None
        };

        let authorized_id: Option<AccountId> = if sender_id != previous_owner_id {
            Some(sender_id)
        } else {
            None
        };

        NearEvent::log_nft_transfer(
            previous_owner_id,
            receiver_id.to_string(),
            vec![token_id],
            None,
            authorized_id,
        );

        payout
    }

    pub fn get_owner(&self) -> AccountId {
        self.tokens.owner_id.clone()
    }
}

fn royalty_to_payout(a: u32, b: Balance) -> U128 {
    U128(a as u128 * b / 10_000u128)
}

// near_contract_standards::impl_non_fungible_token_core!(Contract, tokens);
// near_contract_standards::impl_non_fungible_token_enumeration!(Contract, tokens);
near_contract_standards::impl_non_fungible_token_approval!(Contract, tokens);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Contract {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}

#[near_bindgen]
impl NonFungibleTokenResolver for Contract {
    #[private]
    fn nft_resolve_transfer(
        &mut self,
        previous_owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        approved_account_ids: Option<HashMap<AccountId, u64>>,
    ) -> bool {
        let resp: bool = self.tokens.nft_resolve_transfer(
            previous_owner_id.clone(),
            receiver_id.clone(),
            token_id.clone(),
            approved_account_ids,
        );

        // if not successful, return nft back to original owner
        if !resp {
            NearEvent::log_nft_transfer(receiver_id, previous_owner_id, vec![token_id], None, None);
        }

        resp
    }
}

/// from https://github.com/near/near-sdk-rs/blob/e4abb739ff953b06d718037aa1b8ab768db17348/near-contract-standards/src/non_fungible_token/utils.rs#L29

fn refund_deposit(storage_used: u64, extra_spend: Balance) {
    let required_cost = env::storage_byte_cost() * Balance::from(storage_used);
    let attached_deposit = env::attached_deposit() - extra_spend;

    assert!(
        required_cost <= attached_deposit,
        "Must attach {} yoctoNEAR to cover storage",
        required_cost,
    );

    let refund = attached_deposit - required_cost;
    if refund > 1 {
        Promise::new(env::predecessor_account_id()).transfer(refund);
    }
}

fn to_sec(timestamp: Timestamp) -> TimestampSec {
    (timestamp / 10u64.pow(9)) as u32
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use near_sdk::MockedBlockchain;

    const STORAGE_FOR_CREATE_SERIES: Balance = 8540000000000000000000;
    const STORAGE_FOR_MINT: Balance = 11280000000000000000000;

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    fn setup_contract() -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let contract = Contract::new_default_meta(accounts(0), accounts(4));
        (context, contract)
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new(
            accounts(1),
            accounts(4),
            NFTContractMetadata {
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Triple Triad".to_string(),
                symbol: "TRIAD".to_string(),
                icon: None,
                base_uri: None,
                reference: None,
                reference_hash: None,
            },
            500,
        );
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.get_owner(), accounts(1).to_string());
    }

    fn create_series(
        contract: &mut Contract,
        royalty: &HashMap<AccountId, u32>,
        price: Option<U128>,
        copies: Option<u64>,
        
    ) {
        contract.nft_create_series(
            TokenMetadata {
                title: Some("Tester go".to_string()),
                description: Some("Hello tester".to_string()),
                media: Some(
                    "bafybeidzcan4nzcz7sczs4yzyxly4galgygnbjewipj6haco4kffoqpkiy".to_string(),
                ),
                media_hash: None,
                copies: copies,
                issued_at: None,
                expires_at: None,
                starts_at: None,
                updated_at: None,
                extra: None,
                reference: Some("QmVo5mdBiwMjU66AZPzBsqMvMYJt4SDnuaU4AtXjfL3Hou".to_string()),
                reference_hash: None,
            },
            price,
            Some(royalty.clone()),
            "1".to_string()
        );
    }

    #[test]
    fn test_create_series() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_CREATE_SERIES)
            .build());

        let mut royalty: HashMap<AccountId, u32> = HashMap::new();
        royalty.insert(accounts(1).to_string(), 1000);
        create_series(
            &mut contract,
            &royalty,
            Some(U128::from(1 * 10u128.pow(24))),
            None,
        );

        let nft_series_return = contract.nft_get_series_single("1".to_string());
        assert_eq!(nft_series_return.creator_id, accounts(1).to_string());

        assert_eq!(nft_series_return.token_series_id, "1",);

        assert_eq!(nft_series_return.royalty, royalty,);

        assert!(nft_series_return.metadata.copies.is_none());

        assert_eq!(
            nft_series_return.metadata.title.unwrap(),
            "Tester go".to_string()
        );

        assert_eq!(nft_series_return.metadata.description.unwrap(), "Hello tester".to_string(),);

        assert_eq!(
            nft_series_return.metadata.reference.unwrap(),
            "QmVo5mdBiwMjU66AZPzBsqMvMYJt4SDnuaU4AtXjfL3Hou".to_string()
        );
    }

    #[test]
    fn test_mint() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_CREATE_SERIES)
            .build());

        let mut royalty: HashMap<AccountId, u32> = HashMap::new();
        royalty.insert(accounts(1).to_string(), 1000);

        create_series(&mut contract, &royalty, None, None);

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_MINT)
            .build());

        let token_id = contract.nft_mint("1".to_string(), accounts(2));

        let token_from_nft_token = contract.nft_token(token_id);
        assert_eq!(
            token_from_nft_token.unwrap().owner_id,
            accounts(2).to_string()
        )
    }

    #[test]
    #[should_panic(expected = "Error: Token series is not mintable")]
    fn test_invalid_mint_above_copies() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_CREATE_SERIES)
            .build());

        let mut royalty: HashMap<AccountId, u32> = HashMap::new();
        royalty.insert(accounts(1).to_string(), 1000);

        create_series(&mut contract, &royalty, None, Some(1));

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_MINT)
            .build());

        contract.nft_mint("1".to_string(), accounts(2));
        contract.nft_mint("1".to_string(), accounts(2));
    }

    #[test]
    fn test_decrease_copies() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_CREATE_SERIES)
            .build());

        let mut royalty: HashMap<AccountId, u32> = HashMap::new();
        royalty.insert(accounts(1).to_string(), 1000);

        create_series(&mut contract, &royalty, None, Some(5));

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_MINT)
            .build());

        contract.nft_mint("1".to_string(), accounts(2));
        contract.nft_mint("1".to_string(), accounts(2));

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(1)
            .build());

        contract.nft_decrease_series_copies("1".to_string(), U64::from(3));
    }

    #[test]
    #[should_panic(expected = "Error: cannot decrease supply, already minted : 2")]
    fn test_invalid_decrease_copies() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_CREATE_SERIES)
            .build());

        let mut royalty: HashMap<AccountId, u32> = HashMap::new();
        royalty.insert(accounts(1).to_string(), 1000);

        create_series(&mut contract, &royalty, None, Some(5));

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_MINT)
            .build());

        contract.nft_mint("1".to_string(), accounts(2));
        contract.nft_mint("1".to_string(), accounts(2));

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(1)
            .build());

        contract.nft_decrease_series_copies("1".to_string(), U64::from(4));
    }

    #[test]
    #[should_panic(expected = "Error: price higher than 1000000000000000000000000000000000")]
    fn test_invalid_price_shouldnt_be_higher_than_max_price() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_CREATE_SERIES)
            .build());

        let mut royalty: HashMap<AccountId, u32> = HashMap::new();
        royalty.insert(accounts(1).to_string(), 1000);

        create_series(
            &mut contract,
            &royalty,
            Some(U128::from(1_000_000_000 * 10u128.pow(24))),
            None,
        );

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(1)
            .build());
    }

    #[test]
    fn test_nft_burn() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_CREATE_SERIES)
            .build());

        let mut royalty: HashMap<AccountId, u32> = HashMap::new();
        royalty.insert(accounts(1).to_string(), 1000);

        create_series(&mut contract, &royalty, None, None);

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_MINT)
            .build());

        let token_id = contract.nft_mint("1".to_string(), accounts(2));

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(1)
            .build());

        contract.nft_burn(token_id.clone());
        let token = contract.nft_token(token_id);
        assert!(token.is_none());
    }

    #[test]
    fn test_nft_transfer() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_CREATE_SERIES)
            .build());

        let mut royalty: HashMap<AccountId, u32> = HashMap::new();
        royalty.insert(accounts(1).to_string(), 1000);

        create_series(&mut contract, &royalty, None, None);

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_MINT)
            .build());

        let token_id = contract.nft_mint("1".to_string(), accounts(2));

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(1)
            .build());

        contract.nft_transfer(accounts(3), token_id.clone(), None, None);

        let token = contract.nft_token(token_id).unwrap();
        assert_eq!(token.owner_id, accounts(3).to_string())
    }

    #[test]
    fn test_nft_transfer_payout() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_CREATE_SERIES)
            .build());

        let mut royalty: HashMap<AccountId, u32> = HashMap::new();
        royalty.insert(accounts(1).to_string(), 1000);

        create_series(&mut contract, &royalty, None, None);

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(STORAGE_FOR_MINT)
            .build());

        let token_id = contract.nft_mint("1".to_string(), accounts(2));

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(1)
            .build());

        let payout = contract.nft_transfer_payout(
            accounts(3),
            token_id.clone(),
            Some(0),
            Some(U128::from(1 * 10u128.pow(24))),
            Some(10),
        );

        let mut payout_calc: HashMap<AccountId, U128> = HashMap::new();
        payout_calc.insert(
            accounts(1).to_string(),
            U128::from((1000 * (1 * 10u128.pow(24))) / 10_000),
        );
        payout_calc.insert(
            accounts(2).to_string(),
            U128::from((9000 * (1 * 10u128.pow(24))) / 10_000),
        );

        assert_eq!(payout.unwrap().payout, payout_calc);

        let token = contract.nft_token(token_id).unwrap();
        assert_eq!(token.owner_id, accounts(3).to_string())
    }

    #[test]
    fn test_change_transaction_fee_immediately() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.set_transaction_fee(100, None);

        assert_eq!(contract.get_transaction_fee().current_fee, 100);
    }

    #[test]
    fn test_change_transaction_fee_with_time() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        assert_eq!(contract.get_transaction_fee().current_fee, 500);
        assert_eq!(contract.get_transaction_fee().next_fee, None);
        assert_eq!(contract.get_transaction_fee().start_time, None);

        let next_fee: u16 = 100;
        let start_time: Timestamp = 1618109122863866400;
        let start_time_sec: TimestampSec = to_sec(start_time);
        contract.set_transaction_fee(next_fee, Some(start_time_sec));

        assert_eq!(contract.get_transaction_fee().current_fee, 500);
        assert_eq!(contract.get_transaction_fee().next_fee, Some(next_fee));
        assert_eq!(
            contract.get_transaction_fee().start_time,
            Some(start_time_sec)
        );

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .block_timestamp(start_time + 1)
            .build());

        contract.calculate_current_transaction_fee();
        assert_eq!(contract.get_transaction_fee().current_fee, next_fee);
        assert_eq!(contract.get_transaction_fee().next_fee, None);
        assert_eq!(contract.get_transaction_fee().start_time, None);
    }

    #[test]
    fn test_transaction_fee_locked() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        assert_eq!(contract.get_transaction_fee().current_fee, 500);
        assert_eq!(contract.get_transaction_fee().next_fee, None);
        assert_eq!(contract.get_transaction_fee().start_time, None);

        let next_fee: u16 = 100;
        let start_time: Timestamp = 1618109122863866400;
        let start_time_sec: TimestampSec = to_sec(start_time);
        contract.set_transaction_fee(next_fee, Some(start_time_sec));

        let mut royalty: HashMap<AccountId, u32> = HashMap::new();
        royalty.insert(accounts(1).to_string(), 1000);

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(STORAGE_FOR_CREATE_SERIES)
            .build());

        create_series(
            &mut contract,
            &royalty,
            Some(U128::from(1 * 10u128.pow(24))),
            None,
        );

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.nft_set_series_price("1".to_string(), None);

        assert_eq!(contract.get_transaction_fee().current_fee, 500);
        assert_eq!(contract.get_transaction_fee().next_fee, Some(next_fee));
        assert_eq!(
            contract.get_transaction_fee().start_time,
            Some(start_time_sec)
        );

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .block_timestamp(start_time + 1)
            .attached_deposit(1)
            .build());

        contract.calculate_current_transaction_fee();
        assert_eq!(contract.get_transaction_fee().current_fee, next_fee);
        assert_eq!(contract.get_transaction_fee().next_fee, None);
        assert_eq!(contract.get_transaction_fee().start_time, None);

        let series = contract.nft_get_series_single("1".to_string());
        let series_transaction_fee: u128 = series.transaction_fee.0;
        assert_eq!(series_transaction_fee, 500);
    }
}
