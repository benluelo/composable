
//! Autogenerated weights for `utility`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-01-19, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dali-dev"), DB CACHE: 128

// Executed Command:
// ./target/release/composable
// benchmark
// --chain=dali-dev
// --execution=wasm
// --wasm-execution=compiled
// --pallet=utility
// --extrinsic=*
// --steps=50
// --repeat=20
// --raw
// --output=runtime/dali/src/weights

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `utility`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> utility::WeightInfo for WeightInfo<T> {
	// Storage: CallFilter DisabledCalls (r:1 w:0)
	fn batch(c: u32, ) -> Weight {
		(19_222_000 as Weight)
			// Standard Error: 6_000
			.saturating_add((9_465_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
	// Storage: CallFilter DisabledCalls (r:1 w:0)
	fn as_derivative() -> Weight {
		(8_511_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
	// Storage: CallFilter DisabledCalls (r:1 w:0)
	fn batch_all(c: u32, ) -> Weight {
		(43_258_000 as Weight)
			// Standard Error: 12_000
			.saturating_add((10_000_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
	fn dispatch_as() -> Weight {
		(17_506_000 as Weight)
	}
}
