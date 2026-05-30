use anchor_lang::{prelude::*, system_program::transfer};
use mpl_core::{instructions:: TransferV1CpiBuilder ,ID as MPL_CORE_ID};
use crate::{state::*};
use anchor_spl::{associated_token::AssociatedToken, token::{MintTo, mint_to}, token_interface::{Mint, TokenAccount, TokenInterface}};

#[derive(Accounts)]
pub struct Buy<'info> {
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
    pub marketplace: Account<'info, Marketplace>,

    #[account(
        mut,
        close = maker,  
        seeds = [b"listing", listing.asset.key().as_ref()],
        bump = listing.bump,
        has_one = maker,
        has_one = asset
    )]
    pub listing: Account<'info, Listing>,
    #[account(
        seeds = [b"treasury", marketplace.key().as_ref()],
        bump
    )]
    pub treasury: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"rewards", marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
        mint::decimals = 6,
        mint::authority = marketplace
    )]
    pub reward_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = reward_mint,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_reward_ata: InterfaceAccount<'info, TokenAccount>,
    ///CHECK: validated in cpi by mplCore
    #[account(
        address = MPL_CORE_ID,
    )]
    pub mpl_core_program: UncheckedAccount<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    
    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

impl <'info> Buy <'info>{

    pub fn send_sol(&mut self)->Result<()>{
        // Transfer payment from taker to maker (minus marketplace fee)
        let price = self.listing.price;
        let fee = (price as u128)
        .checked_mul(self.marketplace.fee as u128)
        .unwrap()
        .checked_div(10000)
        .unwrap() as u64;
        
        let maker_amount = price.checked_sub(fee).unwrap();

        // Transfer lamports from taker to maker
        let price_ctx = CpiContext::new(
            self.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: self.taker.to_account_info(),
                to: self.maker.to_account_info(),
            },
        );
        transfer(price_ctx, maker_amount)?;

        let fee_ctx = CpiContext::new(
            self.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: self.taker.to_account_info(),
                to: self.treasury.to_account_info(),
            },
        );
        transfer(fee_ctx, fee)

    }
    pub fn recieve_nft(&mut self)->Result<()>{
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

    pub fn distribute_rewards(&mut self)->Result<()>{
        let price = self.listing.price;
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
                to: self.taker_reward_ata.to_account_info(),
                authority: self.marketplace.to_account_info(),
            },
            signer_seeds
        );
        mint_to(cpi_ctx, reward_amount)?;

        Ok(())
    }
}