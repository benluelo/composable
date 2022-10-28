
//! Autogenerated weights for `utility`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-10-26, STEPS: `2`, REPEAT: 2, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `dev`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dali-dev"), DB CACHE: 1024

// Executed Command:
// /nix/store/0apddjbvqpg9m33hfhxzxsja22scw95c-composable/bin/composable
// benchmark
// pallet
// --chain=dali-dev
// --execution=wasm
// --wasm-execution=compiled
// --wasm-instantiation-strategy=legacy-instance-reuse
// --pallet=*
// --extrinsic=*
// --steps=2
// --repeat=2
// --output=code/parachain/runtime/dali/src/weights

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `utility`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> utility::WeightInfo for WeightInfo<T> {
	// Storage: CallFilter DisabledCalls (r:1 w:0)
	/// The range of component `c` is `[0, 1000]`.
	fn batch(c: u32, ) -> Weight {
		(21_938_000 as Weight)
			// Standard Error: 32_000
			.saturating_add((5_925_000 as Weight).saturating_mul(c as Weight))
	}
	// Storage: CallFilter DisabledCalls (r:1 w:0)
	fn as_derivative() -> Weight {
		(15_500_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
	// Storage: CallFilter DisabledCalls (r:1 w:0)
	/// The range of component `c` is `[0, 1000]`.
	fn batch_all(c: u32, ) -> Weight {
		(18_031_000 as Weight)
			// Standard Error: 37_000
			.saturating_add((6_129_000 as Weight).saturating_mul(c as Weight))
	}
	fn dispatch_as() -> Weight {
		(26_875_000 as Weight)
	}
	// Storage: CallFilter DisabledCalls (r:1 w:0)
	/// The range of component `c` is `[0, 1000]`.
	fn force_batch(c: u32, ) -> Weight {
		(28_837_000 as Weight)
			// Standard Error: 47_000
			.saturating_add((6_022_000 as Weight).saturating_mul(c as Weight))
	}
}
