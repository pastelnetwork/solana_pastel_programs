use anchor_lang::prelude::*;

declare_id!("HV8WdfKMxxrEom7bTNzCqU7iCEA3ueKmntJzbFPVG61E");

#[program]
pub mod solana_pastel_oracle {
    use super::*;
}

const TXID_STATUS_VARIANT_COUNT: usize = 4; // Manually define the number of variants in TxidStatus

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct HashWeight {
    pub hash: String,
    pub weight: u64,
}

#[account]
pub struct Oracle {}

#[account]
pub struct AggregatedConsensusInfo {
    pub oracle: Pubkey,
    pub txid: String,
    pub status_weights: [i32; TXID_STATUS_VARIANT_COUNT],
    pub hash_weights: Vec<HashWeight>,
    pub first_6_characters_of_sha3_256_hash_of_corresponding_file: String,
    pub last_updated: i64, // Unix timestamp indicating the last update time
}

impl AggregatedConsensusInfo {
    pub const LEN: usize = 1000; // todo: calculate correct length
    pub const PREFIX: &'static [u8] = b"aggregated_consensus";
}
