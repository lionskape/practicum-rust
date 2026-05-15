use anchor_lang::{prelude::*, system_program};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount},
};
use mpl_token_metadata::{
    instructions::CreateMetadataAccountV3CpiBuilder, types::DataV2, ID as MPL_TOKEN_METADATA_ID,
};
use sol_usd_oracle::{state::OracleState, PRICE_DECIMALS};

pub const USD_DECIMALS: u8 = 6;
pub const LAMPORTS_PER_SOL_U64: u64 = 1_000_000_000;

declare_id!("95S6rgKz3RGSwgLiSa7sFkW5iY68TLCzj5ezyNMvQrCc");

#[program]
pub mod token_minter {
    use super::*;

    pub fn initialize_minter(
        ctx: Context<InitializeMinter>,
        treasury: Pubkey,
        mint_fee_usd: u64,
        oracle_state: Pubkey,
        oracle_program: Pubkey,
    ) -> Result<()> {
        require!(mint_fee_usd > 0, MinterError::InvalidFeeUsd);

        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.treasury = treasury;
        config.mint_fee_usd = mint_fee_usd;
        config.oracle_program = oracle_program;
        config.oracle_state = oracle_state;
        config.bump = ctx.bumps.config;
        Ok(())
    }

    pub fn set_fee_usd(ctx: Context<SetFeeUsd>, new_fee_usd: u64) -> Result<()> {
        require!(new_fee_usd > 0, MinterError::InvalidFeeUsd);
        ctx.accounts.config.mint_fee_usd = new_fee_usd;
        Ok(())
    }

    pub fn set_treasury(ctx: Context<SetTreasury>, new_treasury: Pubkey) -> Result<()> {
        ctx.accounts.config.treasury = new_treasury;
        Ok(())
    }

    pub fn mint_token(
        ctx: Context<MintToken>,
        decimals: u8,
        initial_supply: u64,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        require!(initial_supply > 0, MinterError::InvalidSupply);
        require!(
            decimals <= 9,
            MinterError::InvalidDecimals // stricter than SPL max of 9..=18 elsewhere
        );

        let oracle_state = &ctx.accounts.oracle_state;
        require!(oracle_state.price > 0, MinterError::OraclePriceZero);
        require!(oracle_state.decimals == PRICE_DECIMALS, MinterError::OracleDecimalsMismatch);

        let fee_lamports =
            compute_fee_lamports(ctx.accounts.config.mint_fee_usd, oracle_state.price)?;

        // Transfer SOL fee from user to treasury
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.user.to_account_info(),
                    to: ctx.accounts.treasury.to_account_info(),
                },
            ),
            fee_lamports,
        )?;

        // Mint account is already initialized via constraints, now mint tokens to ATA
        token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.user_ata.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            initial_supply,
        )?;

        // Optional: create Metaplex token metadata (name/symbol/uri) when name is non-empty
        if !name.is_empty() {
            require!(
                ctx.accounts.token_metadata_program.key() == MPL_TOKEN_METADATA_ID,
                MinterError::InvalidMetadataProgram
            );
            let (metadata_pda, _) = Pubkey::find_program_address(
                &[
                    b"metadata",
                    ctx.accounts.token_metadata_program.key().as_ref(),
                    ctx.accounts.mint.key().as_ref(),
                ],
                &ctx.accounts.token_metadata_program.key(),
            );
            require!(metadata_pda == ctx.accounts.metadata.key(), MinterError::InvalidMetadataPda);

            let name_trim = name.trim();
            let symbol_trim = symbol.trim();
            let uri_trim = uri.trim();
            let name_fit: String = name_trim.chars().take(32).collect();
            let symbol_fit: String = symbol_trim.chars().take(10).collect();
            let uri_fit: String = uri_trim.chars().take(200).collect();

            let data_v2 = DataV2 {
                name: name_fit,
                symbol: symbol_fit,
                uri: uri_fit,
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            };

            CreateMetadataAccountV3CpiBuilder::new(
                &ctx.accounts.token_metadata_program.to_account_info(),
            )
            .metadata(&ctx.accounts.metadata.to_account_info())
            .mint(&ctx.accounts.mint.to_account_info())
            .mint_authority(&ctx.accounts.user.to_account_info())
            .payer(&ctx.accounts.user.to_account_info())
            .update_authority(&ctx.accounts.user.to_account_info(), true)
            .system_program(&ctx.accounts.system_program.to_account_info())
            .rent(Some(&ctx.accounts.rent.to_account_info()))
            .data(data_v2)
            .is_mutable(true)
            .invoke()
            .map_err(|_| MinterError::MetadataCpiFailed)?;
        }

        emit!(TokenCreated {
            creator: ctx.accounts.user.key(),
            mint: ctx.accounts.mint.key(),
            decimals,
            initial_supply,
            fee_lamports,
            sol_usd_price: oracle_state.price,
            slot: Clock::get()?.slot,
        });

        Ok(())
    }
}

fn compute_fee_lamports(mint_fee_usd: u64, price: u64) -> Result<u64> {
    require!(price > 0, MinterError::OraclePriceZero);

    mint_fee_usd
        .checked_mul(LAMPORTS_PER_SOL_U64)
        .and_then(|fee_in_lamport_scale| fee_in_lamport_scale.checked_div(price))
        .ok_or(error!(MinterError::MathOverflow))
}

#[derive(Accounts)]
pub struct InitializeMinter<'info> {
    #[account(
        init,
        payer = admin,
        seeds = [MinterConfig::SEED],
        bump,
        space = 8 + MinterConfig::SIZE
    )]
    pub config: Account<'info, MinterConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetFeeUsd<'info> {
    #[account(mut, seeds = [MinterConfig::SEED], bump = config.bump, has_one = admin)]
    pub config: Account<'info, MinterConfig>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetTreasury<'info> {
    #[account(mut, seeds = [MinterConfig::SEED], bump = config.bump, has_one = admin)]
    pub config: Account<'info, MinterConfig>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(decimals: u8, initial_supply: u64)]
pub struct MintToken<'info> {
    #[account(
        mut,
        seeds = [MinterConfig::SEED],
        bump = config.bump,
        has_one = treasury,
        constraint = config.oracle_program == oracle_program.key() @ MinterError::InvalidOracleProgram,
        constraint = config.oracle_state == oracle_state.key() @ MinterError::InvalidOracleState
    )]
    pub config: Account<'info, MinterConfig>,
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: validated by has_one on config
    #[account(mut, address = config.treasury)]
    pub treasury: UncheckedAccount<'info>,
    pub oracle_program: Program<'info, sol_usd_oracle::program::SolUsdOracle>,
    #[account(
        seeds = [OracleState::SEED],
        bump = oracle_state.bump,
        owner = oracle_program.key(),
        seeds::program = oracle_program
    )]
    pub oracle_state: Account<'info, OracleState>,
    #[account(
        init,
        payer = user,
        mint::decimals = decimals,
        mint::authority = user,
        mint::freeze_authority = user
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = user
    )]
    pub user_ata: Account<'info, TokenAccount>,
    /// CHECK: Metaplex Token Metadata program; only used when name is non-empty
    pub token_metadata_program: UncheckedAccount<'info>,
    /// CHECK: Metadata PDA (metadata program + mint); only used when name is non-empty
    pub metadata: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct MinterConfig {
    pub admin: Pubkey,
    pub treasury: Pubkey,
    pub mint_fee_usd: u64,
    pub oracle_program: Pubkey,
    pub oracle_state: Pubkey,
    pub bump: u8,
}

impl MinterConfig {
    pub const SEED: &'static [u8] = b"minter_config";
    pub const SIZE: usize = 32 + 32 + 8 + 32 + 32 + 1 + 7; // +7 padding for alignment
}

#[event]
pub struct TokenCreated {
    pub creator: Pubkey,
    pub mint: Pubkey,
    pub decimals: u8,
    pub initial_supply: u64,
    pub fee_lamports: u64,
    pub sol_usd_price: u64,
    pub slot: u64,
}

#[error_code]
pub enum MinterError {
    #[msg("Mint fee in USD must be greater than zero")]
    InvalidFeeUsd,
    #[msg("Unauthorized admin call")]
    Unauthorized,
    #[msg("Oracle price must be greater than zero")]
    OraclePriceZero,
    #[msg("Math overflow while computing fee")]
    MathOverflow,
    #[msg("Invalid supply value")]
    InvalidSupply,
    #[msg("Decimals out of allowed range")]
    InvalidDecimals,
    #[msg("Oracle account does not match config")]
    InvalidOracleState,
    #[msg("Oracle program does not match config")]
    InvalidOracleProgram,
    #[msg("Oracle decimals mismatch expected 6")]
    OracleDecimalsMismatch,
    #[msg("Invalid Metaplex Token Metadata program")]
    InvalidMetadataProgram,
    #[msg("Invalid metadata PDA")]
    InvalidMetadataPda,
    #[msg("Metaplex create metadata CPI failed")]
    MetadataCpiFailed,
}
