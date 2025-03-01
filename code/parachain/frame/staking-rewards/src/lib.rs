//! Implements staking rewards protocol.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
	not(test),
	deny(
		clippy::disallowed_methods,
		clippy::disallowed_types,
		clippy::indexing_slicing,
		clippy::todo,
		clippy::unwrap_used,
		clippy::panic
	)
)]
#![deny(
	bad_style,
	bare_trait_objects,
	improper_ctypes,
	non_shorthand_field_patterns,
	no_mangle_generic_items,
	overflowing_literals,
	path_statements,
	patterns_in_fns_without_body,
	private_in_public,
	unconditional_recursion,
	unused_allocation,
	unused_comparisons,
	unused_parens,
	while_true,
	trivial_casts,
	trivial_numeric_casts,
	unused_extern_crates,
	clippy::unseparated_literal_suffix,
	clippy::disallowed_types
)]

#[cfg(any(test, feature = "runtime-benchmarks"))]
mod benchmarking;

#[cfg(test)]
pub(crate) mod runtime;

#[cfg(test)]
mod test;
#[cfg(any(feature = "runtime-benchmarks", test))]
pub(crate) mod test_helpers;

mod validation;

pub mod prelude;
pub mod weights;

use composable_support::math::safe::{SafeDiv, SafeMul, SafeSub};
use composable_traits::staking::{Reward, RewardUpdate};
use core::{
	cmp,
	cmp::Ordering,
	num::{NonZeroU128, NonZeroU64},
	ops::Div,
};
use frame_support::{
	traits::{
		fungibles::{Inspect as FungiblesInspect, InspectHold, MutateHold, Transfer},
		Defensive, DefensiveSaturating, UnixTime,
	},
	BoundedBTreeMap,
};

use crate::prelude::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	pub use crate::weights::WeightInfo;

	use composable_support::{
		math::safe::{SafeAdd, SafeDiv, SafeMul, SafeSub},
		validation::{validators::GeOne, TryIntoValidated, Validated},
	};
	use composable_traits::{
		currency::{BalanceLike, CurrencyFactory},
		fnft::{FinancialNft, FinancialNftProtocol},
		staking::{
			lock::LockConfig, RewardPoolConfiguration::RewardRateBasedIncentive, RewardRatePeriod,
		},
		time::{DurationSeconds, ONE_MONTH, ONE_WEEK},
	};
	use frame_support::{
		defensive,
		traits::{
			fungibles::{
				Inspect as FungiblesInspect, InspectHold as FungiblesInspectHold,
				Mutate as FungiblesMutate, MutateHold as FungiblesMutateHold,
				Transfer as FungiblesTransfer,
			},
			tokens::{
				nonfungibles::{
					Create as NonFungiblesCreate, Inspect as NonFungiblesInspect,
					Mutate as NonFungiblesMutate,
				},
				WithdrawConsequence,
			},
			Defensive, DefensiveSaturating, TryCollect, UnixTime,
		},
		transactional, BoundedBTreeMap, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{GetByKey, LockIdentifier, MultiLockableCurrency};
	use sp_arithmetic::{
		fixed_point::{FixedPointNumber, FixedU64},
		Permill,
	};
	use sp_runtime::{
		traits::{AccountIdConversion, BlockNumberProvider, One},
		ArithmeticError, PerThing,
	};
	use sp_std::{fmt::Debug, ops::Mul, vec, vec::Vec};

	use crate::{
		accumulate_rewards_hook, add_to_rewards_pot, claim_of_stake, prelude::*,
		update_rewards_pool, validation::ValidSplitRatio,
	};

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config> {
		/// Pool with specified id `T::AssetId` was created successfully by `T::AccountId`.
		RewardPoolCreated {
			/// The staked asset of the pool, also used as the pool's id.
			pool_id: T::AssetId,
			/// Owner of the pool.
			owner: T::AccountId,
			/// End block
			end_block: T::BlockNumber,
		},
		/// Pool with specified id `T::AssetId` has started accumulating rewards.
		RewardPoolStarted {
			pool_id: T::AssetId,
		},
		Staked {
			/// Id of the pool that was staked in.
			pool_id: T::AssetId,
			/// Owner of the stake.
			owner: T::AccountId,
			/// The amount that was staked.
			amount: T::Balance,
			/// Duration of stake.
			duration_preset: DurationSeconds,
			/// FNFT Collection Id
			fnft_collection_id: T::AssetId,
			/// FNFT Instance Id
			fnft_instance_id: T::FinancialNftInstanceId,
			/// Reward multiplier
			reward_multiplier: FixedU64,
			// REVIEW(benluelo) is this required to be in the event?
			keep_alive: bool,
		},
		Claimed {
			/// Owner of the stake.
			owner: T::AccountId,
			/// FNFT Collection Id
			fnft_collection_id: T::AssetId,
			/// FNFT Instance Id
			fnft_instance_id: T::FinancialNftInstanceId,
		},
		StakeAmountExtended {
			/// FNFT Collection Id
			fnft_collection_id: T::AssetId,
			/// FNFT Instance Id
			fnft_instance_id: T::FinancialNftInstanceId,
			/// Extended amount
			amount: T::Balance,
		},
		Unstaked {
			/// Owner of the stake.
			owner: T::AccountId,
			/// FNFT Collection Id
			fnft_collection_id: T::AssetId,
			/// FNFT Instance Id
			fnft_instance_id: T::FinancialNftInstanceId,
			/// The amount slashed if the user unstaked early
			slash: Option<T::Balance>,
		},
		/// A staking position was split.
		SplitPosition {
			positions: Vec<(T::AssetId, T::FinancialNftInstanceId, BalanceOf<T>)>,
		},
		/// Reward transfer event.
		RewardTransferred {
			from: T::AccountId,
			pool_id: T::AssetId,
			reward_currency: T::AssetId,
			/// amount of reward currency transferred.
			reward_increment: T::Balance,
		},
		RewardAccumulationHookError {
			pool_id: T::AssetId,
			asset_id: T::AssetId,
			error: RewardAccumulationHookError,
		},
		RewardPoolUpdated {
			pool_id: T::AssetId,
		},
		RewardsPotIncreased {
			pool_id: T::AssetId,
			asset_id: T::AssetId,
			amount: T::Balance,
		},
		UnstakeRewardSlashed {
			pool_id: T::AssetId,
			owner: AccountIdOf<T>,
			fnft_instance_id: FinancialNftInstanceIdOf<T>,
			reward_asset_id: T::AssetId,
			amount_slashed: T::Balance,
		},
		RewardPoolPaused {
			pool_id: T::AssetId,
			asset_id: T::AssetId,
		},
		RewardPoolResumed {
			pool_id: T::AssetId,
			asset_id: T::AssetId,
		},
	}

	#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
	pub enum RewardAccumulationHookError {
		BackToTheFuture,
		Overflow,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Error when creating reward configs.
		RewardConfigProblem,
		/// AssetId is invalid, asset IDs must be greater than 0
		InvalidAssetId,
		/// Reward pool already exists
		RewardsPoolAlreadyExists,
		/// The duration provided was not valid for the pool.
		DurationPresetNotFound,
		/// Too many rewarded asset types per pool violating the storage allowed.
		TooManyRewardAssetTypes,
		/// Invalid start block number provided for creating a pool.
		StartBlockMustBeAfterCurrentBlock,
		/// Invalid end block number provided for creating a pool.
		EndBlockMustBeAfterStartBlock,
		/// Unimplemented reward pool type.
		UnimplementedRewardPoolConfiguration,
		/// Rewards pool not found.
		RewardsPoolNotFound,
		/// Rewards pool has not started.
		RewardsPoolHasNotStarted,
		/// Error when creating reduction configs.
		ReductionConfigProblem,
		/// Not enough assets for a stake.
		NotEnoughAssets,
		/// No stake found for given id.
		StakeNotFound,
		/// Reward's max limit reached.
		MaxRewardLimitReached,
		/// only the owner of stake can unstake it
		OnlyStakeOwnerCanInteractWithStake,
		/// Reward asset not found in reward pool.
		RewardAssetNotFound,
		BackToTheFuture,
		/// The rewards pot for this pool is empty.
		RewardsPotEmpty,
		FnftNotFound,
		/// No duration presets were provided upon pool creation.
		// NOTE(benluelo): This should be removed once this issue gets resolved:
		// https://github.com/paritytech/substrate/issues/12257
		NoDurationPresetsProvided,
		/// Slashed amount of minimum reward is less than existential deposit
		SlashedAmountTooLow,
		/// Slashed amount of minimum staking amount is less than existential deposit
		SlashedMinimumStakingAmountTooLow,
		/// Staked amount is less than the minimum staking amount for the pool.
		StakedAmountTooLow,
		/// Staked amount after split is less than the minimum staking amount for the pool.
		StakedAmountTooLowAfterSplit,
		/// Some operation resulted in an arithmetic overflow.
		ArithmeticError,
	}

	pub(crate) type AssetIdOf<T> = <T as Config>::AssetId;
	pub(crate) type BalanceOf<T> = <T as Config>::Balance;
	pub(crate) type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub(crate) type FinancialNftInstanceIdOf<T> = <T as Config>::FinancialNftInstanceId;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The reward balance type.
		type Balance: Parameter
			+ Member
			+ BalanceLike
			+ FixedPointOperand
			+ From<u128>
			+ Into<u128>
			+ Zero;

		type AssetId: Parameter
			+ Member
			+ AssetIdLike
			+ MaybeSerializeDeserialize
			+ Ord
			+ From<u128>
			+ Into<u128>
			+ Copy
			+ Zero;

		// REVIEW(benluelo): Mutate::CollectionId type?
		type FinancialNft: NonFungiblesMutate<AccountIdOf<Self>>
			+ NonFungiblesCreate<
				AccountIdOf<Self>,
				CollectionId = Self::AssetId,
				ItemId = Self::FinancialNftInstanceId,
			> + FinancialNft<
				AccountIdOf<Self>,
				CollectionId = Self::AssetId,
				ItemId = Self::FinancialNftInstanceId,
			>;

		// https://github.com/rust-lang/rust/issues/52662
		type FinancialNftInstanceId: Parameter
			+ Member
			+ Copy
			+ PartialOrd
			+ Ord
			+ From<u64>
			+ Into<u64>;

		/// Is used to create staked asset per reward pool
		type CurrencyFactory: CurrencyFactory<AssetId = Self::AssetId, Balance = Self::Balance>;

		/// Dependency allowing this pallet to transfer funds from one account to another.
		type Assets: FungiblesTransfer<
				AccountIdOf<Self>,
				Balance = BalanceOf<Self>,
				AssetId = AssetIdOf<Self>,
			> + FungiblesMutate<AccountIdOf<Self>, Balance = BalanceOf<Self>, AssetId = AssetIdOf<Self>>
			+ FungiblesMutateHold<
				AccountIdOf<Self>,
				Balance = BalanceOf<Self>,
				AssetId = AssetIdOf<Self>,
			> + FungiblesInspectHold<
				AccountIdOf<Self>,
				Balance = BalanceOf<Self>,
				AssetId = AssetIdOf<Self>,
			> + MultiLockableCurrency<
				AccountIdOf<Self>,
				Balance = BalanceOf<Self>,
				CurrencyId = AssetIdOf<Self>,
			>;

		/// is used for rate based rewarding and position lock timing
		type UnixTime: UnixTime;

		/// the size of batch to take each time trying to release rewards
		#[pallet::constant]
		type ReleaseRewardsPoolsBatchSize: Get<u8>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Maximum number of staking duration presets allowed.
		#[pallet::constant]
		type MaxStakingDurationPresets: Get<u32>;

		/// Maximum number of reward configurations per pool.
		#[pallet::constant]
		type MaxRewardConfigsPerPool: Get<u32>;

		/// Required origin for reward pool creation.
		type RewardPoolCreationOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Required origin for reward pool creation.
		type RewardPoolUpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		#[pallet::constant]
		type PicaAssetId: Get<Self::AssetId>;

		#[pallet::constant]
		type XPicaAssetId: Get<Self::AssetId>;

		#[pallet::constant]
		type PbloAssetId: Get<Self::AssetId>;

		#[pallet::constant]
		type XPbloAssetId: Get<Self::AssetId>;

		#[pallet::constant]
		type PicaStakeFinancialNftCollectionId: Get<Self::AssetId>;

		#[pallet::constant]
		type PbloStakeFinancialNftCollectionId: Get<Self::AssetId>;

		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type LockId: Get<LockIdentifier>;

		// The account to send the slashed stakes to.
		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		type ExistentialDeposits: GetByKey<Self::AssetId, Self::Balance>;
	}

	/// Abstraction over RewardPoolConfiguration type
	pub(crate) type RewardPoolConfigurationOf<T> = RewardPoolConfiguration<
		AccountIdOf<T>,
		AssetIdOf<T>,
		BalanceOf<T>,
		<T as frame_system::Config>::BlockNumber,
		<T as Config>::MaxRewardConfigsPerPool,
		<T as Config>::MaxStakingDurationPresets,
	>;

	/// Abstraction over RewardPool type
	pub(crate) type RewardPoolOf<T> = RewardPool<
		AccountIdOf<T>,
		AssetIdOf<T>,
		BalanceOf<T>,
		<T as frame_system::Config>::BlockNumber,
		<T as Config>::MaxStakingDurationPresets,
		<T as Config>::MaxRewardConfigsPerPool,
	>;

	/// Abstraction over Stake type
	pub(crate) type StakeOf<T> = Stake<
		AssetIdOf<T>,
		AssetIdOf<T>, // we use AssetId as the reward pool id
		BalanceOf<T>,
		<T as Config>::MaxRewardConfigsPerPool,
	>;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type RewardPools<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, RewardPoolOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn stakes)]
	// REVIEW(benluelo): Twox128 for the hasher?
	pub type Stakes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AssetId, // collection id
		Blake2_128Concat,
		FinancialNftInstanceIdOf<T>,
		StakeOf<T>,
	>;

	#[pallet::storage]
	#[allow(clippy::disallowed_types)]
	pub(super) type RewardsPotIsEmpty<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, T::AssetId, Blake2_128Concat, T::AssetId, ()>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		_phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			let owner: T::AccountId = T::PalletId::get().into_account_truncating();
			create_default_pool::<T>(
				&owner,
				T::PicaAssetId::get(),
				T::XPicaAssetId::get(),
				T::PicaStakeFinancialNftCollectionId::get(),
			);
			create_default_pool::<T>(
				&owner,
				T::PbloAssetId::get(),
				T::XPbloAssetId::get(),
				T::PbloStakeFinancialNftCollectionId::get(),
			);
		}
	}

	fn create_default_pool<T: Config>(
		owner: &T::AccountId,
		staked_asset_id: T::AssetId,
		share_asset_id: T::AssetId,
		financial_nft_asset_id: T::AssetId,
	) {
		// TODO (vim): Review these with product
		let staking_pool: RewardPoolOf<T> = RewardPool {
			owner: owner.clone(),
			rewards: Default::default(),
			claimed_shares: T::Balance::zero(),
			start_block: T::BlockNumber::zero(),
			end_block: T::BlockNumber::zero(),
			lock: LockConfig {
				duration_presets: [
					(
						ONE_WEEK,
						FixedU64::from_rational(101, 100).try_into_validated().expect(">= 1"),
					),
					(
						ONE_MONTH,
						FixedU64::from_rational(110, 100).try_into_validated().expect(">= 1"),
					),
				]
				.into_iter()
				.try_collect()
				.expect("Genesis config must be correct; qed"),
				unlock_penalty: Default::default(),
			},
			share_asset_id,
			financial_nft_asset_id,
			minimum_staking_amount: T::Balance::from(2_000_000_u128),
		};
		RewardPools::<T>::insert(staked_asset_id, staking_pool);
		T::FinancialNft::create_collection(&financial_nft_asset_id, owner, owner)
			.expect("Genesis config must be correct; qed");
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Weight: see `begin_block`
		fn on_initialize(_: T::BlockNumber) -> Weight {
			accumulate_rewards_hook::<T>()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new reward pool based on the config.
		///
		/// Emits `RewardPoolCreated` event when successful.
		#[pallet::weight(T::WeightInfo::create_reward_pool(T::MaxRewardConfigsPerPool::get()))]
		#[transactional]
		pub fn create_reward_pool(
			origin: OriginFor<T>,
			pool_config: RewardPoolConfigurationOf<T>,
		) -> DispatchResult {
			T::RewardPoolCreationOrigin::ensure_origin(origin)?;
			let _ = <Self as ManageStaking>::create_staking_pool(pool_config)?;
			Ok(())
		}

		/// Create a new stake.
		///
		/// Emits `Staked` when successful.
		#[pallet::weight(T::WeightInfo::stake(T::MaxRewardConfigsPerPool::get()))]
		pub fn stake(
			origin: OriginFor<T>,
			pool_id: T::AssetId,
			amount: T::Balance,
			duration_preset: DurationSeconds,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let keep_alive = true;
			let _position_id =
				<Self as Staking>::stake(&owner, &pool_id, amount, duration_preset, keep_alive)?;

			Ok(())
		}

		/// Extend an existing stake.
		///
		/// Emits `StakeExtended` when successful.
		#[pallet::weight(T::WeightInfo::extend(T::MaxRewardConfigsPerPool::get()))]
		pub fn extend(
			origin: OriginFor<T>,
			fnft_collection_id: T::AssetId,
			fnft_instance_id: T::FinancialNftInstanceId,
			amount: T::Balance,
		) -> DispatchResult {
			let who = Self::ensure_stake_owner(
				ensure_signed(origin)?,
				&fnft_collection_id,
				&fnft_instance_id,
			)?;

			// TODO(benluelo): This needs to be passed in through the extrinsic
			let keep_alive = true;

			<Self as Staking>::extend(
				&who,
				(fnft_collection_id, fnft_instance_id),
				amount,
				keep_alive,
			)?;

			Ok(())
		}

		/// Remove a stake.
		///
		/// Emits `Unstaked` when successful.
		#[pallet::weight(T::WeightInfo::unstake(T::MaxRewardConfigsPerPool::get()))]
		pub fn unstake(
			origin: OriginFor<T>,
			fnft_collection_id: T::AssetId,
			fnft_instance_id: T::FinancialNftInstanceId,
		) -> DispatchResult {
			let who = Self::ensure_stake_owner(
				ensure_signed(origin)?,
				&fnft_collection_id,
				&fnft_instance_id,
			)?;

			<Self as Staking>::unstake(&who, &(fnft_collection_id, fnft_instance_id))?;

			Ok(())
		}

		/// Split a stake into two parts, by a ratio.
		///
		/// Emits `SplitPosition` when successful.
		#[pallet::weight(T::WeightInfo::split(T::MaxRewardConfigsPerPool::get()))]
		pub fn split(
			origin: OriginFor<T>,
			fnft_collection_id: T::AssetId,
			fnft_instance_id: T::FinancialNftInstanceId,
			ratio: Validated<Permill, ValidSplitRatio>,
		) -> DispatchResult {
			let who = Self::ensure_stake_owner(
				ensure_signed(origin)?,
				&fnft_collection_id,
				&fnft_instance_id,
			)?;
			<Self as Staking>::split(&who, &(fnft_collection_id, fnft_instance_id), ratio.value())?;
			Ok(())
		}

		/// Updates the reward pool configuration.
		///
		/// Emits `RewardPoolUpdated` when successful.
		#[pallet::weight(T::WeightInfo::update_rewards_pool(reward_updates.len() as u32))]
		pub fn update_rewards_pool(
			origin: OriginFor<T>,
			pool_id: T::AssetId,
			reward_updates: BoundedBTreeMap<
				AssetIdOf<T>,
				RewardUpdate<BalanceOf<T>>,
				T::MaxRewardConfigsPerPool,
			>,
		) -> DispatchResult {
			T::RewardPoolUpdateOrigin::ensure_origin(origin)?;
			update_rewards_pool::<T>(pool_id, reward_updates)
		}

		/// Claim a current reward for some position.
		///
		/// Emits `Claimed` when successful.
		#[pallet::weight(T::WeightInfo::claim(T::MaxRewardConfigsPerPool::get()))]
		pub fn claim(
			origin: OriginFor<T>,
			fnft_collection_id: T::AssetId,
			fnft_instance_id: T::FinancialNftInstanceId,
		) -> DispatchResult {
			let owner = Self::ensure_stake_owner(
				ensure_signed(origin)?,
				&fnft_collection_id,
				&fnft_instance_id,
			)?;
			<Self as Staking>::claim(&owner, &(fnft_collection_id, fnft_instance_id))?;

			Ok(())
		}

		/// Add funds to the reward pool's rewards pot for the specified asset.
		///
		/// Emits `RewardsPotIncreased` when successful.
		#[pallet::weight(T::WeightInfo::add_to_rewards_pot())]
		pub fn add_to_rewards_pot(
			origin: OriginFor<T>,
			pool_id: T::AssetId,
			asset_id: T::AssetId,
			amount: T::Balance,
			keep_alive: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			add_to_rewards_pot::<T>(who, pool_id, asset_id, amount, keep_alive)
		}
	}

	impl<T: Config> ManageStaking for Pallet<T> {
		type AccountId = T::AccountId;
		type AssetId = T::AssetId;
		type BlockNumber = <T as frame_system::Config>::BlockNumber;
		type Balance = T::Balance;
		type RewardConfigsLimit = T::MaxRewardConfigsPerPool;
		type StakingDurationPresetsLimit = T::MaxStakingDurationPresets;
		type RewardPoolId = T::AssetId;

		#[transactional]
		fn create_staking_pool(
			pool_config: RewardPoolConfigurationOf<T>,
		) -> Result<Self::RewardPoolId, DispatchError> {
			match pool_config {
				RewardRateBasedIncentive {
					owner,
					asset_id: pool_asset,
					reward_configs: initial_reward_config,
					start_block,
					end_block,
					lock,
					share_asset_id,
					financial_nft_asset_id,
					minimum_staking_amount,
				} => {
					// AssetIds must be greater than 0
					ensure!(!pool_asset.is_zero(), Error::<T>::InvalidAssetId);
					ensure!(!share_asset_id.is_zero(), Error::<T>::InvalidAssetId);
					ensure!(!financial_nft_asset_id.is_zero(), Error::<T>::InvalidAssetId);

					// now < start_block < end_block
					ensure!(
						// Exclusively greater than to prevent errors/attacks
						start_block > frame_system::Pallet::<T>::current_block_number(),
						Error::<T>::StartBlockMustBeAfterCurrentBlock
					);
					ensure!(end_block > start_block, Error::<T>::EndBlockMustBeAfterStartBlock);

					ensure!(
						!RewardPools::<T>::contains_key(pool_asset),
						Error::<T>::RewardsPoolAlreadyExists
					);

					ensure!(lock.duration_presets.len() > 0, Error::<T>::NoDurationPresetsProvided);

					let now_seconds = T::UnixTime::now().as_secs();

					let existential_deposit = T::ExistentialDeposits::get(&pool_asset);

					ensure!(
						lock.unlock_penalty.left_from_one().mul(minimum_staking_amount) >=
							existential_deposit,
						Error::<T>::SlashedMinimumStakingAmountTooLow
					);

					ensure!(
						initial_reward_config.iter().all(|(reward_asset_id, reward_config)| {
							if reward_config.reward_rate.amount > T::Balance::zero() {
								// if the reward rate amount is non-zero, then ensure that the
								// slashed amount is >= the exisistential deposit for this asset
								lock.unlock_penalty
									.left_from_one()
									.mul(reward_config.reward_rate.amount) >=
									T::ExistentialDeposits::get(reward_asset_id)
							} else {
								// otherwise, since there are no rewards, no need to check against
								// the existential deposit
								true
							}
						}),
						Error::<T>::SlashedAmountTooLow
					);

					// TODO: Replace into_iter with iter_mut once it's available
					let rewards = initial_reward_config
						.into_iter()
						.map(|(asset_id, reward_config)| {
							(asset_id, Reward::from_config(reward_config, now_seconds))
						})
						.try_collect()
						.expect("No items were added; qed;");

					RewardPools::<T>::insert(
						pool_asset,
						RewardPool {
							owner: owner.clone(),
							rewards,
							claimed_shares: T::Balance::zero(),
							start_block,
							end_block,
							lock,
							share_asset_id,
							financial_nft_asset_id,
							minimum_staking_amount,
						},
					);

					T::FinancialNft::create_collection(&financial_nft_asset_id, &owner, &owner)?;

					Self::deposit_event(Event::<T>::RewardPoolCreated {
						pool_id: pool_asset,
						owner,
						end_block,
					});

					Ok(pool_asset)
				},
				_ => Err(Error::<T>::UnimplementedRewardPoolConfiguration.into()),
			}
		}
	}

	impl<T: Config> FinancialNftProtocol for Pallet<T> {
		type ItemId = FinancialNftInstanceIdOf<T>;
		type AssetId = AssetIdOf<T>;
		type Balance = BalanceOf<T>;

		fn collection_asset_ids() -> Vec<Self::AssetId> {
			RewardPools::<T>::iter().map(|(_, pool)| pool.financial_nft_asset_id).collect()
		}

		fn value_of(
			collection: &Self::AssetId,
			instance: &Self::ItemId,
		) -> Result<Vec<(Self::AssetId, Self::Balance)>, DispatchError> {
			RewardPools::<T>::get(collection)
				.zip(Stakes::<T>::get(collection, instance))
				// This can take into account the value of assets held in the asset account as
				// well as the claimable rewards in the future when market places exists for these
				// NFTs.
				.map(|pool| vec![(pool.0.share_asset_id, pool.1.share)])
				.ok_or_else(|| DispatchError::Other(Error::<T>::StakeNotFound.into()))
		}
	}

	impl<T: Config> Staking for Pallet<T> {
		type AccountId = T::AccountId;
		type RewardPoolId = T::AssetId;
		type Balance = T::Balance;
		type PositionId = (T::AssetId, T::FinancialNftInstanceId);

		#[transactional]
		fn stake(
			who: &Self::AccountId,
			pool_id: &Self::RewardPoolId,
			amount: Self::Balance,
			duration_preset: DurationSeconds,
			keep_alive: bool,
		) -> Result<Self::PositionId, DispatchError> {
			let mut rewards_pool =
				RewardPools::<T>::try_get(pool_id).map_err(|_| Error::<T>::RewardsPoolNotFound)?;

			ensure!(amount >= rewards_pool.minimum_staking_amount, Error::<T>::StakedAmountTooLow);

			ensure!(
				rewards_pool.start_block <= frame_system::Pallet::<T>::current_block_number(),
				Error::<T>::RewardsPoolHasNotStarted
			);

			let reward_multiplier = Self::reward_multiplier(&rewards_pool, duration_preset)
				.ok_or(Error::<T>::DurationPresetNotFound)?;

			ensure!(
				matches!(
					T::Assets::can_withdraw(*pool_id, who, amount),
					WithdrawConsequence::Success
				),
				Error::<T>::NotEnoughAssets
			);

			let awarded_shares = Self::boosted_amount(reward_multiplier, amount)?;

			let (rewards, reductions) =
				Self::compute_rewards_and_reductions(awarded_shares, &rewards_pool)?;

			rewards_pool.rewards = rewards;

			let fnft_collection_id = rewards_pool.financial_nft_asset_id;
			let fnft_instance_id = T::FinancialNft::get_next_nft_id(&fnft_collection_id)?;
			let fnft_account =
				T::FinancialNft::asset_account(&fnft_collection_id, &fnft_instance_id);

			let new_position = StakeOf::<T> {
				reward_pool_id: *pool_id,
				stake: amount,
				share: awarded_shares,
				reductions,
				lock: lock::Lock {
					started_at: T::UnixTime::now().as_secs(),
					duration: duration_preset,
					// NOTE: Currently, the early unlock penalty for all stakes in a pool are the
					// same as the pool's penalty *at the time of staking*. This value is duplicated
					// to keep the stake's penalty independent from the reward pool's penalty,
					// allowing for future changes/ feature additions to penalties such as variable
					// penalties per stake (i.e. penalty affected by staked duration or something
					// similar) or updating the pool's penalty while still upholding the staking
					// contracts of existing stakers.
					unlock_penalty: rewards_pool.lock.unlock_penalty,
				},
			};

			// Move staked funds into fNFT asset account & lock the assets
			Self::transfer_stake(who, amount, *pool_id, &fnft_account, keep_alive)?;
			Self::mint_shares(rewards_pool.share_asset_id, awarded_shares, &fnft_account)?;

			// Mint the fNFT
			T::FinancialNft::mint_into(&fnft_collection_id, &fnft_instance_id, who)?;

			RewardPools::<T>::insert(pool_id, rewards_pool);
			Stakes::<T>::insert(fnft_collection_id, fnft_instance_id, new_position);

			Self::deposit_event(Event::<T>::Staked {
				pool_id: *pool_id,
				owner: who.clone(),
				amount,
				duration_preset,
				fnft_instance_id,
				fnft_collection_id,
				reward_multiplier: *reward_multiplier,
				keep_alive,
			});

			Ok((fnft_collection_id, fnft_instance_id))
		}

		#[transactional]
		fn extend(
			who: &Self::AccountId,
			(fnft_collection_id, fnft_instance_id): Self::PositionId,
			amount: Self::Balance,
			keep_alive: bool,
		) -> DispatchResult {
			Stakes::<T>::try_mutate(fnft_collection_id, fnft_instance_id, |maybe_stake| {
				let stake = maybe_stake.as_mut().ok_or(Error::<T>::StakeNotFound)?;

				RewardPools::<T>::try_mutate(stake.reward_pool_id, |maybe_rewards_pool| {
					let rewards_pool =
						maybe_rewards_pool.as_mut().ok_or(Error::<T>::RewardsPoolNotFound)?;

					ensure!(
						matches!(
							T::Assets::can_withdraw(stake.reward_pool_id, who, amount),
							WithdrawConsequence::Success
						),
						Error::<T>::NotEnoughAssets
					);

					// SAFETY: The duration preset on an existing stake should be valid in the
					// pool since it's currently not possible to modify the presets after pool
					// creation.
					let reward_multiplier = rewards_pool
						.lock
						.duration_presets
						.get(&stake.lock.duration)
						.copied()
						.defensive_unwrap_or_else(|| {
							FixedU64::one().try_into_validated().expect("1 is >= 1")
						});

					let new_shares = Self::boosted_amount(reward_multiplier, amount)?;

					let total_shares = T::Assets::total_issuance(rewards_pool.share_asset_id);

					for (reward_asset_id, reward) in &mut rewards_pool.rewards {
						let new_inflation = if total_shares.is_zero() {
							T::Balance::zero()
						} else {
							reward.total_rewards.safe_mul(&new_shares)?.safe_div(&total_shares)?
						};

						reward.total_rewards = reward.total_rewards.safe_add(&new_inflation)?;
						reward.total_dilution_adjustment =
							reward.total_dilution_adjustment.safe_add(&new_inflation)?;

						match stake.reductions.get_mut(reward_asset_id) {
							Some(previous_inflation_and_claims) => {
								*previous_inflation_and_claims =
									previous_inflation_and_claims.safe_add(&new_inflation)?;
							},
							None => {
								// REVIEW(benluelo): Is this an invariant we expect? In
								// ProtocolStaking::transfer_reward assets can be added (and is
								// currently the only way to add a new reward asset to a pool),
								// but they are not added to all existing stakes so this
								// invariant is not upheld
								defensive!("stake.reductions should contain the same assets as reward_pool.rewards");
							},
						}
					}

					let fnft_asset_account =
						T::FinancialNft::asset_account(&fnft_collection_id, &fnft_instance_id);

					Self::transfer_stake(
						who,
						amount,
						stake.reward_pool_id,
						&fnft_asset_account,
						keep_alive,
					)?;
					// only mint the new shares
					Self::mint_shares(
						rewards_pool.share_asset_id,
						new_shares,
						&fnft_asset_account,
					)?;

					Self::deposit_event(Event::<T>::StakeAmountExtended {
						amount,
						fnft_collection_id,
						fnft_instance_id,
					});

					stake.stake = stake.stake.safe_add(&amount)?;
					stake.share = stake.share.safe_add(&new_shares)?;
					stake.lock.started_at = T::UnixTime::now().as_secs();

					Ok(())
				})
			})
		}

		#[transactional]
		fn unstake(
			who: &Self::AccountId,
			(fnft_collection_id, fnft_instance_id): &Self::PositionId,
		) -> DispatchResult {
			// TODO(benluelo): Use ::take here instead of try_get and then remove
			let mut stake = Stakes::<T>::try_get(fnft_collection_id, fnft_instance_id)
				.map_err(|_| Error::<T>::StakeNotFound)?;

			let is_early_unlock = stake.lock.started_at.safe_add(&stake.lock.duration)? >=
				T::UnixTime::now().as_secs();

			// TODO(benluelo): No need to return the staked asset id here, it's the same as
			// stake.reward_pool_id
			let (asset_id, share_asset_id) =
				RewardPools::<T>::try_mutate(stake.reward_pool_id, |rewards_pool| {
					let rewards_pool =
						rewards_pool.as_mut().ok_or(Error::<T>::RewardsPoolNotFound)?;

					Self::collect_rewards(
						stake.reward_pool_id,
						rewards_pool,
						fnft_instance_id,
						&mut stake,
						who,
						is_early_unlock,
					)?;

					Ok::<_, DispatchError>((stake.reward_pool_id, rewards_pool.share_asset_id))
				})?;

			// REVIEW(benluelo): Make this logic a method on Stake
			let staked_amount_returned_to_staker = if is_early_unlock {
				stake.lock.unlock_penalty.left_from_one().mul_ceil(stake.stake)
			} else {
				stake.stake
			};

			let fnft_asset_account =
				T::FinancialNft::asset_account(fnft_collection_id, fnft_instance_id);

			T::Assets::remove_lock(T::LockId::get(), asset_id, &fnft_asset_account)?;
			T::Assets::remove_lock(T::LockId::get(), share_asset_id, &fnft_asset_account)?;
			T::Assets::transfer(
				asset_id,
				&fnft_asset_account,
				who,
				staked_amount_returned_to_staker,
				false, // pallet account doesn't need to be kept alive
			)?;

			Stakes::<T>::remove(fnft_collection_id, fnft_instance_id);

			// transfer slashed stake to the treasury
			if is_early_unlock {
				// If there is no penalty then there is nothing to burn as it will all have been
				// transferred back to the staker. burn_from isn't a noop if the amount to burn
				// is 0, hence the check
				T::Assets::transfer(
					stake.reward_pool_id,
					&fnft_asset_account,
					&T::TreasuryAccount::get(),
					// staked_amount_returned_to_staker should always be <= stake.amount as per
					// the formula used to calculate it, so this should never fail.
					// defensive_saturating_sub uses saturating_sub as a fallback if the
					// operation *were* to fail, resulting in no transfer happening (the
					// transferred amount would be 0) and an error being logged.
					stake.stake.defensive_saturating_sub(staked_amount_returned_to_staker),
					false, // pallet account, doesn't need to be kept alive
				)?;
			}
			// burn the shares
			T::Assets::burn_from(share_asset_id, &fnft_asset_account, stake.share)?;
			T::FinancialNft::burn(fnft_collection_id, fnft_instance_id, Some(who))?;

			Self::deposit_event(Event::<T>::Unstaked {
				owner: who.clone(),
				fnft_collection_id: *fnft_collection_id,
				fnft_instance_id: *fnft_instance_id,
				slash: is_early_unlock.then(|| stake.lock.unlock_penalty.mul_floor(stake.stake)),
			});

			Ok(())
		}

		// TODO(benluelo): Split this out into a separate function/file
		#[transactional]
		fn split(
			who: &Self::AccountId,
			(fnft_collection_id, existing_fnft_instance_id): &Self::PositionId,
			ratio: Permill,
		) -> Result<Self::PositionId, DispatchError> {
			let (new_fnft_instance_id, new_position) = Stakes::<T>::try_mutate(
				fnft_collection_id,
				existing_fnft_instance_id,
				|maybe_existing_position| {
					let existing_position =
						maybe_existing_position.as_mut().ok_or(Error::<T>::StakeNotFound)?;

					let left_from_one_ratio = ratio.left_from_one();

					// create the new position first, before mutating the existing position
					// mul_ceil is used for the new position, and mul_floor for the existing
					// position, that way any rounding is accounted for.
					let new_stake = left_from_one_ratio.mul_ceil(existing_position.stake);
					let new_share = left_from_one_ratio.mul_ceil(existing_position.share);

					let rewards_pool = RewardPools::<T>::get(existing_position.reward_pool_id)
						.ok_or(Error::<T>::RewardsPoolNotFound)?;

					let existing_position_stake = ratio.mul_floor(existing_position.stake);

					ensure!(
						existing_position_stake >= rewards_pool.minimum_staking_amount,
						Error::<T>::StakedAmountTooLowAfterSplit
					);
					ensure!(
						new_stake >= rewards_pool.minimum_staking_amount,
						Error::<T>::StakedAmountTooLowAfterSplit
					);

					let new_reductions = {
						let mut r = existing_position.reductions.clone();
						for (_, reduction) in &mut r {
							*reduction = left_from_one_ratio.mul_ceil(*reduction);
						}
						r
					};

					existing_position.stake = existing_position_stake;
					existing_position.share = ratio.mul_floor(existing_position.share);
					for (_, reduction) in &mut existing_position.reductions {
						*reduction = ratio.mul_floor(*reduction);
					}

					let new_fnft_instance_id =
						T::FinancialNft::get_next_nft_id(fnft_collection_id)?;
					T::FinancialNft::mint_into(
						&rewards_pool.financial_nft_asset_id,
						&new_fnft_instance_id,
						who,
					)?;

					let existing_fnft_asset_account = T::FinancialNft::asset_account(
						fnft_collection_id,
						existing_fnft_instance_id,
					);
					let new_fnft_asset_account =
						T::FinancialNft::asset_account(fnft_collection_id, &new_fnft_instance_id);

					// staked asset
					Self::split_lock(
						existing_position.reward_pool_id,
						&existing_fnft_asset_account,
						&new_fnft_asset_account,
						existing_position.stake,
						new_stake,
					)?;

					// share asset (x-token)
					Self::split_lock(
						rewards_pool.share_asset_id,
						&existing_fnft_asset_account,
						&new_fnft_asset_account,
						existing_position.share,
						new_share,
					)?;

					Self::deposit_event(Event::<T>::SplitPosition {
						positions: sp_std::vec![
							(
								*fnft_collection_id,
								*existing_fnft_instance_id,
								existing_position.stake,
							),
							(*fnft_collection_id, new_fnft_instance_id, new_stake),
						],
					});

					Ok::<_, DispatchError>((
						new_fnft_instance_id,
						Stake {
							stake: new_stake,
							share: new_share,
							reductions: new_reductions,
							reward_pool_id: existing_position.reward_pool_id,
							lock: existing_position.lock,
						},
					))
				},
			)?;

			Stakes::<T>::insert(fnft_collection_id, new_fnft_instance_id, new_position);

			Ok((*fnft_collection_id, new_fnft_instance_id))
		}

		#[transactional]
		fn claim(
			who: &Self::AccountId,
			(fnft_collection_id, fnft_instance_id): &Self::PositionId,
		) -> DispatchResult {
			Stakes::<T>::try_mutate(fnft_collection_id, fnft_instance_id, |stake| {
				let stake = stake.as_mut().ok_or(Error::<T>::StakeNotFound)?;
				RewardPools::<T>::try_mutate(stake.reward_pool_id, |rewards_pool| {
					let rewards_pool =
						rewards_pool.as_mut().ok_or(Error::<T>::RewardsPoolNotFound)?;

					Self::collect_rewards(
						stake.reward_pool_id,
						rewards_pool,
						fnft_instance_id,
						stake,
						who,
						false, // claims aren't penalized
					)?;

					Ok::<_, DispatchError>(())
				})
			})?;

			Self::deposit_event(Event::<T>::Claimed {
				owner: who.clone(),
				fnft_collection_id: *fnft_collection_id,
				fnft_instance_id: *fnft_instance_id,
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn transfer_stake(
			who: &AccountIdOf<T>,
			amount: <T as Config>::Balance,
			staked_asset_id: AssetIdOf<T>,
			fnft_account: &AccountIdOf<T>,
			keep_alive: bool,
		) -> DispatchResult {
			T::Assets::transfer(staked_asset_id, who, fnft_account, amount, keep_alive)?;
			T::Assets::set_lock(T::LockId::get(), staked_asset_id, fnft_account, amount)
		}

		/// Mint share tokens into fNFT asst account & lock the assets
		fn mint_shares(
			share_asset_id: AssetIdOf<T>,
			awarded_shares: <T as Config>::Balance,
			fnft_account: &AccountIdOf<T>,
		) -> DispatchResult {
			T::Assets::mint_into(share_asset_id, fnft_account, awarded_shares)?;
			T::Assets::set_lock(T::LockId::get(), share_asset_id, fnft_account, awarded_shares)?;
			Ok(())
		}

		/// Ensure `who` is the owner of the fNFT associated with a stake
		///
		/// # Errors
		/// * FnftNotFound - No fNFT with the provided collection and instance ID found
		/// * OnlyStakeOwnerCanUnstake -
		pub(crate) fn ensure_stake_owner(
			who: T::AccountId,
			fnft_collection_id: &T::AssetId,
			fnft_instance_id: &T::FinancialNftInstanceId,
		) -> Result<T::AccountId, DispatchError> {
			let owner = T::FinancialNft::owner(fnft_collection_id, fnft_instance_id)
				.ok_or(Error::<T>::FnftNotFound)?;

			ensure!(who == owner, Error::<T>::OnlyStakeOwnerCanInteractWithStake);

			Ok(who)
		}

		pub(crate) fn split_lock(
			asset_id: T::AssetId,
			existing_fnft_asset_account: &T::AccountId,
			new_fnft_asset_account: &T::AccountId,
			existing_account_amount: T::Balance,
			new_account_amount: T::Balance,
		) -> DispatchResult {
			T::Assets::set_lock(
				T::LockId::get(),
				asset_id,
				existing_fnft_asset_account,
				existing_account_amount,
			)?;

			// transfer the amount in the new position from the existing account to the new account
			// (this should be the total unlocked amount)
			T::Assets::transfer(
				asset_id,
				existing_fnft_asset_account,
				new_fnft_asset_account,
				new_account_amount,
				false, // not a user account, doesn't need to be kept alive
			)?;

			// lock assets on new account
			T::Assets::set_lock(
				T::LockId::get(),
				asset_id,
				new_fnft_asset_account,
				new_account_amount,
			)?;

			Ok(())
		}

		/// Transfers the rewards a staker has earned while updating the provided `rewards_pool`.
		///
		/// # Params
		/// * `pool_id` - Pool identifier
		/// * `mut rewards_pool` - Rewards pool to update
		/// * `stake` - Stake position
		/// * `early_unlock` - If there should be an early unlock penalty
		/// * `keep_alive` - If the transaction should be kept alive
		// TODO(benluelo): This function does too much - while claim and unstake have similar
		// functionality, I don't think this is the best abstraction of that. Refactor to have
		// smaller functions that can then be used in both claim and unstake.
		// NOTE: Low priority, this is currently working, just not optimal
		pub(crate) fn collect_rewards(
			pool_id: T::AssetId,
			rewards_pool: &mut RewardPoolOf<T>,
			fnft_instance_id: &T::FinancialNftInstanceId,
			stake: &mut StakeOf<T>,
			owner: &T::AccountId,
			penalize_for_early_unlock: bool,
		) -> Result<(), DispatchError> {
			for (reward_asset_id, reward) in &mut rewards_pool.rewards {
				let claim = claim_of_stake::<T>(
					stake,
					&rewards_pool.share_asset_id,
					reward,
					reward_asset_id,
				)?;

				let possibly_slashed_claim = if penalize_for_early_unlock {
					let amount_slashed = stake.lock.unlock_penalty.mul_floor(claim);

					Self::deposit_event(Event::<T>::UnstakeRewardSlashed {
						owner: owner.clone(),
						pool_id,
						fnft_instance_id: *fnft_instance_id,
						reward_asset_id: *reward_asset_id,
						amount_slashed,
					});

					T::Assets::transfer(
						*reward_asset_id,
						&Self::pool_account_id(&stake.reward_pool_id),
						&T::TreasuryAccount::get(),
						amount_slashed,
						false, // pallet account doesn't need to be kept alive
					)?;

					// SAFETY: amount_slashed is <= claim as is shown above
					claim.defensive_saturating_sub(amount_slashed)
				} else {
					claim
				};

				// REVIEW(benluelo): Review logic/ calculations regarding total_rewards & claimed
				// rewards
				let possibly_slashed_claim = sp_std::cmp::min(
					possibly_slashed_claim,
					reward.total_rewards.safe_sub(&reward.claimed_rewards)?,
				);

				// REVIEW(benluelo): Should the claimed_rewards include the slashed amount?
				reward.claimed_rewards =
					reward.claimed_rewards.safe_add(&possibly_slashed_claim)?;

				// REVIEW(benluelo): Expected behaviour if none?
				if let Some(inflation) = stake.reductions.get_mut(reward_asset_id) {
					*inflation += claim;
				}

				T::Assets::transfer(
					*reward_asset_id,
					&Self::pool_account_id(&stake.reward_pool_id),
					owner,
					possibly_slashed_claim,
					false, // pallet account doesn't need to be kept alive
				)?;
			}

			Ok(())
		}

		pub(crate) fn pool_account_id(pool_id: &T::AssetId) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(pool_id)
		}

		// TODO(benluelo): Rename to 'reward_multiplier_of' and return a Result<&_, Error<T>>
		// (remove the clone as well)
		// REVIEW(benluelo): Does this function provide anything meaningful?
		pub(crate) fn reward_multiplier(
			rewards_pool: &RewardPoolOf<T>,
			duration_preset: DurationSeconds,
		) -> Option<Validated<FixedU64, GeOne>> {
			rewards_pool.lock.duration_presets.get(&duration_preset).cloned()
		}

		pub(crate) fn boosted_amount(
			reward_multiplier: Validated<FixedU64, GeOne>,
			amount: T::Balance,
		) -> Result<T::Balance, ArithmeticError> {
			reward_multiplier.checked_mul_int(amount).ok_or(ArithmeticError::Overflow)
		}

		fn compute_rewards_and_reductions(
			shares: T::Balance,
			rewards_pool: &RewardPoolOf<T>,
		) -> Result<
			(
				BoundedBTreeMap<T::AssetId, Reward<T::Balance>, T::MaxRewardConfigsPerPool>,
				BoundedBTreeMap<T::AssetId, T::Balance, T::MaxRewardConfigsPerPool>,
			),
			DispatchError,
		> {
			let mut reductions = BoundedBTreeMap::new();
			let mut rewards_btree_map = BoundedBTreeMap::new();

			let total_shares: T::Balance =
				<T::Assets as FungiblesInspect<T::AccountId>>::total_issuance(
					rewards_pool.share_asset_id,
				);

			for (asset_id, reward) in rewards_pool.rewards.iter() {
				let inflation = if total_shares.is_zero() {
					T::Balance::zero()
				} else {
					reward.total_rewards.safe_mul(&shares)?.safe_div(&total_shares)?
				};

				let new_total_rewards = reward.total_rewards.safe_add(&inflation)?;
				let new_total_dilution_adjustment =
					reward.total_dilution_adjustment.safe_add(&inflation)?;

				rewards_btree_map
					.try_insert(
						*asset_id,
						Reward {
							total_rewards: new_total_rewards,
							total_dilution_adjustment: new_total_dilution_adjustment,
							..reward.clone()
						},
					)
					.map_err(|_| Error::<T>::ReductionConfigProblem)?;

				reductions
					.try_insert(*asset_id, inflation)
					.map_err(|_| Error::<T>::ReductionConfigProblem)?;
			}

			Ok((rewards_btree_map, reductions))
		}
	}

	impl<T: Config> ProtocolStaking for Pallet<T> {
		type AccountId = T::AccountId;
		type AssetId = T::AssetId;
		type Balance = T::Balance;
		type RewardPoolId = T::AssetId;

		#[transactional]
		fn transfer_reward(
			from: &Self::AccountId,
			pool_id: &Self::RewardPoolId,
			reward_currency: Self::AssetId,
			amount: Self::Balance,
			keep_alive: bool,
		) -> DispatchResult {
			RewardPools::<T>::try_mutate(pool_id, |maybe_reward_pool| {
				let reward_pool =
					maybe_reward_pool.as_mut().ok_or(Error::<T>::RewardsPoolNotFound)?;

				let pool_account_id = Self::pool_account_id(pool_id);

				let do_transfer = || {
					T::Assets::transfer(reward_currency, from, &pool_account_id, amount, keep_alive)
				};

				match reward_pool.rewards.get_mut(&reward_currency) {
					Some(_reward) => {
						do_transfer()?;
					},
					None => {
						let reward = Reward {
							total_rewards: amount,
							claimed_rewards: Zero::zero(),
							total_dilution_adjustment: T::Balance::zero(),
							reward_rate: RewardRate {
								amount: T::Balance::zero(),
								period: RewardRatePeriod::PerSecond,
							},
							last_updated_timestamp: T::UnixTime::now().as_secs(),
						};
						reward_pool
							.rewards
							.try_insert(reward_currency, reward)
							.map_err(|_| Error::<T>::TooManyRewardAssetTypes)?;
						do_transfer()?;
					},
				}

				Self::deposit_event(Event::RewardTransferred {
					from: from.clone(),
					pool_id: *pool_id,
					reward_currency,
					reward_increment: amount,
				});

				Ok(())
			})
		}
	}
}
/// Accumulates the rewards in a pool, if the pot isn't empty. Emits the relevant events
/// after accumulation. See [`accumulate_reward`] for more information about how the
/// accumulation calculation is done.
pub(crate) fn reward_accumulation_hook_reward_update_calculation<T: Config>(
	pool_id: T::AssetId,
	reward_asset_id: T::AssetId,
	reward: &mut Reward<T::Balance>,
	now_seconds: u64,
) {
	let pool_account = Pallet::<T>::pool_account_id(&pool_id);

	log::info!("calculating rewards for pool {pool_id:?}, asset {reward_asset_id:?}");

	// no need to calculate if the pot is empty, this will be unset if and when enough funds
	// are added to the pot
	if RewardsPotIsEmpty::<T>::contains_key(pool_id, reward_asset_id) {
		log::info!(
			"pot for pool {pool_id:?}, asset {reward_asset_id:?} is empty, not accumulating"
		);
		return
	}

	log::info!("accumulating rewards for pool {pool_id:?}, asset {reward_asset_id:?}");

	match accumulate_reward::<T>(reward_asset_id, reward, &pool_account, now_seconds) {
		RewardAccumulationCalculationOutcome::Success => {
			log::info!("accumulation successful");
		},
		RewardAccumulationCalculationOutcome::BackToTheFuture => {
			Pallet::<T>::deposit_event(Event::<T>::RewardAccumulationHookError {
				pool_id,
				asset_id: reward_asset_id,
				error: RewardAccumulationHookError::BackToTheFuture,
			});
		},
		RewardAccumulationCalculationOutcome::Overflow => {
			Pallet::<T>::deposit_event(Event::<T>::RewardAccumulationHookError {
				pool_id,
				asset_id: reward_asset_id,
				error: RewardAccumulationHookError::Overflow,
			});
		},
		RewardAccumulationCalculationOutcome::RewardsPotEmpty => {
			log::info!("accumulation successful, but pot is now empty");
			// The event only needs to be emitted once, since there's no need to notify that
			// the pool has paused if the pot was already emptied previously. Due to the
			// check above, we know that the pot had funds before the accumulation was done;
			// this `debug_assert!` exists to ensure that nothing else modifies the storage
			// between the aforementioned check and now.
			debug_assert!(!RewardsPotIsEmpty::<T>::contains_key(pool_id, reward_asset_id));

			RewardsPotIsEmpty::<T>::insert(pool_id, reward_asset_id, ());

			Pallet::<T>::deposit_event(Event::<T>::RewardPoolPaused {
				pool_id,
				asset_id: reward_asset_id,
			});
		},
	}
}

pub(crate) fn accumulate_rewards_hook<T: Config>() -> Weight {
	let now_seconds = T::UnixTime::now().as_secs();
	let unix_time_now_weight = T::WeightInfo::unix_time_now();

	let mut total_weight = unix_time_now_weight;

	let current_block = frame_system::Pallet::<T>::block_number();

	RewardPools::<T>::translate(|pool_id, mut reward_pool: RewardPoolOf<T>| {
		// NOTE: `StorageMap::translate` does one read and one write per item,
		// unconditionally
		total_weight += T::DbWeight::get().reads(1) + T::DbWeight::get().writes(1);

		total_weight +=
			accumulate_pool_rewards::<T>(pool_id, &mut reward_pool, current_block, now_seconds);

		Some(reward_pool)
	});

	total_weight
}

/// Accumulates all of the rewards in the provided pool, updating them in-place. Returns the weight
/// of the calculations.
#[must_use = "the calculated weight does nothing on it's own"]
fn accumulate_pool_rewards<T: Config>(
	pool_id: T::AssetId,
	reward_pool: &mut RewardPoolOf<T>,
	current_block: T::BlockNumber,
	now_seconds: u64,
) -> Weight {
	// If reward pool has not started, do not accumulate rewards or adjust weight
	Weight::from_ref_time(match reward_pool.start_block.cmp(&current_block) {
		// start block < current -> accumulate normally
		Ordering::Less =>
			(&mut reward_pool.rewards)
				.into_iter()
				.fold(0_u64, |mut acc, (asset_id, reward)| {
					reward_accumulation_hook_reward_update_calculation::<T>(
						pool_id,
						*asset_id,
						reward,
						now_seconds,
					);

					acc = acc.defensive_saturating_add(
						T::WeightInfo::reward_accumulation_hook_reward_update_calculation()
							.ref_time(),
					);

					acc
				}),
		// start block == current -> accumulation starts now, but the effects won't be seen until
		// the next block; set all of the reward's last updated timestamp to `now` so that reward
		// accumulation starts from this point in time, not when the pool was created. Also notify
		// that the pool has started.
		Ordering::Equal => {
			for (_asset_id, reward) in &mut reward_pool.rewards {
				reward.last_updated_timestamp = now_seconds;
			}

			Pallet::<T>::deposit_event(Event::RewardPoolStarted { pool_id });

			0
		},
		// start block > current -> don't accumulate, do nothing
		Ordering::Greater => 0,
	})
}

fn add_to_rewards_pot<T: Config>(
	who: T::AccountId,
	pool_id: T::AssetId,
	asset_id: T::AssetId,
	amount: T::Balance,
	keep_alive: bool,
) -> DispatchResult {
	RewardPools::<T>::try_mutate(pool_id, |pool| {
		let pool = pool.as_mut().ok_or(Error::<T>::RewardsPoolNotFound)?;

		let reward = pool.rewards.get_mut(&asset_id).ok_or(Error::<T>::RewardAssetNotFound)?;

		let pool_account = Pallet::<T>::pool_account_id(&pool_id);

		T::Assets::transfer(asset_id, &who, &pool_account, amount, keep_alive)?;
		T::Assets::hold(asset_id, &pool_account, amount)?;

		Pallet::<T>::deposit_event(Event::<T>::RewardsPotIncreased { pool_id, asset_id, amount });

		// if the pot was previously empty, *and* the amount added resulted in the pot having enough
		// balance to reward again, then resume accumulation and un-mark the pot as empty.
		//
		// NOTE: Pools that haven't started yet will never (i.e. should never) be in the
		// `RewardsPotIsEmpty` storage, so this check should fail for the aforementioned pools.
		//
		// TODO(benluelo): Maybe add some debug assertions for the above? This would be the most
		// useful if the chain that QA uses is compiled with debug assertions enabled.
		//
		// REVIEW(benluelo): This could be averted if we stored the timestamp in the
		// `RewardsPotIsEmpty` storage, making it into a "reward pool state" representatatio
		// instead.
		if RewardsPotIsEmpty::<T>::contains_key(pool_id, asset_id) &&
			T::Assets::balance_on_hold(asset_id, &pool_account) >=
				reward
					.reward_rate
					.amount_per_period()
					.defensive_unwrap_or_else(|| u128::MAX.into())
		{
			reward.last_updated_timestamp = T::UnixTime::now().as_secs();
			RewardsPotIsEmpty::<T>::remove(pool_id, asset_id);

			Pallet::<T>::deposit_event(Event::<T>::RewardPoolResumed { pool_id, asset_id });
		}

		Ok(())
	})
}

fn update_rewards_pool<T: Config>(
	pool_id: T::AssetId,
	reward_updates: BoundedBTreeMap<
		AssetIdOf<T>,
		RewardUpdate<BalanceOf<T>>,
		T::MaxRewardConfigsPerPool,
	>,
) -> DispatchResult {
	RewardPools::<T>::try_mutate(pool_id, |pool| {
		let pool = pool.as_mut().ok_or(Error::<T>::RewardsPoolNotFound)?;

		let now_seconds = T::UnixTime::now().as_secs();

		for (asset_id, update) in reward_updates {
			let reward = pool.rewards.get_mut(&asset_id).ok_or(Error::<T>::RewardAssetNotFound)?;

			reward_accumulation_hook_reward_update_calculation::<T>(
				pool_id,
				asset_id,
				reward,
				now_seconds,
			);

			reward.reward_rate = update.reward_rate;
		}

		Pallet::<T>::deposit_event(Event::<T>::RewardPoolUpdated { pool_id });

		Ok(())
	})
}

/// Calculates the update to the reward and unlocks the accumulated rewards from the pool account,
/// updating the provided [`Reward`] in-place.
///
/// NOTE: `<uX as Div<NonZeroUX>>::div` is used throughout this function as a safe,
/// non-panicking alternative to the regular `Div::div` implementation on the primitive numeric
/// types. The semantics are the same for both functions, see [here][nonzero-impls] for more
/// information.
///
/// [nonzero-impls]: https://doc.rust-lang.org/src/core/num/nonzero.rs.html#268-308
pub(crate) fn accumulate_reward<T: Config>(
	asset_id: T::AssetId,
	reward: &mut Reward<T::Balance>,
	pool_account: &T::AccountId,
	now_seconds: u64,
) -> RewardAccumulationCalculationOutcome {
	// TODO(benluelo): Refactor the calculations here into a separate function to make it easier to
	// test with proptest/ kani. The nonzero checks can be left outside of said function.

	// short-circuit if the reward rate amount is zero
	let Some(reward_rate_amount) = NonZeroU128::new(reward.reward_rate.amount.into())
		else {
			return RewardAccumulationCalculationOutcome::Success
		};

	// REVIEW(benluelo): Should this be a user-facing error? Or would defensively saturating at zero
	// for elapsed_time be better? This should never be hit, and if it is then it's either a logic
	// error or the chain state is wonky (in which case there are probably bigger issues than this
	// hook!)
	let Some(elapsed_time) = now_seconds
		.checked_sub(reward.last_updated_timestamp)
		else {
			return RewardAccumulationCalculationOutcome::BackToTheFuture
		};

	let reward_rate_period_seconds = reward.reward_rate.period.as_secs();

	//          elapsed_time
	// = --------------------------
	//   reward_rate_period_seconds
	let Some(periods_surpassed) =
		NonZeroU64::new(<u64 as Div<NonZeroU64>>::div(elapsed_time, reward_rate_period_seconds))
		else {
			// if no periods have been surpassed, short-circuit
			return RewardAccumulationCalculationOutcome::Success
		};

	let total_locked_rewards: u128 = T::Assets::balance_on_hold(asset_id, pool_account).into();
	log::info!("total_locked_rewards = {total_locked_rewards}");

	// the maximum amount repayable given the reward rate.
	// i.e. if total locked is 50, and the reward rate is 15, then this would be 3
	//
	//   total_locked_rewards
	// = --------------------
	//    reward_rate_amount
	let Some(maximum_releasable_periods) =
		NonZeroU128::new(<u128 as Div<NonZeroU128>>::div(total_locked_rewards, reward_rate_amount))
		else {
			// if the maximum releasable periods is zero, then that means the pot doesn't have
			// enough in it to fund a single period.
			return RewardAccumulationCalculationOutcome::RewardsPotEmpty;
		};

	//     ( total_locked_rewards          elapsed_time        )
	// min ( -------------------- , -------------------------- )
	//     (  reward_rate_amount    reward_rate_period_seconds )
	let releasable_periods_surpassed =
		cmp::min(maximum_releasable_periods, periods_surpassed.into());

	// SAFETY: Usage of mul is safe here. See the following proof:
	//
	// ```plaintext
	//   {                      total_locked_rewards
	//   { reward_rate_amount * --------------------,       if maximum_releasable_periods <= periods_surpassed
	//   {                       reward_rate_amount
	// = {
	//   {                             elapsed_time
	//   { reward_rate_amount * --------------------------, if periods_surpassed < maximum_releasable_periods
	//   {                      reward_rate_period_seconds
	// ```
	//
	// Note that the second function of the piecewise definition above will always be <= the first,
	// since if it were larger it would not have been selected; and since the first function is
	// always infallible (the resulting value will always be <= the total_locked_rewards input due
	// to integer division), the second part is also infallible if it is less than the first.
	let newly_accumulated_rewards = releasable_periods_surpassed
		.checked_mul(reward_rate_amount)
		.expect("should not fail; see above for proof; qed;");

	let Some(new_total_rewards) = newly_accumulated_rewards
		.get()
		.checked_add(reward.total_rewards.into())
		else {
			return RewardAccumulationCalculationOutcome::Overflow;
		};

	log::info!("asset_id: {asset_id:?}; new_total_rewards = {new_total_rewards}");

	// `u64::MAX` in seconds is roughly 584.9 billion years in the future, so saturating at that
	// should be ok; we should never reach a case where the timestamp overflows that. Use defensive
	// anyways so we get notified if this is hit somehow due to some sort of logic error.
	let last_updated_timestamp = reward.last_updated_timestamp.defensive_saturating_add(
		reward_rate_period_seconds.get().defensive_saturating_mul(
			releasable_periods_surpassed.get().try_into().defensive_unwrap_or(u64::MAX),
		),
	);

	T::Assets::release(
		asset_id,
		pool_account,
		newly_accumulated_rewards.get().into(),
		false, // not best effort, entire amount must be released
	)
	.expect("funds should be available to release; see above for proof; qed;");

	reward.total_rewards = new_total_rewards.into();
	reward.last_updated_timestamp = last_updated_timestamp;

	RewardAccumulationCalculationOutcome::Success
}

#[derive(Debug)]
pub(crate) enum RewardAccumulationCalculationOutcome {
	/// The calculation succeeded with no errors.
	Success,
	/// T::UnixTime::now() returned a value in the past.
	BackToTheFuture,
	/// The rewards pot (held balance) for this pool is empty or doesn't have enough held balance
	/// to release for one period.
	RewardsPotEmpty,
	/// Accumulating rewards for an account overflowed.
	Overflow,
}

pub(crate) fn claim_of_stake<T: Config>(
	stake: &StakeOf<T>,
	share_asset_id: &<T as Config>::AssetId,
	reward: &Reward<T::Balance>,
	reward_asset_id: &<T as Config>::AssetId,
) -> Result<T::Balance, DispatchError> {
	let total_shares: T::Balance =
		<T::Assets as FungiblesInspect<T::AccountId>>::total_issuance(*share_asset_id);

	let claim = if total_shares.is_zero() {
		T::Balance::zero()
	} else {
		let inflation = stake.reductions.get(reward_asset_id).cloned().unwrap_or_else(Zero::zero);

		// REVIEW(benluelo): Review expected rounding behaviour, possibly switching to the following
		// implementation (or something similar):
		// Perbill::from_rational(stake.share, total_issuance)
		// 	.mul_floor(reward.total_rewards)
		// 	.safe_sub(&inflation)?;

		reward
			.total_rewards
			.safe_mul(&stake.share)?
			.safe_div(&total_shares)?
			.safe_sub(&inflation)?
	};

	Ok(claim)
}
