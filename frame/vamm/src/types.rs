use frame_support::pallet_prelude::*;
use sp_core::U256;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Data relating to the state of a virtual market.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct VammState<Balance, Moment, Decimal> {
	/// The total amount of base asset present in the vamm.
	pub base_asset_reserves: Balance,

	/// The total amount of quote asset present in the vamm.
	pub quote_asset_reserves: Balance,

	/// The magnitude of the quote asset reserve.
	pub peg_multiplier: Balance,

	/// The invariant `K`.
	pub invariant: U256,

	/// Whether this market is closed or not.
	///
	/// This variable function as a signal to allow pallets who uses the
	/// Vamm to set a market as "operating as normal" or "not to be used
	/// anymore".  If the value is `None` it means the market is operating
	/// as normal, but if the value is `Some(timestamp)` it means the market
	/// is flagged to be closed and the closing action will take (or took)
	/// effect at the time `timestamp`.
	pub closed: Option<Moment>,

	/// The time weighted average price of
	/// [`base`](composable_traits::vamm::AssetType::Base) asset w.r.t.
	/// [`quote`](composable_traits::vamm::AssetType::Quote) asset.  If
	/// wanting to get `quote_asset_twap`, just call
	/// `base_asset_twap.reciprocal()` as those values should always be
	/// reciprocal of each other. For more information about computing the
	/// reciprocal, please check
	/// [`reciprocal`](sp_runtime::FixedPointNumber::reciprocal).
	pub base_asset_twap: Decimal,

	/// The timestamp for the last update of
	/// [`base_asset_twap`](VammState::base_asset_twap).
	pub twap_timestamp: Moment,

	/// The frequency with which the vamm must have its funding rebalanced.
	/// (Used only for twap calculations.)
	pub twap_period: Moment,
}

/// Represents the direction a of a position.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum SwapDirection {
	/// Adding an asset to the vamm, receiving the other in return.
	Add,
	/// Removing an asset from the vamm, giving the other in return.
	Remove,
}
