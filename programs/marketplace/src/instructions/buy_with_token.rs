use anchor_lang::prelude::*;
use mpl_core::{instructions:: TransferV1CpiBuilder ,ID as MPL_CORE_ID};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, MintTo},
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use crate::{state::*, error::ErrorCode};

#[derive(Accounts)]
pub struct BuyWithToken<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,
    ///CHECK: validated in cpi by mplCore
    #[account(mut)]
    pub maker: UncheckedAccount<'info>,
    ///CHECK: validated in cpi by mplCore
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,
    ///CHECK: validated in cpi
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
        has_one = asset,
        has_one = payment_mint
    )]
    pub listing: Box<Account<'info, Listing>>,

    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = taker,
        seeds = [b"treasury", marketplace.key().as_ref(), payment_mint.key().as_ref()],
        bump,
        token::mint = payment_mint,
        token::authority = marketplace,
        token::token_program = token_program
    )]
    pub treasury_payment_account: Box<InterfaceAccount<'info, TokenAccount>>,

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
        payer = taker,
        associated_token::mint = reward_mint,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_reward_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    ///CHECK: validated in cpi by mplCore
    #[account(
        address = MPL_CORE_ID,
    )]
    pub mpl_core_program: UncheckedAccount<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

impl <'info> BuyWithToken <'info> {
    pub fn send_tokens(&mut self) -> Result<()> {
        require!(
            self.listing.payment_mint == self.payment_mint.key(),
            ErrorCode::MintMismatch
        );

        let price = self.listing.price;
        let fee = (price as u128)
            .checked_mul(self.marketplace.fee as u128)
            .unwrap()
            .checked_div(10000)
            .unwrap() as u64;
        let maker_amount = price.checked_sub(fee).unwrap();
        let decimals = self.payment_mint.decimals;

        let maker_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            TransferChecked {
                from: self.taker_payment_ata.to_account_info(),
                mint: self.payment_mint.to_account_info(),
                to: self.maker_payment_ata.to_account_info(),
                authority: self.taker.to_account_info(),
            },
        );
        transfer_checked(maker_ctx, maker_amount, decimals)?;

        let fee_ctx = CpiContext::new(
            self.token_program.to_account_info(),
            TransferChecked {
                from: self.taker_payment_ata.to_account_info(),
                mint: self.payment_mint.to_account_info(),
                to: self.treasury_payment_account.to_account_info(),
                authority: self.taker.to_account_info(),
            },
        );
        transfer_checked(fee_ctx, fee, decimals)
    }

    pub fn recieve_nft(&mut self) -> Result<()> {
        let asset_key = self.asset.key();
        let bump = self.listing.bump;
        let seed: &[&[u8]] = &[b"listing", asset_key.as_ref(), &[bump]];
        let signer_seeds = &[seed];

        TransferV1CpiBuilder::new(
            &self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(self.collection.as_ref().map(|c|c.as_ref()))
            .payer(&self.taker.to_account_info())
            .authority(Some(&self.listing.to_account_info()))
            .new_owner(&self.taker.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(signer_seeds)?;

        Ok(())
    }

    pub fn distribute_rewards(&mut self) -> Result<()> {
        let price = self.listing.price;
        let reward_amount = (price as u128)
            .checked_mul(100)
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
                to: self.taker_reward_ata.to_account_info(),
                authority: self.marketplace.to_account_info(),
            },
            signer_seeds
        );
        mint_to(cpi_ctx, reward_amount)?;

        Ok(())
    }
}
