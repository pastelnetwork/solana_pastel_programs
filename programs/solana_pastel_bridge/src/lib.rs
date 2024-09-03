use anchor_lang::{
    prelude::*,
    solana_program::hash::hash,
    system_program::{transfer, Transfer},
};
use solana_pastel_oracle::Oracle;

declare_id!("G7uVj97ovf94Eayow7Cujyt2eqJirDoiU65xbaE323fV");

#[program]
pub mod solana_pastel_bridge {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts
            .handle(ctx.bumps.bridge_reward_pool, ctx.bumps.bridge_escrow)
    }

    pub fn register_new_bridge_node(
        ctx: Context<RegisterNewBridgeNode>,
        pastel_id: String,
        bridge_node_psl_address: String,
    ) -> Result<()> {
        ctx.accounts
            .handle(ctx.bumps.bridge_node, &pastel_id, &bridge_node_psl_address)
    }

    pub fn submit_service_request(
        ctx: Context<SubmitServiceRequest>,
        pastel_ticket_type: PastelTicketType,
        first_6_chars_of_hash: String,
        ipfs_cid: String,
        file_size_bytes: u64,
    ) -> Result<()> {
        ctx.accounts.handle(
            ctx.bumps.service_request,
            pastel_ticket_type,
            &first_6_chars_of_hash,
            &ipfs_cid,
            file_size_bytes,
        )
    }

    pub fn initialize_best_price_quote(ctx: Context<InitializeBestPriceQuote>) -> Result<()> {
        ctx.accounts.handle()
    }

    pub fn submit_price_quote(
        ctx: Context<SubmitPriceQuote>,
        quoted_price_lamports: u64,
    ) -> Result<()> {
        ctx.accounts.handle(quoted_price_lamports)
    }

    pub fn submit_pastel_txid(ctx: Context<SubmitPastelTxid>, pastel_txid: String) -> Result<()> {
        ctx.accounts.handle(&pastel_txid)
    }
}

#[error_code]
pub enum BridgeError {
    #[msg("Bridge Contract state is already initialized")]
    ContractStateAlreadyInitialized,

    #[msg("Bridge node is already registered")]
    BridgeNodeAlreadyRegistered,

    #[msg("Action attempted by an unregistered bridge node")]
    UnregisteredBridgeNode,

    #[msg("Service request is invalid or malformed")]
    InvalidServiceRequest,

    #[msg("Escrow account for service request is not adequately funded")]
    EscrowNotFunded,

    #[msg("File size exceeds the maximum allowed limit")]
    InvalidFileSize,

    #[msg("Invalid or unsupported service type in service request")]
    InvalidServiceType,

    #[msg("Submitted price quote has expired and is no longer valid")]
    QuoteExpired,

    #[msg("Bridge node is inactive based on defined inactivity threshold")]
    BridgeNodeInactive,

    #[msg("Insufficient funds in escrow to cover the transaction")]
    InsufficientEscrowFunds,

    #[msg("Duplicate service request ID")]
    DuplicateServiceRequestId,

    #[msg("Contract is paused and no operations are allowed")]
    ContractPaused,

    #[msg("Invalid or missing first 6 characters of SHA3-256 hash")]
    InvalidFileHash,

    #[msg("IPFS CID is empty")]
    InvalidIpfsCid,

    #[msg("User Solana address is missing or invalid")]
    InvalidUserSolAddress,

    #[msg("Initial service request status must be Pending")]
    InvalidRequestStatus,

    #[msg("Payment in escrow should initially be false")]
    InvalidPaymentInEscrow,

    #[msg("Initial values for certain fields must be None")]
    InvalidInitialFieldValues,

    #[msg("Initial escrow amount and fees must be None or zero")]
    InvalidEscrowOrFeeAmounts,

    #[msg("Invalid service request ID.")]
    InvalidServiceRequestId,

    #[msg("Invalid Pastel transaction ID.")]
    InvalidPastelTxid,

    #[msg("Bridge node has not been selected for the service request.")]
    BridgeNodeNotSelected,

    #[msg("Unauthorized bridge node.")]
    UnauthorizedBridgeNode,

    #[msg("Service request to Pastel transaction ID mapping not found")]
    MappingNotFound,

    #[msg("Pastel transaction ID does not match with any service request")]
    TxidMismatch,

    #[msg("Consensus data from the oracle is outdated")]
    OutdatedConsensusData,

    #[msg("Service request not found")]
    ServiceRequestNotFound,

    #[msg("Service request is not in a pending state")]
    ServiceRequestNotPending,

    #[msg("Price quote submitted too late")]
    QuoteResponseTimeExceeded,

    #[msg("Bridge node is currently banned")]
    BridgeNodeBanned,

    #[msg("Quoted price cannot be zero")]
    InvalidQuotedPrice,

    #[msg("Price quote status is not set to 'Submitted'")]
    InvalidQuoteStatus,

    #[msg("Bridge node does not meet the minimum score requirements for rewards")]
    BridgeNodeScoreTooLow,

    #[msg("Pastel TXID not found in oracle data")]
    TxidNotFound,

    #[msg("Txid to Service Request ID Mapping account not found")]
    TxidMappingNotFound,

    #[msg("Insufficient registration fee paid by bridge node")]
    InsufficientRegistrationFee,

    #[msg("Withdrawal request from unauthorized account")]
    UnauthorizedWithdrawalAccount,

    #[msg("Insufficient funds for withdrawal request")]
    InsufficientFunds,

    #[msg("Timestamp conversion error")]
    TimestampConversionError,

    #[msg("Registration fee not paid")]
    RegistrationFeeNotPaid,
}

const MAX_QUOTE_RESPONSE_TIME: i64 = 600; // Max time for bridge nodes to respond with a quote in seconds (10 minutes)
const QUOTE_VALIDITY_DURATION: i64 = 21_600; // The time for which a submitted price quote remains valid. e.g., 6 hours in seconds
const ESCROW_DURATION: i64 = 7_200; // Duration to hold SOL in escrow in seconds (2 hours); if the service request is not fulfilled within this time, the SOL is refunded to the user and the bridge node won't receive any payment even if they fulfill the request later.
const DURATION_IN_SECONDS_TO_WAIT_AFTER_ANNOUNCING_NEW_PENDING_SERVICE_REQUEST_BEFORE_SELECTING_BEST_QUOTE: u64 = 300; // amount of time to wait after advertising pending service requests to select the best quote
const TRANSACTION_FEE_PERCENTAGE: u8 = 2; // Percentage of the selected (best) quoted price in SOL to be retained as a fee by the bridge contract for each successfully completed service request; the rest should be paid to the bridge node. If the bridge node fails to fulfill the service request, the full escrow  is refunded to the user without any deduction for the service fee.
const BRIDGE_NODE_REGISTRATION_FEE_IN_LAMPORTS: u64 = 1_000_000; // Registration fee for bridge nodes in lamports
const SERVICE_REQUEST_VALIDITY: u64 = 86_400; // Time until a service request expires if not responded to. e.g., 24 hours in seconds
const BRIDGE_NODE_INACTIVITY_THRESHOLD: u64 = 86_400; // e.g., 24 hours in seconds
const MIN_COMPLIANCE_SCORE_FOR_REWARD: f32 = 65.0; // Bridge Node must have a compliance score of at least N to be eligible for rewards
const MIN_RELIABILITY_SCORE_FOR_REWARD: f32 = 80.0; // Minimum reliability score to be eligible for rewards
const SERVICE_REQUESTS_FOR_PERMANENT_BAN: u32 = 250; //
const SERVICE_REQUESTS_FOR_TEMPORARY_BAN: u32 = 50; // Considered for temporary ban after 50 service requests
const TEMPORARY_BAN_SERVICE_FAILURES_THRESHOLD: u32 = 5; // Number of non-consensus report submissions for temporary ban
const TEMPORARY_BAN_DURATION: u64 = 24 * 60 * 60; // Duration of temporary ban in seconds (e.g., 1 day)
const MAX_DURATION_IN_SECONDS_FROM_LAST_REPORT_SUBMISSION_BEFORE_SELECTING_WINNING_QUOTE: i64 =
    2 * 60; // Maximum duration in seconds from service quote request before selecting the best quote (e.g., 2 minutes)
const DATA_RETENTION_PERIOD: u64 = 24 * 60 * 60; // How long to keep data in the contract state (1 day)
const TXID_STATUS_VARIANT_COUNT: usize = 4; // Manually define the number of variants in TxidStatus
const MAX_TXID_LENGTH: usize = 64; // Maximum length of a TXID
const MAX_ALLOWED_TIMESTAMP_DIFFERENCE: u64 = 600; // 10 minutes in seconds; maximum allowed time difference between the last update of the oracle's consensus data and the bridge contract's access to this data to ensure the bridge contract acts on timely and accurate data.
const LAMPORTS_PER_SOL: u64 = 1_000_000_000; // Number of lamports in one SOL

// Enums:

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, AnchorSerialize, AnchorDeserialize)]
pub enum TxidStatus {
    Invalid,
    PendingMining,
    MinedPendingActivation,
    MinedActivated,
}

//These are the different kinds of service requests that can be submitted to the bridge contract.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, AnchorSerialize, AnchorDeserialize)]
pub enum PastelTicketType {
    Sense,
    Cascade,
    Nft,
    InferenceApi,
}

// This tracks the status of an individual price quote submitted by a bridge node a for service request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, AnchorSerialize, AnchorDeserialize)]
pub enum ServicePriceQuoteStatus {
    Submitted,
    RejectedAsInvalid,
    RejectedAsTooHigh,
    Accepted,
}

// This tracks the status of the selection of the best price quote for a particular service request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, AnchorSerialize, AnchorDeserialize)]
pub enum BestQuoteSelectionStatus {
    NoQuotesReceivedYet,
    NoValidQuotesReceivedYet,
    WaitingToSelectBestQuote,
    BestQuoteSelected,
}

// This tracks the status of Solana payments for service requests.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, AnchorSerialize, AnchorDeserialize)]
pub enum PaymentStatus {
    Pending,
    Received,
}

// This controls emergency actions that can be taken by the admin to pause operations, modify parameters, etc. in the bridge contract in the event of an emergency.
#[derive(Debug, Clone, PartialEq, Eq, Hash, AnchorSerialize, AnchorDeserialize)]
pub enum EmergencyAction {
    PauseOperations,  // Pause all contract operations.
    ResumeOperations, // Resume all contract operations.
    ModifyParameters { key: String, value: String }, // Modify certain operational parameters.
                      // Additional emergency actions as needed...
}

// These are the various states that a service request can assume during its lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, AnchorSerialize, AnchorDeserialize)]
pub enum RequestStatus {
    Pending,            // The request has been created and is awaiting further action.
    AwaitingPayment,    // The request is waiting for SOL payment from the user.
    PaymentReceived,    // Payment for the request has been received and is held in escrow.
    BridgeNodeSelected, // A bridge node has been selected to fulfill the service request.
    InProgress,         // The service is currently being rendered by the selected bridge node.
    AwaitingCompletionConfirmation, // The service has been completed and is awaiting confirmation from the oracle.
    Completed,                      // The service has been successfully completed and confirmed.
    Failed,                         // The service request has failed or encountered an error.
    Expired,                        // The request has expired due to inactivity or non-fulfillment.
    Refunded,                       // Indicates that the request has been refunded to the user.
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct HashWeight {
    pub hash: String,
    pub weight: i32,
}

// The nodes that perform the service requests on behalf of the end users are called bridge nodes.
#[account]
pub struct Bridge {
    pub is_paused: bool,
    pub admin: Pubkey,
    pub bridge_nodes_count: u32,
    pub service_requests_count: u32,
    pub aggregated_consensuses_count: u32,
    pub oracle: Option<Pubkey>,
    pub bridge_reward_pool: Pubkey,
    pub bridge_reward_pool_bump: u8,
    pub bridge_escrow: Pubkey,
    pub bridge_escrow_bump: u8,
}

impl Bridge {
    pub const REWARD_POOL_PREFIX: &'static [u8] = b"bridge_reward_pool";
    pub const ESCROW_PREFIX: &'static [u8] = b"bridge_escrow";
}

// The nodes that perform the service requests on behalf of the end users are called bridge nodes.
#[account]
pub struct BridgeNode {
    pub bridge: Pubkey,
    pub pastel_id: String, // The unique identifier of the bridge node in the Pastel network, used as the primary key throughout the bridge contract to identify the bridge node.
    pub reward_address: Pubkey, // The Solana address of the bridge node, used to send rewards to the bridge node.
    pub bridge_node_psl_address: String, // The Pastel address of the bridge node, used to pay for service requests made by the bridge node on behalf of end users.
    pub registration_entrance_fee_transaction_signature: String, // The signature of the transaction that paid the registration fee in SOL for the bridge node to register with the bridge contract.
    pub compliance_score: f32, // The compliance score of the bridge node, which is a combined measure of the bridge node's overall track record of performing services quickly, accurately, and reliably.
    pub reliability_score: f32, // The reliability score of the bridge node, which is the percentage of all attempted service requests that were completed successfully by the bridge node.
    pub last_active_timestamp: i64, // The timestamp of the last time the bridge node performed a service request.
    pub total_price_quotes_submitted: u32, // The total number of price quotes submitted by the bridge node since registration.
    pub total_service_requests_attempted: u32, // The total number of service requests attempted by the bridge node since registration.
    pub successful_service_requests_count: u32, // The total number of service requests successfully completed by the bridge node since registration.
    pub current_streak: u32, // The current number of consecutive service requests successfully completed by the bridge node.
    pub failed_service_requests_count: u32, // The total number of service requests attempted by the bridge node that failed or were not completed successfully for any reason (even if not the bridge node's fault).
    pub ban_expiry: i64, // The timestamp when the bridge node's ban expires, if applicable.
    pub is_eligible_for_rewards: bool, // Indicates if the bridge node is eligible for rewards.
    pub is_recently_active: bool, // Indicates if the bridge node has been active recently.
    pub is_reliable: bool, // Indicates if the bridge node is considered reliable based on its track record.
    pub is_banned: bool,   // Indicates if the bridge node is currently banned.
    pub bump: u8,
}

impl BridgeNode {
    pub const LEN: usize = 1000; // todo: calculate correct length
    pub const PREFIX: &'static [u8] = b"bridge_node";
}

// These are the requests for services on Pastel Network submitted by end users; they are stored in the active service requests account.
// These requests initially come in with the type of service requested, the file hash, the IPFS CID, the file size, and the file MIME type;
// Then the bridge nodes submit price quotes for the service request, and the contract selects the best quote and selects a bridge node to fulfill the request.
// The end user then pays the quoted amount in SOL to the Bridge contract, which holds this amount in escrow until the service is completed successfully.
// The selected bridge node then performs the service and submits the Pastel transaction ID to the bridge contract when it's available; the bridge node ALSO submits the same
// Pastel transaction ID (txid) to the oracle contract for monitoring. When the oracle contract confirms the status of the transaction as being mined and activated,
// the bridge contract confirms that the ticket has been activated and that the file referenced in the ticket matches the file hash submitted in the
// service request. If the file hash matches, the escrowed SOL is released to the bridge node (minus the service fee paid to the Bridge contract) and the service request is marked as completed.
// If the file hash does not match, or if the oracle contract does not confirm the transaction as being mined and activated within the specified time limit, the escrowed SOL is refunded to the end user.
// If the bridge node fails to submit the Pastel transaction ID within the specified time limit, the service request is marked as failed and the escrowed SOL is refunded to the end user.
// The retained service fee (assuming a successful service request) is distributed to the the bridge contract's reward pool and the selected bridge node's scores are then
// updated based on the outcome of the service request.
#[account]
pub struct ServiceRequest {
    pub bridge: Pubkey,
    pub service_type: PastelTicketType, // Type of service requested (e.g., Sense, Cascade, Nft).
    pub first_6_characters_of_sha3_256_hash_of_corresponding_file: String, // First 6 characters of the SHA3-256 hash of the file involved in the service request.
    pub ipfs_cid: String,         // IPFS Content Identifier for the file.
    pub file_size_bytes: u64,     // Size of the file in bytes.
    pub user_sol_address: Pubkey, // Solana address of the end user who initiated the service request.
    pub status: RequestStatus,    // Current status of the service request.
    pub payment_in_escrow: bool, // Indicates if the payment for the service is currently in escrow.
    pub request_expiry: i64,     // Timestamp when the service request expires.
    pub sol_received_from_user_timestamp: Option<u64>, // Timestamp when SOL payment is received from the end user for the service request to be held in escrow.
    pub selected_bridge_node: Option<Pubkey>, // The Pastel ID of the bridge node selected to fulfill the service request.
    pub best_quoted_price_in_lamports: Option<u64>, // Price quoted for the service in lamports.
    pub service_request_creation_timestamp: i64, // Timestamp when the service request was created.
    pub bridge_node_selection_timestamp: Option<i64>, // Timestamp when a bridge node was selected for the service.
    pub bridge_node_submission_of_txid_timestamp: Option<i64>, // Timestamp when the bridge node submitted the Pastel transaction ID for the service request.
    pub submission_of_txid_to_oracle_timestamp: Option<u64>, // Timestamp when the Pastel transaction ID was submitted by the bridge contract to the oracle contract for monitoring.
    pub service_request_completion_timestamp: Option<u64>, // Timestamp when the service was completed.
    pub payment_received_timestamp: Option<u64>, // Timestamp when the payment was received into escrow.
    pub payment_release_timestamp: Option<u64>, // Timestamp when the payment was released from escrow, if applicable.
    pub escrow_amount_lamports: Option<u64>, // Amount of SOL held in escrow for this service request.
    pub service_fee_retained_by_bridge_contract_lamports: Option<u64>, // Amount of SOL retained by the bridge contract as a service fee, taken from the escrowed amount when the service request is completed.
    pub pastel_txid: Option<String>, // The Pastel transaction ID for the service request once it is created.
    pub bump: u8,
}

impl ServiceRequest {
    pub const LEN: usize = 1000; // todo: calculate correct length
    pub const PREFIX: &'static [u8] = b"service_request";

    // Function to generate the service_request_id based on the service type,
    // first 6 characters of the file hash,
    // and the end user's Solana address from which they will be paying for the service.
    pub fn generate_service_request_id(&self) -> String {
        let service_type_string: String = match self.service_type {
            PastelTicketType::Sense => String::from("Sense"),
            PastelTicketType::Cascade => String::from("Cascade"),
            PastelTicketType::Nft => String::from("Nft"),
            PastelTicketType::InferenceApi => String::from("InferenceApi"),
        };

        let user_sol_address_string = self.user_sol_address.to_string();

        let concatenated_str = format!(
            "{}{}{}",
            service_type_string,
            self.first_6_characters_of_sha3_256_hash_of_corresponding_file,
            user_sol_address_string,
        );

        // Convert the concatenated string to bytes
        let preimage_bytes = concatenated_str.as_bytes();
        // Compute hash
        let service_request_id_hash = hash(preimage_bytes);

        // Convert the first 12 bytes of Hash to a hex string (within 32 bytes limit for seed)
        let service_request_id_truncated = &service_request_id_hash.to_bytes()[..12];
        let hex_string: String = service_request_id_truncated
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect();

        hex_string
    }
}

// This holds the information for an individual price quote from a given bridge node for a particular service request.
#[account]
pub struct ServicePriceQuote {
    pub service_request: Pubkey,
    pub bridge_node: Pubkey,
    pub quoted_price_lamports: u64,
    pub quote_timestamp: i64,
    pub price_quote_status: ServicePriceQuoteStatus,
}

impl ServicePriceQuote {
    pub const LEN: usize = 1000; // todo: calculate correct length
    pub const PREFIX: &'static [u8] = b"service_price_quote";
}

#[account]
pub struct BestPriceQuote {
    pub service_request: Pubkey,
    pub best_bridge_node: Pubkey,
    pub best_quoted_price_in_lamports: u64,
    pub best_quote_timestamp: i64,
    pub best_quote_selection_status: BestQuoteSelectionStatus,
}

impl BestPriceQuote {
    pub const LEN: usize = 1000; // todo: calculate correct length
    pub const PREFIX: &'static [u8] = b"best_price_quote";
}

#[account]
pub struct AggregatedConsensus {
    pub bridge: Pubkey,
    pub txid: String,
    pub status_weights: [i32; TXID_STATUS_VARIANT_COUNT],
    pub hash_weights: Vec<HashWeight>,
    pub first_6_characters_of_sha3_256_hash_of_corresponding_file: String,
    pub last_updated_ts: i64, // Unix timestamp indicating the last update time
}

impl AggregatedConsensus {
    pub const LEN: usize = 1000; // todo: calculate correct length
    pub const PREFIX: &'static [u8] = b"aggregated_consensus";
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    pub admin: Signer<'info>,

    #[account(zero)]
    pub bridge: Account<'info, Bridge>,

    /// CHECK: unique by bridge
    #[account(seeds = [Bridge::REWARD_POOL_PREFIX, &bridge.key().to_bytes()], bump)]
    pub bridge_reward_pool: UncheckedAccount<'info>,

    /// CHECK: unique by bridge
    #[account(seeds = [Bridge::ESCROW_PREFIX, &bridge.key().to_bytes()], bump)]
    pub bridge_escrow: UncheckedAccount<'info>,

    /// Oracle state account
    pub oracle: Option<Account<'info, Oracle>>,

    // System program is needed for account creation
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn handle(&mut self, bridge_reward_pool_bump: u8, bridge_escrow_bump: u8) -> Result<()> {
        self.bridge.set_inner(Bridge {
            is_paused: false,
            admin: self.admin.key(),
            bridge_nodes_count: 0,
            service_requests_count: 0,
            aggregated_consensuses_count: 0,
            oracle: if self.oracle.is_some() {
                Some(self.oracle.as_ref().unwrap().key())
            } else {
                None
            },
            bridge_reward_pool: self.bridge_reward_pool.key(),
            bridge_reward_pool_bump,
            bridge_escrow: self.bridge_escrow.key(),
            bridge_escrow_bump,
        });

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(pastel_id: String)]
pub struct RegisterNewBridgeNode<'info> {
    /// CHECK: Manual checks are performed in the instruction to ensure the contributor_account is valid and safe to use.
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut, has_one = bridge_reward_pool)]
    pub bridge: Box<Account<'info, Bridge>>,

    /// unique bridge_node seeded by pastel_id given a bridge
    #[account(init,
        seeds = [BridgeNode::PREFIX, pastel_id.as_bytes(), &bridge.key().to_bytes()], bump,
        payer = payer, space = BridgeNode::LEN
    )]
    pub bridge_node: Account<'info, BridgeNode>,

    /// CHECK: unique by bridge
    #[account(
        seeds = [Bridge::REWARD_POOL_PREFIX, &bridge.key().to_bytes()],
        bump = bridge.bridge_reward_pool_bump,
    )]
    pub bridge_reward_pool: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> RegisterNewBridgeNode<'info> {
    pub fn handle(
        &mut self,
        bump: u8,
        pastel_id: &String,
        bridge_node_psl_address: &String,
    ) -> Result<()> {
        // pay non-refundable fee
        transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.user.to_account_info(),
                    to: self.bridge_reward_pool.to_account_info(),
                },
            ),
            BRIDGE_NODE_REGISTRATION_FEE_IN_LAMPORTS,
        )?;

        self.bridge_node.set_inner(BridgeNode {
            bridge: self.bridge.key(),
            pastel_id: pastel_id.clone(),
            reward_address: self.user.key(),
            bridge_node_psl_address: bridge_node_psl_address.clone(),
            registration_entrance_fee_transaction_signature: String::new(), // Replace with actual data if available
            compliance_score: 1.0,  // Initial compliance score
            reliability_score: 1.0, // Initial reliability score
            last_active_timestamp: Clock::get()?.unix_timestamp, // Set the last active timestamp to the current time
            total_price_quotes_submitted: 0, // Initially, no price quotes have been submitted
            total_service_requests_attempted: 0, // Initially, no service requests attempted
            successful_service_requests_count: 0, // Initially, no successful service requests
            current_streak: 0,               // No streak at the beginning
            failed_service_requests_count: 0, // No failed service requests at the start
            ban_expiry: 0,                   // No ban initially set
            is_eligible_for_rewards: false,  // Initially not eligible for rewards
            is_recently_active: false,       // Initially not considered active
            is_reliable: false,              // Initially not considered reliable
            is_banned: false,                // Initially not banned
            bump,
        });

        self.bridge.bridge_nodes_count += 1;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(pastel_ticket_type: PastelTicketType, first_6_chars_of_hash: String)]
pub struct SubmitServiceRequest<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub bridge: Box<Account<'info, Bridge>>,

    #[account(init,
        seeds = [
            ServiceRequest::PREFIX,
            &[pastel_ticket_type as u8],
            first_6_chars_of_hash.as_bytes(),
            &user.key().to_bytes(),
            &bridge.key().to_bytes(),
        ],
        bump, payer = payer, space = ServiceRequest::LEN
    )]
    pub service_request: Account<'info, ServiceRequest>,

    pub system_program: Program<'info, System>,
}

impl<'info> SubmitServiceRequest<'info> {
    pub fn handle(
        &mut self,
        bump: u8,
        pastel_ticket_type: PastelTicketType,
        first_6_chars_of_hash: &String,
        ipfs_cid: &String,
        file_size_bytes: u64,
    ) -> Result<()> {
        let current_timestamp = Clock::get()?.unix_timestamp;

        self.service_request.set_inner(ServiceRequest {
            bridge: self.bridge.key(),
            service_type: pastel_ticket_type,
            first_6_characters_of_sha3_256_hash_of_corresponding_file: first_6_chars_of_hash
                .clone(),
            ipfs_cid: ipfs_cid.clone(),
            file_size_bytes,
            user_sol_address: self.user.key(),
            status: RequestStatus::Pending,
            payment_in_escrow: false,
            request_expiry: current_timestamp + ESCROW_DURATION, // Set appropriate expiry timestamp
            sol_received_from_user_timestamp: None,
            selected_bridge_node: None,
            best_quoted_price_in_lamports: None,
            service_request_creation_timestamp: current_timestamp,
            bridge_node_selection_timestamp: None,
            bridge_node_submission_of_txid_timestamp: None,
            submission_of_txid_to_oracle_timestamp: None,
            service_request_completion_timestamp: None,
            payment_received_timestamp: None,
            payment_release_timestamp: None,
            escrow_amount_lamports: None,
            service_fee_retained_by_bridge_contract_lamports: None,
            pastel_txid: None,
            bump,
        });

        self.bridge.service_requests_count += 1;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeBestPriceQuote<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub service_request: Account<'info, ServiceRequest>,

    #[account(init,
        seeds = [BestPriceQuote::PREFIX, &service_request.key().to_bytes()], bump,
        payer = payer, space = BestPriceQuote::LEN
    )]
    pub best_price_quote: Account<'info, BestPriceQuote>,

    pub system_program: Program<'info, System>,
}

impl<'info> InitializeBestPriceQuote<'info> {
    pub fn handle(&mut self) -> Result<()> {
        self.best_price_quote.set_inner(BestPriceQuote {
            service_request: self.service_request.key(),
            best_bridge_node: Pubkey::default(),
            best_quoted_price_in_lamports: 0,
            best_quote_timestamp: 0,
            best_quote_selection_status: BestQuoteSelectionStatus::NoQuotesReceivedYet,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct SubmitPriceQuote<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub bridge: Account<'info, Bridge>,

    #[account(mut, has_one = bridge)]
    pub bridge_node: Account<'info, BridgeNode>,

    #[account(has_one = bridge)]
    pub service_request: Account<'info, ServiceRequest>,

    #[account(mut)]
    pub best_price_quote: Account<'info, BestPriceQuote>,

    #[account(init,
        seeds = [ServicePriceQuote::PREFIX, &service_request.key().to_bytes(), &bridge_node.key().to_bytes()], bump,
        payer = payer, space = ServicePriceQuote::LEN
    )]
    pub service_price_quote: Account<'info, ServicePriceQuote>,

    pub system_program: Program<'info, System>,
}

impl<'info> SubmitPriceQuote<'info> {
    pub fn handle(&mut self, quoted_price_lamports: u64) -> Result<()> {
        require_gt!(quoted_price_lamports, 0, BridgeError::InvalidQuotedPrice);

        let quote_timestamp = Clock::get()?.unix_timestamp;

        require_gte!(
            self.service_request.service_request_creation_timestamp + MAX_QUOTE_RESPONSE_TIME,
            quote_timestamp,
            BridgeError::QuoteResponseTimeExceeded
        );
        require_gte!(
            self.service_request.service_request_creation_timestamp + MAX_DURATION_IN_SECONDS_FROM_LAST_REPORT_SUBMISSION_BEFORE_SELECTING_WINNING_QUOTE,
            quote_timestamp,
            BridgeError::QuoteResponseTimeExceeded
        );
        require!(
            self.service_request.status == RequestStatus::Pending,
            BridgeError::ServiceRequestNotPending
        );

        require!(
            self.bridge_node.ban_expiry <= quote_timestamp,
            BridgeError::BridgeNodeBanned
        );
        require_gte!(
            self.bridge_node.compliance_score,
            MIN_COMPLIANCE_SCORE_FOR_REWARD,
            BridgeError::BridgeNodeScoreTooLow
        );
        require_gte!(
            self.bridge_node.reliability_score,
            MIN_RELIABILITY_SCORE_FOR_REWARD,
            BridgeError::BridgeNodeScoreTooLow
        );

        self.service_price_quote.set_inner(ServicePriceQuote {
            bridge_node: self.bridge_node.key(),
            service_request: self.service_request.key(),
            quote_timestamp,
            quoted_price_lamports,
            price_quote_status: ServicePriceQuoteStatus::Submitted,
        });

        if self.best_price_quote.best_quote_selection_status
            == BestQuoteSelectionStatus::NoQuotesReceivedYet
        {
            self.best_price_quote.best_quote_selection_status =
                BestQuoteSelectionStatus::WaitingToSelectBestQuote;
            self.best_price_quote.best_quote_timestamp = quote_timestamp;
        }

        if self.best_price_quote.best_quote_selection_status
            == BestQuoteSelectionStatus::WaitingToSelectBestQuote
        {
            if self.best_price_quote.best_quoted_price_in_lamports < quoted_price_lamports {
                self.best_price_quote.best_bridge_node = self.bridge_node.key();
                self.best_price_quote.best_quoted_price_in_lamports = quoted_price_lamports;
            }
        }

        if quote_timestamp > self.best_price_quote.best_quote_timestamp + QUOTE_VALIDITY_DURATION {
            self.best_price_quote.best_quote_selection_status =
                BestQuoteSelectionStatus::BestQuoteSelected;
            self.service_request.selected_bridge_node =
                Some(self.best_price_quote.best_bridge_node);
            self.service_request.bridge_node_selection_timestamp = Some(quote_timestamp);
        } else {
            self.bridge_node.total_price_quotes_submitted += 1;
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct SubmitPastelTxid<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut,
        constraint = service_request.selected_bridge_node.is_some() @ BridgeError::BridgeNodeNotSelected,
        constraint = service_request.selected_bridge_node.unwrap() == bridge_node.key() @ BridgeError::BridgeNodeNotSelected,
        constraint = service_request.status == RequestStatus::Pending @ BridgeError::InvalidRequestStatus,
    )]
    pub service_request: Account<'info, ServiceRequest>,

    // The bridge contract state
    #[account(
        mut,
        constraint = bridge.is_paused @ BridgeError::ContractPaused
    )]
    pub bridge: Account<'info, Bridge>,

    // Bridge Nodes Data Account
    #[account(mut, has_one = bridge)]
    pub bridge_node: Account<'info, BridgeNode>,

    pub system_program: Program<'info, System>,
}

impl<'info> SubmitPastelTxid<'info> {
    pub fn handle(&mut self, pastel_txid: &String) -> Result<()> {
        require!(
            !pastel_txid.is_empty() && pastel_txid.len() <= MAX_TXID_LENGTH,
            BridgeError::InvalidPastelTxid
        );

        self.service_request.pastel_txid = Some(pastel_txid.clone());
        self.service_request
            .bridge_node_submission_of_txid_timestamp = Some(Clock::get()?.unix_timestamp);
        self.service_request.status = RequestStatus::AwaitingCompletionConfirmation;

        Ok(())
    }
}
