use anchor_lang::prelude::*;
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

use crate::state::Listing;
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Delist<'info> {
	#[account(mut)]
	pub maker: Signer<'info>,
	#[account(
		mut,
		seeds = [b"listing", asset.key().as_ref()],
		bump = listing.bump,
		close = maker
	)]
	pub listing: Account<'info, Listing>,
	///CHECK: validated in cpi by mplCore
	#[account(mut)]
	pub asset: UncheckedAccount<'info>,
	///CHECK: validated in cpi
	#[account(mut)]
	pub collection: Option<UncheckedAccount<'info>>,
	///CHECK: validated in cpi by mplCore
	#[account(
		address = MPL_CORE_ID,
	)]
	pub mpl_core_program: UncheckedAccount<'info>,
	pub system_program: Program<'info, System>,
}

impl<'info> Delist<'info> {
	pub fn delist(&mut self) -> Result<()> {
		// Verify that the maker is the original lister
		require_eq!(
			self.maker.key(),
			self.listing.maker,
			ErrorCode::Unauthorized
		);

		// Transfer the asset back from the listing PDA to the maker
		TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
			.asset(&self.asset.to_account_info())
			.collection(self.collection.as_ref().map(|c| c.as_ref()))
			.payer(&self.maker.to_account_info())
			.authority(Some(&self.listing.to_account_info()))
			.new_owner(&self.maker.to_account_info())
			.system_program(Some(&self.system_program.to_account_info()))
			.invoke_signed(&[&[
				b"listing",
				self.asset.key().as_ref(),
				&[self.listing.bump],
			]])?;

		Ok(())
	}
}