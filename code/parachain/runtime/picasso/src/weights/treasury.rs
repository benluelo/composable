
//! Autogenerated weights for `treasury`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-01-18, STEPS: `50`, REPEAT: 10, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `442788252fb1`, CPU: `Intel(R) Xeon(R) CPU @ 3.10GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("picasso-dev"), DB CACHE: 1024

// Executed Command:
// /nix/store/a8zw7i97gjsaanq9c839pbaklnsjsqcf-composable/bin/composable
// benchmark
// pallet
// --chain=picasso-dev
// --execution=wasm
// --wasm-execution=compiled
// --wasm-instantiation-strategy=legacy-instance-reuse
// --pallet=*
// --extrinsic=*
// --steps=50
// --repeat=10
// --output=code/parachain/runtime/picasso/src/weights

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `treasury`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> treasury::WeightInfo for WeightInfo<T> {
	fn spend() -> Weight {
		Weight::from_ref_time(423_000 as u64)
	}
	// Storage: Treasury ProposalCount (r:1 w:1)
	// Storage: Treasury Proposals (r:0 w:1)
	fn propose_spend() -> Weight {
		Weight::from_ref_time(55_079_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(2 as u64))
	}
	// Storage: Treasury Proposals (r:1 w:1)
	// Storage: System Account (r:2 w:2)
	fn reject_proposal() -> Weight {
		Weight::from_ref_time(77_097_000 as u64)
			.saturating_add(T::DbWeight::get().reads(3 as u64))
			.saturating_add(T::DbWeight::get().writes(3 as u64))
	}
	// Storage: Treasury Proposals (r:1 w:0)
	// Storage: Treasury Approvals (r:1 w:1)
	/// The range of component `p` is `[0, 29]`.
	fn approve_proposal(p: u32, ) -> Weight {
		Weight::from_ref_time(21_787_000 as u64)
			// Standard Error: 6_356
			.saturating_add(Weight::from_ref_time(708_026 as u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: Treasury Approvals (r:1 w:1)
	fn remove_approval() -> Weight {
		Weight::from_ref_time(20_194_000 as u64)
			.saturating_add(T::DbWeight::get().reads(1 as u64))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
	// Storage: System Account (r:1 w:0)
	// Storage: Treasury Approvals (r:1 w:1)
	// Storage: Treasury Proposals (r:1 w:1)
	/// The range of component `p` is `[0, 30]`.
	fn on_initialize_proposals(p: u32, ) -> Weight {
		Weight::from_ref_time(48_768_000 as u64)
			// Standard Error: 51_645
			.saturating_add(Weight::from_ref_time(54_853_982 as u64).saturating_mul(p as u64))
			.saturating_add(T::DbWeight::get().reads(2 as u64))
			.saturating_add(T::DbWeight::get().reads((3 as u64).saturating_mul(p as u64)))
			.saturating_add(T::DbWeight::get().writes(1 as u64))
			.saturating_add(T::DbWeight::get().writes((3 as u64).saturating_mul(p as u64)))
	}
}
