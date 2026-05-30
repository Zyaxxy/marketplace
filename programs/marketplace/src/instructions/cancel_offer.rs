use anchor_lang::prelude::*;
use crate::state::*;
use crate::error::ErrorCode;
use anchor_lang::system_program::{Transfer, transfer};

#[derive(Accounts)]
pub struct CancelOffer<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    ///CHECK: asset validated by PDA seeds on offer
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,
    #[account(
        mut,
        close = buyer,
        seeds = [b"offer", asset.key().as_ref(), buyer.key().as_ref()],
        bump = offer.bump,
        has_one = buyer,
        has_one = asset
    )]
    pub offer: Box<Account<'info, Offer>>,
    ///CHECK: Vault PDA to hold lamports for this offer (program-owned PDA)
    #[account(
        mut,
        seeds = [b"offer_vault", asset.key().as_ref(), buyer.key().as_ref()],
        bump = offer.vault_bump,
    )]
    pub vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> CancelOffer<'info> {
    pub fn cancel(&mut self) -> Result<()> {
        // Only the original buyer (offer maker) may cancel — enforced by has_one and signer
            require_keys_eq!(self.buyer.key(), self.offer.buyer, ErrorCode::UnauthorizedOfferCancellation);
            require_keys_eq!(self.asset.key(), self.offer.asset, ErrorCode::OfferAssetMismatch);

        let asset_key = self.asset.key();
        let vault_seed: &[&[u8]] = &[
            b"offer_vault".as_ref(),
            asset_key.as_ref(),
            self.offer.buyer.as_ref(),
            &[self.offer.vault_bump],
        ];
        let signer_seeds = &[vault_seed];

        let cpi_ctx = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            Transfer {
                from: self.vault.to_account_info(),
                to: self.buyer.to_account_info(),
            },
            signer_seeds,
        );

        transfer(cpi_ctx, self.vault.lamports())?;

        Ok(())
    }
}
