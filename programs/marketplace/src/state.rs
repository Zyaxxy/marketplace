use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Marketplace{
    pub admin: Pubkey,
    pub fee: u16,
    pub bump: u8,
    pub treasury_bump: u8,
    pub rewards_bump: u8,
    #[max_len(32)]
    pub name: String,
}

#[account]
#[derive(InitSpace)]
pub struct Listing{
    pub maker: Pubkey,
    pub asset: Pubkey,
    pub payment_mint: Pubkey,
    pub price: u64,
    pub bump: u8
}

#[account]
#[derive(InitSpace)]
pub struct Offer{
    pub buyer: Pubkey,
    pub asset: Pubkey,
    pub amount: u64,
    pub bump: u8,
    pub vault_bump: u8,
}