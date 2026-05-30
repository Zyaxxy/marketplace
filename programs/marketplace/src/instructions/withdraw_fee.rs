use anchor_lang::{prelude::*, system_program::transfer};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::{state::Marketplace, error::ErrorCode};

#[derive(Accounts)]
pub struct WithdrawFee<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Box<Account<'info, Marketplace>>,

    // SOL treasury and recipient
    #[account(
        mut,
        seeds = [b"treasury", marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,
    #[account(mut)]
    pub to: SystemAccount<'info>,

    // SPL token fields (must be provided when withdrawing tokens)
    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        seeds = [b"treasury", marketplace.key().as_ref(), payment_mint.key().as_ref()],
        bump,
        token::mint = payment_mint,
        token::authority = marketplace,
        token::token_program = token_program
    )]
    pub treasury_payment_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut)]
    pub to_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

impl<'info> WithdrawFee<'info> {
    pub fn withdraw_sol(&mut self, amount: u64) -> Result<()> {
        require!(self.admin.key() == self.marketplace.admin, ErrorCode::UnauthorizedAdmin);

        let marketplace_key = self.marketplace.key();
        let seeds = &[
            b"treasury".as_ref(),
            marketplace_key.as_ref(),
            &[self.marketplace.treasury_bump],
        ];
        let signer_seeds = &[seeds.as_ref()];

        let cpi_ctx = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: self.treasury.to_account_info(),
                to: self.to.to_account_info(),
            },
            signer_seeds,
        );

        transfer(cpi_ctx, amount)
    }

    pub fn withdraw_token(&mut self, amount: u64) -> Result<()> {
        require!(self.admin.key() == self.marketplace.admin, ErrorCode::UnauthorizedAdmin);

        let seeds = &[
            b"marketplace".as_ref(),
            self.marketplace.name.as_str().as_bytes(),
            &[self.marketplace.bump],
        ];
        let signer_seeds = &[seeds.as_ref()];

        let decimals = self.payment_mint.decimals;

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            TransferChecked {
                from: self.treasury_payment_account.to_account_info(),
                mint: self.payment_mint.to_account_info(),
                to: self.to_payment_ata.to_account_info(),
                authority: self.marketplace.to_account_info(),
            },
            signer_seeds,
        );

        anchor_spl::token_interface::transfer_checked(cpi_ctx, amount, decimals)
    }
}
