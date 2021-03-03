//! # Art Gallery
//! The module provides implementations for art gallery with
//! non-fungible-tokens.
//!
//! - [`Config`](./trait.Config.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! This module tightly coupled with NFT module provides basic functions to
//! manage Art Gallery.
//!
//! ### Module Functions
//!
//! - `mint` - Mint NFT(non fungible token)
//! - `burn` - Burn NFT(non fungible token)
//! - `transfer` - Change owner for NFT(non fungible token) with tree hierarchy
//! limitation
//! - `assign` - Add NFT(non fungible token) to gallery hierarchy
//! - `unassign` - Remove NFT(non fungible token) from gallery hierarchy
//! - `mint_and_assign` - Mint NFT(non fungible token) and add to gallery
//! hierarchy

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{ 
	decl_error, decl_event, decl_module, 
	decl_storage, ensure,
};
use frame_support::traits::{
	Currency, LockableCurrency, LockIdentifier, WithdrawReasons,
	Get, ExistenceRequirement,
};
use frame_system::{ ensure_signed, ensure_root };
use orml_nft::{self as nft};
// use pallet_atomic_swap::{self as atomic_swap};
use sp_runtime::{ 	
	traits::{Zero},
	DispatchResult, };
use sp_std::prelude::*;

const PALLET_ID: LockIdentifier = *b"gallery ";

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
pub enum ReportReason {
	None,
	Illigal,
	Plagiarism,
	Duplicate,
	Reported
}

#[derive(Encode, Decode, Clone, Debug, Eq, PartialEq)]
#[derive(Serialize)]
pub struct TokenData {
    pub ipfs_pin: Vec<u8>,
}

#[derive(Encode, Decode, Clone, Debug, Eq, PartialEq)]
#[derive(Serialize)]
pub struct ClassData {}

#[derive(Encode, Decode, Clone, Debug, PartialEq)]
pub struct ExtendedInfo {
    pub display_flag: bool,
    pub report: ReportReason,
}

decl_error! {
	/// Error for art gallery
	pub enum Error for Module<T: Config> {
		/// Collection does't exists
		CollectionNotFound,
		/// Token doesn't exists
		TokenNotFound,
		/// Sender should equal token owner
		MustBeTokenOwner,
		/// Sender should be collection owner
		MustBeCollectionOwner,
		/// Sender should be collection owner or curator
		MustBeCollectionOwnerOrCurator,
		/// Sender should be curator
		MustBeCurator,
		/// Specified amount is above sender balance
		BalanceNotEnough,
	}
}

pub trait Config: frame_system::Config + nft::Config  { //+ atomic_swap::Config
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// The currency trait.
	type Currency: LockableCurrency<Self::AccountId>;

	/// Token default cost.
	type DefaultCost: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        ClassId = <T as nft::Config>::ClassId,
		TokenId = <T as nft::Config>::TokenId,
		Balance = BalanceOf<T>
    {
        /// New collection was created
        /// 
        /// # Arguments
        /// 
		/// ClassId: Globally unique identifier of newly created collection.
        CollectionCreated(ClassId),

        /// New item was created.
        /// 
        /// # Arguments
        /// 
		/// ClassId: Id of the collection where item was created.
		/// 
		/// TokenId: Id of an item. Unique within the collection.
        NFTCreated(ClassId, TokenId),

        /// Collection item was burned.
        /// 
        /// # Arguments
        /// 
        /// ClassId: Collection Id
		/// 
		/// TokenId: Identifier of NFT.
		NFTBurned(ClassId, TokenId),
		
		/// Transfer has been ended.
        /// 
        /// # Arguments
        /// 
        /// ClassId: Collection Id
		/// 
		/// TokenId: Identifier of NFT.
        /// 
		/// AccountId: Recipient.
		Transfer(ClassId, TokenId, AccountId),
		
		/// Offer has been created.
        /// 
        /// # Arguments
        /// 
        /// ClassId: Collection Id
		/// 
		/// TokenId: Identifier of NFT.
        /// 
		/// Balance: Price of NFT.
		/// 
		/// AccountId: Buyer Address
		OfferCreated(ClassId, TokenId, Balance, AccountId),
		
		/// Offer has been accepted.
        /// 
        /// # Arguments
        /// 
        /// ClassId: Collection Id
		/// 
		/// TokenId: Identifier of NFT.
        /// 
		/// AccountId: Seller Address
		/// 
		/// AccountId: Buyer Address
		AcceptOffer(ClassId, TokenId, AccountId, AccountId),
		
		/// Offer canceled.
        /// 
        /// # Arguments
        /// 
		/// ClassId: Collection Id
		/// 
		/// TokenId: Identifier of NFT.
        /// 
		/// AccountId: Seller Address
		/// 
		/// AccountId: Buyer Address
		CancelOffer(ClassId, TokenId, AccountId, AccountId),
		
		/// Appreciation has been sent.
        /// 
        /// # Arguments
        /// 
		/// ClassId: Collection Id
		/// 
		/// TokenId: Identifier of NFT.
        /// 
		/// Balance: Amount of appreciation.
		/// 
		AppreciationReceived(ClassId, TokenId, Balance),
		
		/// Display flag has been toggled.
        /// 
        /// # Arguments
        /// 
		/// bool: Display flag
		ToggleDisplay(bool),
		
		/// Report state has been set.
        /// 
        /// # Arguments
        /// 
		/// ClassId: Collection Id
		/// 
		/// TokenId: Identifier of NFT.
		/// 
		/// ReportReason: Reason of report
		ArtReported(ClassId, TokenId, ReportReason),
		
		/// Report has been accepted.
        /// 
        /// # Arguments
        /// 
		/// ClassId: Collection Id
		/// 
		/// TokenId: Identifier of NFT.
		ArtReportAccepted(ClassId, TokenId),
		
		/// Report has been cleared.
        /// 
        /// # Arguments
        /// 
		/// ClassId: Collection Id
		/// 
		/// TokenId: Identifier of NFT.
        ArtReportCleared(ClassId, TokenId),
    }
);

decl_storage! {
	trait Store for Module<T: Config> as ArtGallery {
		/// Curator address
		pub Curator: T::AccountId;

		/// Returns `None` if info not set or removed.
		pub TokenExtendedInfo get(fn token_extended_info): double_map hasher(twox_64_concat) T::ClassId, hasher(twox_64_concat) T::TokenId => Option<ExtendedInfo>;

	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = 0]
		pub fn create_collection(origin, 
				metadata: Vec<u8>, 
				data: <T as orml_nft::Config>::ClassData) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let collection_id = nft::Module::<T>::create_class(&_who, metadata, data)?;

			Self::deposit_event(RawEvent::CollectionCreated(collection_id));

			Ok(())
		}

		#[weight = 0]
		pub fn mint(origin,
				collection_id: T::ClassId,
				metadata: Vec<u8>,
				data: <T as orml_nft::Config>::TokenData) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// collection exists check
			let collection = nft::Module::<T>::classes(collection_id).ok_or(Error::<T>::CollectionNotFound)?;

			ensure!(collection.owner == _who, Error::<T>::MustBeCollectionOwner);

			let balance = T::Currency::free_balance(&_who);
			ensure!(!balance.is_zero(), Error::<T>::BalanceNotEnough);

		    //	let locked = balance.saturating_sub(T::DefaultCost::get());	

			T::Currency::set_lock(PALLET_ID, &_who, T::DefaultCost::get(), WithdrawReasons::all());
			let token_id = nft::Module::<T>::mint(&_who, collection_id, metadata, data)?;

			Self::deposit_event(RawEvent::NFTCreated(collection_id, token_id));

			Ok(())
		}

		#[weight = 0]
		pub fn burn(origin,
				collection_id: T::ClassId,
				token_id: T::TokenId) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// collection exists check
			let collection = nft::Module::<T>::classes(collection_id).ok_or(Error::<T>::CollectionNotFound)?;

			ensure!(Curator::<T>::get() == _who || collection.owner == _who, 
				Error::<T>::MustBeCollectionOwnerOrCurator);

			T::Currency::remove_lock(PALLET_ID, &_who);
			nft::Module::<T>::burn(&_who, (collection_id, token_id))?;	

			Self::deposit_event(RawEvent::NFTBurned(collection_id, token_id));

			Ok(())
		}

		#[weight = 0]
		pub fn transfer(origin,
				collection_id: T::ClassId,
				token_id: T::TokenId,
				recipient: T::AccountId) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// token exists check
			let token = nft::Module::<T>::tokens(collection_id, token_id).ok_or(Error::<T>::TokenNotFound)?;
			ensure!(token.owner == _who, Error::<T>::MustBeTokenOwner);

			nft::Module::<T>::transfer(&_who, &recipient, (collection_id, token_id))?;	
			Self::deposit_event(RawEvent::Transfer(collection_id, token_id, recipient));

			Ok(())
		}

		#[weight = 0]
		pub fn create_offer(origin,
			collection_id: T::ClassId,
			token_id: T::TokenId,
			price: BalanceOf<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Event
			// 	OfferCreated
			// 		Collection ID
			// 		Token ID
			// 		Price
			// 		Buyer Address

			Ok(())	
		}

		#[weight = 0]
		pub fn accept_offer(origin,
			collection_id: T::ClassId,
			token_id: T::TokenId,
			buyer_address: T::AccountId) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Event
			// 	OfferAccepted
			// 		Collection ID
			// 		Token ID
			// 		Seller Address
			// 		Buyer Address

			Ok(())	
		}

		#[weight = 0]
		pub fn cancel_offer(origin,
			collection_id: T::ClassId,
			token_id: T::TokenId) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Event
			// 	OfferCanceled
			// 		Collection ID
			// 		Token ID
			// 		Seller Address
			// 		Buyer Address

			Ok(())	
		}

		#[weight = 0]
		pub fn appreciate(origin,
			collection_id: T::ClassId,
			token_id: T::TokenId,
			amount: BalanceOf<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// token exists check
			let token = nft::Module::<T>::tokens(collection_id, token_id).ok_or(Error::<T>::TokenNotFound)?;

			let balance = T::Currency::free_balance(&_who);
			ensure!(balance >= amount, Error::<T>::BalanceNotEnough);

			T::Currency::transfer(&_who, &token.owner, amount, ExistenceRequirement::AllowDeath)?;
			Self::deposit_event(RawEvent::AppreciationReceived(collection_id, token_id, amount));

			Ok(())	
		}

		#[weight = 0]
		pub fn toogle_display(origin,
			collection_id: T::ClassId,
			token_id: T::TokenId,
			display: bool) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// token exists check
			let token = nft::Module::<T>::tokens(collection_id, token_id).ok_or(Error::<T>::TokenNotFound)?;
			ensure!(token.owner == _who, Error::<T>::MustBeTokenOwner);

			// get token info
			let mut info = TokenExtendedInfo::<T>::get(collection_id, token_id).unwrap_or_else(|| ExtendedInfo {
				display_flag: false,
				report: ReportReason::None,
			});
			info.display_flag = display;

			TokenExtendedInfo::<T>::insert(collection_id, token_id, info);
			Self::deposit_event(RawEvent::ToggleDisplay(display));

			Ok(())	
		}

		#[weight = 0]
		pub fn report(origin,
			collection_id: T::ClassId,
			token_id: T::TokenId,
			reason: ReportReason) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// token exists check
			ensure!(nft::Module::<T>::tokens(collection_id, token_id).is_some(), Error::<T>::TokenNotFound);

			// get token info
			let mut info = TokenExtendedInfo::<T>::get(collection_id, token_id).unwrap_or_else(|| ExtendedInfo {
				display_flag: false,
				report: ReportReason::None,
			});
			info.report = reason.clone();

			TokenExtendedInfo::<T>::insert(collection_id, token_id, info);
			Self::deposit_event(RawEvent::ArtReported(collection_id, token_id, reason));

			Ok(())	
		}

		#[weight = 0]
		pub fn accept_report(origin,
			collection_id: T::ClassId,
			token_id: T::TokenId) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// token exists check
			ensure!(nft::Module::<T>::tokens(collection_id, token_id).is_some(), Error::<T>::TokenNotFound);

			ensure!(Curator::<T>::get() == _who, Error::<T>::MustBeCurator);

			// get token info
			let mut info = TokenExtendedInfo::<T>::get(collection_id, token_id).unwrap_or_else(|| ExtendedInfo {
				display_flag: false,
				report: ReportReason::None,
			});
			info.report = ReportReason::Reported;

			Self::deposit_event(RawEvent::ArtReportAccepted(collection_id, token_id));

			Ok(())	
		}

		#[weight = 0]
		pub fn clear_report(origin,
			collection_id: T::ClassId,
			token_id: T::TokenId) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// token exists check
			ensure!(nft::Module::<T>::tokens(collection_id, token_id).is_some(), Error::<T>::TokenNotFound);

			ensure!(Curator::<T>::get() == _who, Error::<T>::MustBeCurator);

			// get token info
			let mut info = TokenExtendedInfo::<T>::get(collection_id, token_id).unwrap_or_else(|| ExtendedInfo {
				display_flag: false,
				report: ReportReason::None,
			});
			info.report = ReportReason::Reported;

			Self::deposit_event(RawEvent::ArtReportCleared(collection_id, token_id));

			Ok(())	
		}

		#[weight = 0]
		pub fn set_curator(origin,
			curator: T::AccountId) -> DispatchResult {
			let _who = ensure_root(origin)?;

			Curator::<T>::put(curator);	

			Ok(())	
		}
	}
}
