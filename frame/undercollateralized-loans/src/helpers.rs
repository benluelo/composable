use crate::{
	types::{
		LoanConfigOf, LoanInputOf, MarketConfigOf, MarketInfoOf, MarketInputOf, RepaymentResult,
		Timestamp,
	},
	validation::{AssetIsSupportedByOracle, CurrencyPairIsNotSame, LoanInputIsValid},
	Config, DebtTokenForMarketStorage, Error, MarketsStorage, Pallet,
};
use composable_support::validation::Validated;
use composable_traits::{
	currency::CurrencyFactory,
	defi::DeFiComposableConfig,
	oracle::Oracle,
	undercollateralized_loans::{LoanConfig, MarketConfig, MarketInfo, UndercollateralizedLoans},
	vault::{Deposit, FundsAvailability, StrategicVault, Vault, VaultConfig},
};
use frame_support::{
	ensure,
	storage::with_transaction,
	traits::{
		fungible::{Inspect as NativeInspect, Transfer as NativeTransfer},
		fungibles::{Inspect, Mutate, Transfer},
		Get, UnixTime,
	},
};
use sp_runtime::{
	traits::{One, Saturating, Zero},
	DispatchError, Perquintill, TransactionOutcome,
};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use sp_std::vec::Vec;

// #generalization
impl<T: Config> Pallet<T> {
	pub(crate) fn do_create_market(
		manager: T::AccountId,
		input: Validated<
			MarketInputOf<T>,
			(CurrencyPairIsNotSame, AssetIsSupportedByOracle<T::Oracle>),
		>,
		keep_alive: bool,
	) -> Result<MarketInfoOf<T>, DispatchError> {
		let config_input = input.value();
		crate::MarketsCounterStorage::<T>::try_mutate(|counter| {
			*counter += T::Counter::one();
			ensure!(
				*counter <= T::MaxMarketsCounterValue::get(),
				Error::<T>::ExceedMaxMarketsCounterValue
			);
			let market_account_id = Self::market_account_id(*counter);
			let borrow_asset_vault = T::Vault::create(
				Deposit::Existential,
				VaultConfig {
					asset_id: config_input.borrow_asset(),
					reserved: config_input.reserved_factor(),
					manager: manager.clone(),
					strategies: [(
						market_account_id.clone(),
						Perquintill::one().saturating_sub(config_input.reserved_factor()),
					)]
					.into_iter()
					.collect(),
				},
			)?;

			let initial_pool_size = Self::calculate_initial_pool_size(config_input.borrow_asset())?;

			ensure!(
				initial_pool_size > T::Balance::zero(),
				Error::<T>::PriceOfInitialBorrowVaultShouldBeGreaterThanZero
			);

			T::MultiCurrency::transfer(
				config_input.borrow_asset(),
				&manager,
				&market_account_id,
				initial_pool_size,
				keep_alive,
			)?;

			let market_config = MarketConfig::new(
				market_account_id.clone(),
				manager,
				borrow_asset_vault,
				config_input.borrow_asset(),
				config_input.collateral_asset(),
				config_input.max_price_age,
				config_input.whitelist,
			);
			let market_info = MarketInfo::new(market_config, config_input.liquidation_strategies);
			let debt_token_id = T::CurrencyFactory::reserve_lp_token_id(T::Balance::default())?;

			DebtTokenForMarketStorage::<T>::insert(market_account_id.clone(), debt_token_id);
			MarketsStorage::<T>::insert(market_account_id, market_info.clone());
			Ok(market_info)
		})
	}
	// Create non-active loan, which should be activated via borrower.
	// TODO: @mikolaichuk: check why LoanInputOf does not work here
	pub(crate) fn do_create_loan(
		input: Validated<LoanInputOf<T>, LoanInputIsValid<crate::Pallet<T>>>,
	) -> Result<LoanConfigOf<T>, DispatchError> {
		let config_input = input.value();
		// Get market config. Unwrapped since we have checked market existence during input
		// validation process.
		let market_config =
			Self::get_market_config_via_account_id(&config_input.market_account_id)?;
		// Align schedule timstamps to the beginign of the day.
		// 24.08.1991 08:45:03 -> 24.08.1991 00:00:00
		let schedule = config_input
			.payment_schedule
			.into_iter()
			.map(|(timestamp, balance)| (Self::get_date_aligned_timestamp(timestamp), balance))
			.collect();
		// Create non-activated loan and increment loans' counter.
		// This loan have to be activated by borrower further.
		crate::LoansCounterStorage::<T>::try_mutate(|counter| {
			*counter += T::Counter::one();
			let loan_account_id = Self::loan_account_id(*counter);
			let loan_config = LoanConfig::new(
				loan_account_id.clone(),
				config_input.market_account_id,
				config_input.borrower_account_id,
				market_config.collateral_asset_id().clone(),
				market_config.borrow_asset_id().clone(),
				config_input.principal,
				config_input.collateral,
				schedule,
			);
			crate::LoansStorage::<T>::insert(loan_account_id.clone(), loan_config.clone());
			crate::NonActiveLoansStorage::<T>::insert(loan_account_id, ());
			Ok(loan_config)
		})
	}

	/// Borrow assets as per loan_account loan terms.
	/// Activates the loan.
	/// Supposed to be called from transactional dispatchable function.
	pub(crate) fn do_borrow(
		borrower_account_id: T::AccountId,
		loan_account_id: T::AccountId,
		keep_alive: bool,
	) -> Result<LoanConfigOf<T>, DispatchError> {
		// Check if loan's account id is in non-activated loans list.
		// If it is not, loan does not exist or was already activated.
		ensure!(
			crate::NonActiveLoansStorage::<T>::contains_key(loan_account_id.clone()),
			Error::<T>::LoanDoesNotExistOrWasActivated
		);
		let loan_config = Self::get_loan_config_via_account_id(&loan_account_id)?;
		// Check if borrower is authorized to execute this loan agreement.
		ensure!(
			*loan_config.borrower_account_id() == borrower_account_id,
			Error::<T>::ThisUserIsNotAllowedToExecuteThisContract
		);
		// Check if borrower tries to activate expired loan.
		// Need this check since we remove expired loans only once a day.
		let today = Self::current_date();
		let first_payment_date = Self::date_from_timestamp(*loan_config.first_payment_moment());
		// Loan should be activated before the first payment date.
		if today >= first_payment_date {
			crate::NonActiveLoansStorage::<T>::remove(loan_account_id.clone());
			crate::LoansStorage::<T>::remove(loan_account_id.clone());
			Err(Error::<T>::TheLoanContractIsExpired)?;
		}
		// Transfer minimum amount of native asset from the borrower's account to the loan's account
		// to ensure loan account existence.
		T::NativeCurrency::transfer(
			&borrower_account_id,
			&loan_account_id,
			T::NativeCurrency::minimum_balance(),
			keep_alive,
		)?;
		// Transfer collateral from the borrower's account to the loan's account.
		let collateral_asset_id = *loan_config.collateral_asset_id();
		let source = &borrower_account_id;
		let destination = &loan_account_id;
		let amount = *loan_config.collateral();
		T::MultiCurrency::transfer(collateral_asset_id, source, destination, amount, keep_alive)?;
		// Transfer borrowed assets from market's account to the borrower's account.
		let borrow_asset_id = *loan_config.borrow_asset_id();
		let source = loan_config.market_account_id();
		let destination = &borrower_account_id;
		let amount = *loan_config.principal();
		T::MultiCurrency::transfer(borrow_asset_id, source, destination, amount, keep_alive)?;
		// Mint 'principal' amount of debt tokens to the loan's account.
		// We use these tokens to indicate which part of principal is not refunded yet.
		let debt_token_id =
			crate::DebtTokenForMarketStorage::<T>::get(loan_config.market_account_id())
				.ok_or(Error::<T>::MarketDoesNotExist)?;
		<T as Config>::MultiCurrency::mint_into(debt_token_id, &loan_account_id, amount)?;
		// Loan is active now.
		// Remove loan configuration from the non-activated loans accounts ids storage.
		crate::NonActiveLoansStorage::<T>::remove(loan_account_id.clone());
		// Build payment schedule.
		for timestamp in loan_config.schedule().keys() {
			crate::ScheduleStorage::<T>::mutate(timestamp, |loans_accounts_ids| {
				loans_accounts_ids.insert(loan_account_id.clone())
			});
		}
		Ok(loan_config)
	}

	// Repay any amount of money
	pub(crate) fn do_repay(
		payer_account_id: T::AccountId,
		loan_account_id: T::AccountId,
		repay_amount: T::Balance,
		keep_alive: bool,
	) -> Result<T::Balance, DispatchError> {
		// Get loan's info.
		let loan_config = Self::get_loan_config_via_account_id(&loan_account_id)?;
		let borrow_asset_id = loan_config.borrow_asset_id();
		// Transfer 'amount' of assets from the payer account to the loan account
		T::MultiCurrency::transfer(
			*borrow_asset_id,
			&payer_account_id,
			&loan_account_id,
			repay_amount,
			keep_alive,
		)
	}

	pub(crate) fn do_liquidate(loan_config: &LoanConfigOf<T>) -> () {
		Self::deposit_event(crate::Event::<T>::TheLoanWasSentToLiquidation {
			loan_config: loan_config.clone(),
		});
	}

	// Close loan contract since loan is paid.
	pub(crate) fn close_loan_contract(loan_config: &LoanConfigOf<T>, keep_alive: bool) {
		// Transfer collateral to borrower's account.
		T::MultiCurrency::transfer(
			*loan_config.collateral_asset_id(),
			loan_config.market_account_id(),
			loan_config.borrower_account_id(),
			*loan_config.collateral(),
			keep_alive,
		)
	    // May happens if borrower's account died for some reason.
        // TODO: @mikolaichuk: what we are going to with such situations? 
        // Can we transfer collateral at market account in this case?
        // Perhaps we have to allow manager to transfer collateral to any account in such cases? 
        .map_or_else(|error| log::error!("Collateral was not transferred back to the borrower's account due to the following error: {:?}", error), |_| ());
		// Transfer borrowed assets to the borrower's account.
		// If borrower has transferred extra money by mistake we have to return them.
		let loan_account_borrow_asset_balance =
			T::MultiCurrency::balance(*loan_config.borrow_asset_id(), loan_config.account_id());
		T::MultiCurrency::transfer(
			*loan_config.borrow_asset_id(),
			loan_config.market_account_id(),
			loan_config.borrower_account_id(),
            loan_account_borrow_asset_balance,
			keep_alive,
		)
	    // May happens if borrower's account died for some reason.
        .map_or_else(|error| log::error!("Remainded borrow asset was not transferred back to the borrower's account due to the following error: {:?}", error), |_| ());
		// Remove all information about the loan.
		Self::terminate_activated_loan(loan_config);
		Self::deposit_event(crate::Event::<T>::LoanWasClosed { loan_config: loan_config.clone() });
	}

	// Check if borrower's account is whitelisted for particular market.
	pub(crate) fn is_borrower_account_whitelisted(
		borrower_account_id_ref: &T::AccountId,
		market_account_id_ref: &T::AccountId,
	) -> Result<bool, DispatchError> {
		let market_config = Self::get_market_config_via_account_id(market_account_id_ref)?;
		Ok(market_config.whitelist().contains(borrower_account_id_ref))
	}

	pub(crate) fn calculate_initial_pool_size(
		borrow_asset: <T::Oracle as composable_traits::oracle::Oracle>::AssetId,
	) -> Result<<T as DeFiComposableConfig>::Balance, DispatchError> {
		T::Oracle::get_price_inverse(borrow_asset, T::OracleMarketCreationStake::get())
	}

	// Check if provided account id belongs to the market manager.
	pub(crate) fn is_market_manager_account(
		account_id: &T::AccountId,
		market_account_id_ref: &T::AccountId,
	) -> Result<bool, DispatchError> {
		let market_config = Self::get_market_config_via_account_id(market_account_id_ref)?;
		Ok(market_config.manager() == account_id)
	}

	// TODO: @mikolaichuk: Add weights calculation
	// #generalization
	pub(crate) fn treat_vaults_balance(block_number: T::BlockNumber) -> () {
		let _ = with_transaction(|| {
			let now = Self::now();
			let mut errors = crate::MarketsStorage::<T>::iter()
				.map(|(market_account_id, market_info)| {
					match Self::available_funds(&market_info.config(), &market_account_id)? {
						FundsAvailability::Withdrawable(balance) => {
							Self::handle_withdrawable(
								&market_info.config(),
								&market_account_id,
								balance,
							)?;
						},
						FundsAvailability::Depositable(balance) => {
							Self::handle_depositable(
								&market_info.config(),
								&market_account_id,
								balance,
							)?;
						},
						FundsAvailability::MustLiquidate => {
							Self::handle_must_liquidate(&market_info.config(), market_account_id)?;
						},
					}
					Result::<(), DispatchError>::Ok(())
				})
				.filter_map(|r| match r {
					Ok(_) => None,
					Err(err) => Some(err),
				})
				.peekable();

			if errors.peek().is_none() {
				TransactionOutcome::Commit(Ok(1000))
			} else {
				errors.for_each(|e| {
					log::error!(
						"This should never happen, could not initialize block!!! {:#?} {:#?}",
						block_number,
						e
					)
				});
				TransactionOutcome::Rollback(Err(DispatchError::Other(
					"failed to initialize block",
				)))
			}
		});
	}

	pub(crate) fn check_payments(today: Timestamp) -> () {
		// Retrive collection of loans(loans' accounts ids) which have to be paid now.
		// If nothing found we get empty set.
		let loans_accounts_ids = crate::ScheduleStorage::<T>::try_get(today).unwrap_or_default();

		for loan_account_id in loans_accounts_ids {
			// Treat situation when non-existent loan's id is in global schedule.
			let loan_config = match Self::get_loan_config_via_account_id(&loan_account_id) {
				Ok(loan_config) => loan_config,
				Err(_) => continue,
			};
			match Self::check_payment(&loan_config, today) {
				RepaymentResult::Failed(error) => log::error!(
					"Payment check for loan {:?} were failed becasue of the following error: {:?}",
					loan_account_id,
					error
				),
				RepaymentResult::PaymentMadeOnTime => (),
				RepaymentResult::LastPaymentMadeOnTime =>
					Self::close_loan_contract(&loan_config, false),
				RepaymentResult::PaymentIsOverdue => Self::do_liquidate(&loan_config),
			}
			// We do not need information regarding this date anymore.
			crate::ScheduleStorage::<T>::remove(today);
		}
	}

	// Check if a payment is succeed.
	fn check_payment(loan_config: &LoanConfigOf<T>, today: i64) -> RepaymentResult {
		let payment_amount = match loan_config.get_payment_for_particular_moment(&today) {
			Some(payment_amount) => payment_amount,
			None =>
				return RepaymentResult::Failed(
					crate::Error::<T>::ThereIsNoSuchMomentInTheLoanPaymentSchedule.into(),
				),
		};
		match T::MultiCurrency::transfer(
			*loan_config.borrow_asset_id(),
			loan_config.account_id(),
			loan_config.market_account_id(),
			*payment_amount,
			true,
		) {
			Ok(_) if *loan_config.last_payment_moment() == today =>
				return RepaymentResult::LastPaymentMadeOnTime,
			Ok(_) => return RepaymentResult::PaymentMadeOnTime,
			Err(_) => return RepaymentResult::PaymentIsOverdue,
		}
	}

	// Check if vault balanced or we have to deposit money to the vault or withdraw money from it.
	// If vault is balanced we will do nothing.
	// #generalization
	fn available_funds(
		config: &MarketConfigOf<T>,
		market_account: &T::AccountId,
	) -> Result<FundsAvailability<T::Balance>, DispatchError> {
		<T::Vault as StrategicVault>::available_funds(&config.borrow_asset_vault(), market_account)
	}

	// If we can withdraw from the vault
	// #generalization
	fn handle_withdrawable(
		config: &MarketConfigOf<T>,
		market_account: &T::AccountId,
		balance: T::Balance,
	) -> Result<(), DispatchError> {
		<T::Vault as StrategicVault>::withdraw(
			&config.borrow_asset_vault(),
			market_account,
			balance,
		)
	}

	// If vault is unblanced and we have to deposit some assets to the vault.
	// #generalization
	fn handle_depositable(
		config: &MarketConfigOf<T>,
		market_account: &T::AccountId,
		balance: T::Balance,
	) -> Result<(), DispatchError> {
		let asset_id = <T::Vault as Vault>::asset_id(&config.borrow_asset_vault())?;
		let balance =
			<T as Config>::MultiCurrency::reducible_balance(asset_id, market_account, false)
				.min(balance);
		<T::Vault as StrategicVault>::deposit(&config.borrow_asset_vault(), market_account, balance)
	}

	// TODO: @mikolaichuk: Implement logic when vault is stopped, tombstoned or does not exist.
	// #generalization
	fn handle_must_liquidate(
		_market_config: &MarketConfigOf<T>,
		_market_account_id: T::AccountId,
	) -> Result<(), DispatchError> {
		todo!()
	}

	// #generalization
	pub(crate) fn now() -> Timestamp {
		T::UnixTime::now().as_secs() as Timestamp
	}

	// #generalization
	pub(crate) fn get_market_info_via_account_id(
		market_account_id_ref: &T::AccountId,
	) -> Result<MarketInfoOf<T>, crate::Error<T>> {
		crate::MarketsStorage::<T>::try_get(market_account_id_ref)
			.map_err(|_| crate::Error::<T>::MarketDoesNotExist)
	}

	pub(crate) fn get_market_config_via_account_id(
		market_account_id_ref: &T::AccountId,
	) -> Result<MarketConfigOf<T>, crate::Error<T>> {
		let market_info = Self::get_market_info_via_account_id(market_account_id_ref)?;
		Ok(market_info.config().clone())
	}

	pub(crate) fn get_loan_config_via_account_id(
		loan_account_id_ref: &T::AccountId,
	) -> Result<LoanConfigOf<T>, crate::Error<T>> {
		crate::LoansStorage::<T>::try_get(loan_account_id_ref)
			.map_err(|_| crate::Error::<T>::ThereIsNoSuchLoan)
	}

	// Removes expired non-activated loans.
	// Expired non-activated loans are loans which were not activated by borrower before first
	// payment date.
	pub(crate) fn terminate_non_activated_expired_loans(today: Timestamp) {
		let mut removed_non_active_accounts_ids = vec![];
		for non_active_loan_account_id in crate::NonActiveLoansStorage::<T>::iter_keys() {
			// If, for some reason, loan is not presented in the loans storage
			// we just remove it from from the non-active loans set.
			if let Ok(loan_config) =
				Self::get_loan_config_via_account_id(&non_active_loan_account_id)
			{
				// If current date is larger then the loan first payment date, then this loan is
				// expired.
				if today >= *loan_config.first_payment_moment() {
					crate::LoansStorage::<T>::remove(non_active_loan_account_id.clone());
				}
				removed_non_active_accounts_ids.push(non_active_loan_account_id);
			}
		}
		removed_non_active_accounts_ids
			.iter()
			.for_each(|account_id| crate::NonActiveLoansStorage::<T>::remove(account_id));
		Self::deposit_event(crate::Event::<T>::NonActivatedExpiredLoansWereTerminated {
			loans_ids: removed_non_active_accounts_ids.clone(),
		});
	}

	// Remove all information about activated loan.
	pub(crate) fn terminate_activated_loan(loan_config: &LoanConfigOf<T>) {
		// Get payment moments for the loan.
		let payment_moments: Vec<Timestamp> =
			loan_config.schedule().keys().map(|&payment_moment| payment_moment).collect();
		// Remove account id from the global payment schedule for each payment date.
		payment_moments.iter().for_each(|payment_moment| {
			crate::ScheduleStorage::<T>::mutate(payment_moment, |loans_accounts_ids| {
				loans_accounts_ids.remove(loan_config.account_id())
			});
		});
		// TODO: @mikolaichuk: What we suppose to do if borrower's account is died?
		T::NativeCurrency::transfer(
			loan_config.account_id(),
			loan_config.borrower_account_id(),
			T::NativeCurrency::minimum_balance(),
			false,
		)
		.map_or_else(|error| log::error!("Fee was not transferred back to the borrowers account due to the following error: {:?}", error), |_| ());
		Self::deposit_event(crate::Event::<T>::LoanWasTerminated {
			loan_config: loan_config.clone(),
		})
	}

	pub(crate) fn current_day_timestamp() -> Timestamp {
		crate::CurrentDateStorage::<T>::get()
	}

	pub(crate) fn current_date() -> NaiveDate {
		Self::date_from_timestamp(Self::current_day_timestamp())
	}

	pub(crate) fn date_from_timestamp(timestamp: Timestamp) -> NaiveDate {
		NaiveDateTime::from_timestamp(timestamp, 0).date()
	}

	pub(crate) fn get_date_aligned_timestamp(timestamp: Timestamp) -> Timestamp {
		Self::date_from_timestamp(timestamp).and_time(NaiveTime::default()).timestamp()
	}
}
