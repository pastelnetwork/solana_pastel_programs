use anchor_lang::prelude::*;

#[account]
pub struct Oracle {}

declare_id!("HV8WdfKMxxrEom7bTNzCqU7iCEA3ueKmntJzbFPVG61E");

#[program]
pub mod solana_pastel_oracle {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
