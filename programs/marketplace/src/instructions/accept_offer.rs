use anchor_lang::prelude::*;
use mpl_core::{instructions:: TransferV1CpiBuilder ,ID as MPL_CORE_ID};
use anchor_spl::{associated_token::AssociatedToken, token::{mint_to, MintTo}, token_interface::{Mint, TokenAccount, TokenInterface}};
use crate::{state::*, error::ErrorCode};
use anchor_lang::system_program::{Transfer, transfer};

#[derive(Accounts)]
pub struct AcceptOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    ///CHECK: buyer is validated by PDA seeds on offer
    #[account(mut)]
    pub buyer: UncheckedAccount<'info>,
    ///CHECK: asset validated in cpi
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,
    ///CHECK: optional collection passed to mpl-core
    #[account(mut)]
    pub collection: Option<UncheckedAccount<'info>>,

    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes()],
        bump = marketplace.bump,
        )]
    pub marketplace: Box<Account<'info, Marketplace>>,

    #[account(
        mut,
        close = maker,
        seeds = [b"listing", listing.asset.key().as_ref()],
        bump = listing.bump,
        has_one = maker,
        has_one = asset
    )]
    pub listing: Box<Account<'info, Listing>>,

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

    #[account(
        mut,
        seeds = [b"treasury", marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [b"rewards", marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
        mint::decimals = 6,
        mint::authority = marketplace
    )]
    pub reward_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = reward_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program
    )]
    pub buyer_reward_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    ///CHECK: validated in cpi by mplCore
    #[account(
        address = MPL_CORE_ID,
    )]
    pub mpl_core_program: UncheckedAccount<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> AcceptOffer<'info> {
    pub fn accept(&mut self) -> Result<()> {
        let price = self.offer.amount;
        require!(price > 0, ErrorCode::InvalidOfferAmount);
        require_keys_eq!(self.asset.key(), self.offer.asset, ErrorCode::OfferAssetMismatch);

        let fee = (price as u128)
            .checked_mul(self.marketplace.fee as u128)
            .unwrap()
            .checked_div(10000)
            .unwrap() as u64;
        let maker_amount = price.checked_sub(fee).unwrap();

        // signer seeds for the vault PDA
        let asset_key = self.asset.key();
        let vault_seed: &[&[u8]] = &[
            b"offer_vault".as_ref(),
            asset_key.as_ref(),
            self.offer.buyer.as_ref(),
            &[self.offer.vault_bump],
        ];
        let signer_seeds = &[vault_seed];

        // Transfer maker amount from vault to maker
        let maker_ctx = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            Transfer {
                from: self.vault.to_account_info(),
                to: self.maker.to_account_info(),
            },
            signer_seeds,
        );
        transfer(maker_ctx, maker_amount)?;

        // Transfer fee to treasury
        let fee_ctx = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            Transfer {
                from: self.vault.to_account_info(),
                to: self.treasury.to_account_info(),
            },
            signer_seeds,
        );
        transfer(fee_ctx, fee)?;
        // Refund any remaining lamports in the vault to the buyer
        let rent_ctx = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            Transfer {
                from: self.vault.to_account_info(),
                to: self.buyer.to_account_info(),
            },
            signer_seeds,
        );
        transfer(rent_ctx, self.vault.lamports())?;

        // Transfer NFT from listing to buyer (offerer)
        let asset_key = self.asset.key();
        let bump = self.listing.bump;
        let seed: &[&[u8]] = &[b"listing", asset_key.as_ref(), &[bump]];
        let signer = &[seed];

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(self.collection.as_ref().map(|c|c.as_ref()))
            .payer(&self.maker.to_account_info())
            .authority(Some(&self.listing.to_account_info()))
            .new_owner(&self.buyer.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(signer)?;

        // Distribute rewards to buyer
        let reward_amount = (price as u128)
            .checked_mul(100) // 1% rewards
            .unwrap()
            .checked_div(10000)
            .unwrap() as u64;

        let seeds = &[
            b"marketplace".as_ref(),
            self.marketplace.name.as_str().as_bytes(),
            &[self.marketplace.bump],
        ];
        let signer_seeds = &[seeds.as_ref()];

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            MintTo {
                mint: self.reward_mint.to_account_info(),
                to: self.buyer_reward_ata.to_account_info(),
                authority: self.marketplace.to_account_info(),
            },
            signer_seeds,
        );
        mint_to(cpi_ctx, reward_amount)?;

        Ok(())
    }
}
