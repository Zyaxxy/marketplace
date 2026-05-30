#![allow(deprecated)]
use anchor_lang::prelude::*;
pub mod state;
pub mod instructions;
pub mod error;

pub use state::*;
pub use instructions::*;
declare_id!("6ghUFmM6K4x1y3nq4WbCwtAZFCe6mip2LNbiukUjV9g3");

#[program]
pub mod marketplace {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, name:String, fee:u16 ) -> Result<()> {
        ctx.accounts.init(name, fee, &ctx.bumps)
    }

    pub fn list(ctx: Context<List>, price: u64) -> Result<()> {
        ctx.accounts.create_listing(price, &ctx.bumps)
    }

    pub fn buy(ctx: Context<Buy>) -> Result<()> {
        ctx.accounts.send_sol()?;
        ctx.accounts.recieve_nft()?;
        ctx.accounts.distribute_rewards()
    }

    pub fn buy_with_token(ctx: Context<BuyWithToken>) -> Result<()> {
        ctx.accounts.send_tokens()?;
        ctx.accounts.recieve_nft()?;
        ctx.accounts.distribute_rewards()
    }

    pub fn delist(ctx: Context<Delist>) -> Result<()> {
        ctx.accounts.delist()
    }
}

