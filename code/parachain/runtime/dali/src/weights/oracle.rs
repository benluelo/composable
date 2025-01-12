
//! Autogenerated weights for `oracle`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-12-16, STEPS: `50`, REPEAT: 10, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `fde3d2d43403`, CPU: `Intel(R) Xeon(R) CPU @ 2.20GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dali-dev"), DB CACHE: 1024

// Executed Command:
// /nix/store/y1z2mfgy9msqas77hhxszf78hqg6mx5y-composable/bin/composable
// benchmark
// pallet
// --chain=dali-dev
// --execution=wasm
// --wasm-execution=compiled
// --wasm-instantiation-strategy=legacy-instance-reuse
// --pallet=*
// --extrinsic=*
// --steps=50
// --repeat=10
// --output=code/parachain/runtime/dali/src/weights

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `oracle`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> oracle::WeightInfo for WeightInfo<T> {
	// Storage: Oracle AssetsCount (r:1 w:1)
	// Storage: Oracle RewardTrackerStore (r:1 w:1)
	// Storage: Oracle AssetsInfo (r:1 w:1)
	fn add_asset_and_info() -> Weight {
		Weight::from_ref_time(44_953_000_u64)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: Oracle RewardTrackerStore (r:1 w:1)
	fn adjust_rewards() -> Weight {
		Weight::from_ref_time(45_098_000_u64)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	// Storage: Oracle ControllerToSigner (r:1 w:1)
	// Storage: Oracle SignerToController (r:1 w:1)
	// Storage: Oracle OracleStake (r:1 w:1)
	fn set_signer() -> Weight {
		Weight::from_ref_time(141_691_000_u64)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	// Storage: Oracle ControllerToSigner (r:1 w:0)
	// Storage: Oracle OracleStake (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	fn add_stake() -> Weight {
		Weight::from_ref_time(132_493_000_u64)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	// Storage: Oracle ControllerToSigner (r:1 w:0)
	// Storage: Oracle OracleStake (r:1 w:1)
	// Storage: Oracle DeclaredWithdraws (r:0 w:1)
	fn remove_stake() -> Weight {
		Weight::from_ref_time(56_074_000_u64)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	// Storage: Oracle ControllerToSigner (r:1 w:1)
	// Storage: Oracle DeclaredWithdraws (r:1 w:1)
	// Storage: System Account (r:1 w:0)
	// Storage: Oracle SignerToController (r:0 w:1)
	fn reclaim_stake() -> Weight {
		Weight::from_ref_time(65_348_000_u64)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	// Storage: Oracle OracleStake (r:1 w:0)
	// Storage: Oracle Prices (r:1 w:0)
	// Storage: Oracle AssetsInfo (r:1 w:0)
	// Storage: Oracle AnswerInTransit (r:1 w:1)
	// Storage: Oracle PrePrices (r:1 w:1)
	/// The range of component `p` is `[1, 25]`.
	fn submit_price(p: u32, ) -> Weight {
		Weight::from_ref_time(80_629_000_u64)
			// Standard Error: 88_000
			.saturating_add(Weight::from_ref_time(385_000_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(5_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	// Storage: Oracle PrePrices (r:1 w:1)
	// Storage: Oracle AnswerInTransit (r:1 w:1)
	/// The range of component `p` is `[1, 25]`.
	fn update_pre_prices(p: u32, ) -> Weight {
		Weight::from_ref_time(18_698_000_u64)
			// Standard Error: 31_000
			.saturating_add(Weight::from_ref_time(392_000_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	// Storage: Oracle PriceHistory (r:1 w:1)
	// Storage: Oracle SignerToController (r:1 w:0)
	// Storage: Oracle AnswerInTransit (r:1 w:1)
	// Storage: Oracle RewardTrackerStore (r:1 w:0)
	// Storage: Oracle Prices (r:0 w:1)
	// Storage: Oracle PrePrices (r:0 w:1)
	/// The range of component `p` is `[1, 25]`.
	fn update_price(p: u32, ) -> Weight {
		Weight::from_ref_time(45_137_000_u64)
			// Standard Error: 118_000
			.saturating_add(Weight::from_ref_time(6_024_000_u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
}
