use anchor_lang::prelude::*;

#[account]
pub struct OracleState {
    pub admin: Pubkey,
    pub price: u64,
    pub decimals: u8,
    pub last_updated_slot: u64,
    pub bump: u8,
}

impl OracleState {
    pub const SEED: &'static [u8] = b"oracle_state";
    pub const SIZE: usize = 32 + 8 + 1 + 8 + 1; // 50 bytes
}
