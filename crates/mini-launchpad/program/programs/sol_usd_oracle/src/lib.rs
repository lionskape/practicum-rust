use anchor_lang::prelude::*;

pub const PRICE_DECIMALS: u8 = 6;

pub mod state;
pub use state::OracleState;

declare_id!("3SyERpqhcx5V7z1wc8pwpCftbNhVPCfcW1BNtM5baUm8");

fn apply_price_update(oracle: &mut OracleState, new_price: u64, current_slot: u64) -> Result<()> {
    oracle.price = new_price;
    oracle.last_updated_slot = current_slot;
    Ok(())
}

#[program]
pub mod sol_usd_oracle {
    use super::*;

    pub fn initialize_oracle(ctx: Context<InitializeOracle>, admin: Pubkey) -> Result<()> {
        let oracle = &mut ctx.accounts.oracle;
        oracle.admin = admin;
        oracle.price = 0;
        oracle.decimals = PRICE_DECIMALS;
        oracle.last_updated_slot = Clock::get()?.slot;
        oracle.bump = ctx.bumps.oracle;
        Ok(())
    }

    pub fn update_price(ctx: Context<UpdatePrice>, new_price: u64) -> Result<()> {
        require!(new_price > 0, OracleError::InvalidPrice);

        let oracle = &mut ctx.accounts.oracle;
        require_keys_eq!(ctx.accounts.admin.key(), oracle.admin, OracleError::Unauthorized);

        let current_slot = Clock::get()?.slot;
        apply_price_update(oracle, new_price, current_slot)
    }
}

#[derive(Accounts)]
pub struct InitializeOracle<'info> {
    #[account(
        init,
        payer = payer,
        seeds = [OracleState::SEED],
        bump,
        space = 8 + OracleState::SIZE
    )]
    pub oracle: Account<'info, OracleState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdatePrice<'info> {
    #[account(mut, seeds = [OracleState::SEED], bump = oracle.bump, has_one = admin)]
    pub oracle: Account<'info, OracleState>,
    pub admin: Signer<'info>,
}

#[error_code]
pub enum OracleError {
    #[msg("Only oracle admin may call this instruction")]
    Unauthorized,
    #[msg("Price must be greater than zero")]
    InvalidPrice,
}
