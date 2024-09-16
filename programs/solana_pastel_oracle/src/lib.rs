use anchor_lang::prelude::*;

declare_id!("HV8WdfKMxxrEom7bTNzCqU7iCEA3ueKmntJzbFPVG61E");

#[program]
pub mod solana_pastel_oracle {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.handle(ctx.bumps.reward_pool)
    }
}

#[error_code]
pub enum OracleError {
    ContributorAlreadyRegistered,
    UnregisteredOracle,
    InvalidTxid,
    InvalidFileHashLength,
    MissingPastelTicketType,
    MissingFileHash,
    RegistrationFeeNotPaid,
    NotEligibleForReward,
    NotBridgeContractAddress,
    InsufficientFunds,
    UnauthorizedWithdrawalAccount,
    InvalidPaymentAmount,
    PaymentNotFound,
    PendingPaymentAlreadyInitialized,
    AccountAlreadyInitialized,
    PendingPaymentInvalidAmount,
    InvalidPaymentStatus,
    InvalidTxidStatus,
    InvalidPastelTicketType,
    ContributorNotRegistered,
    ContributorBanned,
    EnoughReportsSubmittedForTxid,
}

const REGISTRATION_ENTRANCE_FEE_IN_LAMPORTS: u64 = 10_000_000; // 0.10 SOL in lamports
const MIN_NUMBER_OF_ORACLES: usize = 8; // Minimum number of oracles to calculate consensus
const MIN_REPORTS_FOR_REWARD: u32 = 10; // Data Contributor must submit at least 10 reports to be eligible for rewards
const MIN_COMPLIANCE_SCORE_FOR_REWARD: f32 = 65.0; // Data Contributor must have a compliance score of at least 80 to be eligible for rewards
const MIN_RELIABILITY_SCORE_FOR_REWARD: f32 = 80.0; // Minimum reliability score to be eligible for rewards
const BASE_REWARD_AMOUNT_IN_LAMPORTS: u64 = 100_000; // 0.0001 SOL in lamports is the base reward amount, which is scaled based on the number of highly reliable contributors
const COST_IN_LAMPORTS_OF_ADDING_PASTEL_TXID_FOR_MONITORING: u64 = 100_000; // 0.0001 SOL in lamports
const PERMANENT_BAN_THRESHOLD: u32 = 100; // Number of non-consensus report submissions for permanent ban
const CONTRIBUTIONS_FOR_PERMANENT_BAN: u32 = 250; // Considered for permanent ban after 250 contributions
const TEMPORARY_BAN_THRESHOLD: u32 = 5; // Number of non-consensus report submissions for temporary ban
const CONTRIBUTIONS_FOR_TEMPORARY_BAN: u32 = 50; // Considered for temporary ban after 50 contributions
const TEMPORARY_BAN_DURATION: u32 = 24 * 60 * 60; // Duration of temporary ban in seconds (e.g., 1 day)
const MAX_DURATION_IN_SECONDS_FROM_LAST_REPORT_SUBMISSION_BEFORE_COMPUTING_CONSENSUS: u32 = 10 * 60; // Maximum duration in seconds from last report submission for a given TXID before computing consensus (e.g., 10 minutes)
const DATA_RETENTION_PERIOD: u32 = 24 * 60 * 60; // How long to keep data in the contract state (1 day)
const SUBMISSION_COUNT_RETENTION_PERIOD: u32 = 24 * 60 * 60; // Number of seconds to retain submission counts (i.e., 24 hours)
const TXID_STATUS_VARIANT_COUNT: usize = 4; // Manually define the number of variants in TxidStatus
const MAX_TXID_LENGTH: usize = 64; // Maximum length of a TXID

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct HashWeight {
    pub hash: String,
    pub weight: u64,
}

#[account]
pub struct Oracle {
    pub admin: Pubkey,
    pub reward_pool: Pubkey,
    pub reward_pool_bump: u8,
}

impl Oracle {
    pub const REWARD_POOL_PREFIX: &'static [u8] = b"reward_pool";
}

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
    pub const LEN: usize = 128 + 200; // addtional 200 for `txid` and `first_6_characters_of_sha3_256_hash_of_corresponding_file`
    pub const PREFIX: &'static [u8] = b"aggregated_consensus";
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    pub admin: Signer<'info>,

    #[account(zero)]
    pub oracle: Account<'info, Oracle>,

    /// CHECK: unique by oracle
    #[account(seeds = [Oracle::REWARD_POOL_PREFIX, &oracle.key().to_bytes()], bump)]
    pub reward_pool: UncheckedAccount<'info>,

    // System program is needed for account creation
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn handle(&mut self, reward_pool_bump: u8) -> Result<()> {
        self.oracle.set_inner(Oracle {
            admin: self.admin.key(),
            reward_pool: self.reward_pool.key(),
            reward_pool_bump,
        });

        Ok(())
    }
}
