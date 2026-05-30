use anchor_lang::prelude::*;
use crate::state::*;
use crate::error::ErrorCode;
use anchor_lang::system_program::{Transfer, transfer};

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    ///CHECK: asset validated when used by maker/accept
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,
    #[account(
        init,
        payer = buyer,
        seeds = [b"offer", asset.key().as_ref(), buyer.key().as_ref()],
        bump,
        space = Offer::DISCRIMINATOR.len() + Offer::INIT_SPACE
    )]
    pub offer: Box<Account<'info, Offer>>,
    
    #[account(
        mut,
        seeds = [b"offer_vault", asset.key().as_ref(), buyer.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> MakeOffer<'info> {
    pub fn create_offer(&mut self, amount: u64, bumps: &MakeOfferBumps) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidOfferAmount);

        self.offer.set_inner(Offer {
            buyer: self.buyer.key(),
            asset: self.asset.key(),
            amount,
            bump: bumps.offer,
            vault_bump: bumps.vault,
        });

        // Transfer lamports from buyer into the vault PDA
        let cpi_ctx = CpiContext::new(
            self.system_program.to_account_info(),
            Transfer {
                from: self.buyer.to_account_info(),
                to: self.vault.to_account_info(),
            },
        );

        transfer(cpi_ctx, amount)?;

        Ok(())
    }
}
