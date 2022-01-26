pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use codec::Codec;
	use composable_traits::{
		currency::LocalAssets,
		math::SafeArithmetic,
		oracle::{Oracle, Price},
		vault::Vault,
	};
	use frame_support::pallet_prelude::*;
	use sp_runtime::{
		helpers_128bit::multiply_by_rational, ArithmeticError, DispatchError, FixedPointNumber,
		FixedU128,
	};
	use sp_std::fmt::Debug;

	use crate::mocks::{currency::CurrencyId, Balance};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type VaultId: Clone + Codec + Debug + PartialEq + Default + Parameter;
		type Vault: Vault<AssetId = CurrencyId, VaultId = Self::VaultId, Balance = Balance>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn btc_value)]
	// FIXME: Temporary fix to get CI to pass, separate PRs will be made per pallet to refactor to
	// use OptionQuery instead
	#[allow(clippy::disallowed_type)]
	pub type BTCValue<T: Config> = StorageValue<_, u128, ValueQuery>;

	impl<T: Config> Pallet<T> {
		pub fn get_price(
			asset: CurrencyId,
			amount: Balance,
		) -> Result<Price<Balance, ()>, DispatchError> {
			<Self as Oracle>::get_price(asset, amount)
		}
		pub fn set_btc_price(price: u128) {
			BTCValue::<T>::set(price)
		}
	}

	impl<T: Config> Oracle for Pallet<T> {
		type AssetId = CurrencyId;
		type Balance = Balance;
		type Timestamp = ();
		type LocalAssets = ();

		fn get_price(
			asset: Self::AssetId,
			amount: Balance,
		) -> Result<Price<Self::Balance, Self::Timestamp>, DispatchError> {
			let derive_price = |p: u128, a: u128| {
				let e = 10_u128
					.checked_pow(Self::LocalAssets::decimals(asset)?)
					.ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
				let price = multiply_by_rational(p, a, e)
					.map_err(|_| DispatchError::Arithmetic(ArithmeticError::Overflow))?;
				Ok(Price { price, block: () })
			};

			#[allow(clippy::inconsistent_digit_grouping)] // values are in cents
			match asset {
				/* NOTE(hussein-aitlahcen)
					Ideally we would have all the static currency quoted against USD cents on chain.
					So that we would be able to derive LP tokens price.
				*/
				CurrencyId::USDT => Ok(Price { price: amount, block: () }),
				CurrencyId::PICA => derive_price(10_00, amount),
				CurrencyId::BTC => derive_price(Self::btc_value(), amount),
				CurrencyId::ETH => derive_price(3400_00, amount),
				CurrencyId::LTC => derive_price(180_00, amount),

				/* NOTE(hussein-aitlahcen)
					If we want users to be able to consider LP tokens as currency,
					the oracle should know the whole currency system in order to
					recursively resolve the price of an LP token generated by an
					arbitrary level of vaults.

					The base asset represented by the level 0 (out of LpToken case)
					should have a price defined.

					One exception can occur if the LP token hasn't been generated by a vault.
				*/
				x @ CurrencyId::LpToken(_) => {
					let vault = T::Vault::token_vault(x)?;
					let base = T::Vault::asset_id(&vault)?;
					let Price { price, block } = Self::get_price(base, amount)?;
					let rate = T::Vault::stock_dilution_rate(&vault)?;
					let derived = rate
						.checked_mul_int(price)
						.ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
					Ok(Price { price: derived, block })
				},
			}
		}

		fn get_twap(
			_of: Self::AssetId,
			_weights: Vec<u128>,
		) -> Result<Self::Balance, DispatchError> {
			Ok(0_u32.into())
		}

		fn get_ratio(
			pair: composable_traits::defi::CurrencyPair<Self::AssetId>,
		) -> Result<sp_runtime::FixedU128, DispatchError> {
			let base: u128 = Self::get_price(pair.base, (10_u32 ^ 12).into())?.price.into();
			let quote: u128 = Self::get_price(pair.quote, (10_u32 ^ 12).into())?.price.into();
			let base = FixedU128::saturating_from_integer(base);
			let quote = FixedU128::saturating_from_integer(quote);
			Ok(base.safe_div(&quote)?)
		}

		fn get_price_inverse(
			asset_id: Self::AssetId,
			amount: Self::Balance,
		) -> Result<Self::Balance, DispatchError> {
			let price = Self::get_price(asset_id, 10 ^ 12)?;
			let inversed = amount / price.price / 10 ^ 12;
			Ok(inversed)
		}
	}
}
