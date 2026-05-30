use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid fee: Fee must be less than or equal to 10000 basis points)")]
    InvalidFee,
    #[msg("Invalid name: Name must be less than or equal to 32 characters)")]
    InvalidName,
    #[msg("Unauthorized: Only the maker can delist this item)")]
    Unauthorized,
    #[msg("Unauthorized: Only the admin can perform this action")]
    UnauthorizedAdmin,
    #[msg("Invalid offer: amount must be non-zero and positive")]
    InvalidOfferAmount,
    #[msg("Unauthorized: Only the offer maker can cancel this offer")]
    UnauthorizedOfferCancellation,
    #[msg("Offer asset mismatch")]
    OfferAssetMismatch,
    #[msg("Payment mint mismatch for this listing")]
    MintMismatch,
}
