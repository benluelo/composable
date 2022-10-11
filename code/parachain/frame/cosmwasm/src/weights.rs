
//! Autogenerated weights for `cosmwasm`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-10-11, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `dev`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dali-dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/composable
// benchmark
// pallet
// --chain
// dali-dev
// --execution=wasm
// --wasm-execution=compiled
// --wasm-instantiation-strategy=legacy-instance-reuse
// --pallet=cosmwasm
// --extrinsic=*
// --steps=50
// --repeat=20
// --output
// parachain/frame/cosmwasm/src/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(trivial_numeric_casts)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
	fn upload(n: usize, ) -> Weight;
	fn db_read() -> Weight;
	fn db_read_other_contract() -> Weight;
	fn db_write() -> Weight;
	fn db_scan() -> Weight;
	fn db_next() -> Weight;
	fn db_remove() -> Weight;
	fn balance() -> Weight;
	fn transfer() -> Weight;
	fn instantiate() -> Weight;
	fn execute() -> Weight;
	fn set_contract_meta() -> Weight;
	fn running_contract_meta() -> Weight;
	fn contract_meta() -> Weight;
	fn addr_validate() -> Weight;
	fn addr_canonicalize() -> Weight;
	fn addr_humanize() -> Weight;
	fn secp256k1_recover_pubkey() -> Weight;
	fn secp256k1_verify() -> Weight;
	fn ed25519_verify() -> Weight;
	fn ed25519_batch_verify() -> Weight;
	fn continue_instantiate() -> Weight;
	fn continue_execute() -> Weight;
	fn continue_migrate() -> Weight;
	fn query_info() -> Weight;
	fn query_continuation() -> Weight;
	fn query_raw() -> Weight;
}

/// Weight functions for `cosmwasm`.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Cosmwasm CodeHashToId (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: Cosmwasm CurrentCodeId (r:1 w:1)
	// Storage: Cosmwasm PristineCode (r:0 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:0 w:1)
	// Storage: Cosmwasm CodeIdToInfo (r:0 w:1)
	/// The range of component `n` is `[1, 514288]`.
	fn upload(n: usize, ) -> Weight {
		(514_403_000 as Weight)
			// Standard Error: 0
			.saturating_add((46_000 as Weight).saturating_mul(n as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm ContractToInfo (r:1 w:1)
	// Storage: Cosmwasm CurrentNonce (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	fn instantiate() -> Weight {
		(679_248_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: Cosmwasm ContractToInfo (r:1 w:0)
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	fn execute() -> Weight {
		(656_665_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	// Storage: unknown [0xe9a804b2e527fd3601d2ffc0bb023cd668656c6c6f20776f726c64] (r:1 w:0)
	fn db_read() -> Weight {
		(195_874_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	// Storage: unknown [0xe9a804b2e527fd3601d2ffc0bb023cd668656c6c6f20776f726c64] (r:1 w:0)
	fn db_read_other_contract() -> Weight {
		(192_458_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	// Storage: unknown [0x46fb7408d4f285228f4af516ea25851b68656c6c6f] (r:1 w:1)
	fn db_write() -> Weight {
		(190_667_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	fn db_scan() -> Weight {
		(185_166_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	// Storage: unknown [0x] (r:1 w:0)
	fn db_next() -> Weight {
		(283_125_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	// Storage: unknown [0x46fb7408d4f285228f4af516ea25851b68656c6c6f] (r:1 w:1)
	fn db_remove() -> Weight {
		(191_374_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: Tokens Accounts (r:1 w:0)
	fn balance() -> Weight {
		(2_875_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
	fn transfer() -> Weight {
		(167_000 as Weight)
	}
	// Storage: Cosmwasm ContractToInfo (r:1 w:1)
	fn set_contract_meta() -> Weight {
		(5_500_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	fn running_contract_meta() -> Weight {
		(185_124_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Cosmwasm ContractToInfo (r:1 w:0)
	fn contract_meta() -> Weight {
		(3_875_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
	fn addr_validate() -> Weight {
		(833_000 as Weight)
	}
	fn addr_canonicalize() -> Weight {
		(792_000 as Weight)
	}
	fn addr_humanize() -> Weight {
		(167_000 as Weight)
	}
	fn secp256k1_recover_pubkey() -> Weight {
		(35_749_000 as Weight)
	}
	fn secp256k1_verify() -> Weight {
		(250_000 as Weight)
	}
	fn ed25519_verify() -> Weight {
		(37_875_000 as Weight)
	}
	fn ed25519_batch_verify() -> Weight {
		(75_542_000 as Weight)
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	// Storage: Cosmwasm ContractToInfo (r:1 w:1)
	// Storage: Cosmwasm CurrentNonce (r:1 w:1)
	fn continue_instantiate() -> Weight {
		(818_248_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	// Storage: Cosmwasm ContractToInfo (r:1 w:0)
	fn continue_execute() -> Weight {
		(811_165_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	// Storage: Cosmwasm ContractToInfo (r:1 w:0)
	fn continue_migrate() -> Weight {
		(710_664_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	// Storage: Cosmwasm ContractToInfo (r:1 w:0)
	fn query_info() -> Weight {
		(191_499_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	// Storage: Cosmwasm ContractToInfo (r:1 w:0)
	fn query_continuation() -> Weight {
		(706_415_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: Cosmwasm CodeIdToInfo (r:1 w:1)
	// Storage: Cosmwasm InstrumentedCode (r:1 w:0)
	// Storage: Timestamp Now (r:1 w:0)
	// Storage: unknown [0x3a65787472696e7369635f696e646578] (r:1 w:0)
	// Storage: Cosmwasm ContractToInfo (r:1 w:0)
	// Storage: unknown [0x46fb7408d4f285228f4af516ea25851b68656c6c6f] (r:1 w:1)
	fn query_raw() -> Weight {
		(197_333_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
}
