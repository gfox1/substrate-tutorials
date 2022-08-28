#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	// Added this line to include Currency and Randomness in pallet
	use frame_support::traits::{Currency, Randomness};


	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);
		
	// Allows easy access our Pallet's `Balance` type. Comes from `Currency` interface.
	type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	// The Gender type used in the `Kitty` struct
	#[derive(Clone, Encode, Decode, PartialEq, Copy, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum Gender {
		Male,
		Female,
	}

	// Struct for holding kitty information
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Copy)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		// Using 16 bytes to represent a kitty DNA
		pub dna: [u8; 16],
		// `None` assumes not for sale
		pub price: Option<BalanceOf<T>>,
		pub gender: Gender,
		pub owner: T::AccountId,
	}

// Your Pallet's configuration trait, representing custom external types and interfaces.
#[pallet::config]
pub trait Config: frame_system::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

    /// The Currency handler for the kitties pallet.
    type Currency: Currency<Self::AccountId>;

    /// The maximum amount of kitties a single account can own.
    #[pallet::constant]
    type MaxKittiesOwned: Get<u32>;

    /// The type of Randomness we want to specify for this pallet.
    type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;
}

	/// Keeps track of the number of kitties in existence.
	#[pallet::storage]
	pub(super) type CountForKitties<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Maps the kitty struct to the kitty DNA.
	#[pallet::storage]
	pub(super) type Kitties<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], Kitty<T>>;

	/// Track the kitties owned by each account.
	#[pallet::storage]
	pub(super) type KittiesOwned<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<[u8; 16], T::MaxKittiesOwned>,
		ValueQuery,
	>;


	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]

		// New kitty was created succesfully
		Created { kitty: [u8; 16], owner: T::AccountId},
		// Kitty was successfully transfered. 
		Transferred { from: T::AccountId, to: T::AccountID, kitty [u8; 16] },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Account has reached Max Kitties owned
		TooManyOwned,
		/// This kitt already exisits
		DuplicateKitty,
		/// An overflow has occured!
		Overflow,
		// Kitty does nto exist
		NoKitty,
		// You are nto the owner
		NotOwner,
		// Trying to transfer or buy a kitty from oneself
		TransferToSelf, 
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	// Your Pallet's callable functions.
    impl<T: Config> Pallet<T> {

		// Create a new unique kitty
		// The acutal kitty creation is done in the 'mint()' function.
		#[pallet::weight(0)]
		pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			// Generate uniqiue DNA and Gender using a helper function
			let (kitty_gen_dna, gender) = Self::gen_dna();

			// Write new kitty to storage by calling helper function
			Self::mint(&sender, kitty_gen_dna, gender)?;

			Ok(())
		}


		/// Directly transfer a kitty to another recipient.
		///
		/// Any account that holds a kitty can send it to another Account. This will reset the
		/// asking price of the kitty, marking it not for sale.
		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			kitty_id: [u8; 16],
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let from = ensure_signed(origin)?;
			let kitty = Kitties::<T>::get(&kitty_id).ok_or(Error::<T>::NoKitty)?;
			ensure!(kitty.owner == from, Error::<T>::NotOwner);
			Self::do_transfer(kitty_id, to)?;
			Ok(())
		}


	}
	
	
	
	// Your Pallet's internal functions (not callable by users).
	 // Your Pallet's internal functions.
	 impl<T: Config> Pallet<T> {
        // Generates and returns DNA and Gender
        fn gen_dna() -> ([u8; 16], Gender) {
            // Create randomness
            let random = T::KittyRandomness::random(&b"dna"[..]).0;

            // Create randomness payload. Multiple kitties can be generated in the same block,
            // retaining uniqueness.
            let unique_payload = (
                random,
                frame_system::Pallet::<T>::extrinsic_index().unwrap_or_default(),
                frame_system::Pallet::<T>::block_number(),
            );

            // Turns into a byte array
            let encoded_payload = unique_payload.encode();
            let hash = frame_support::Hashable::blake2_128(&encoded_payload);

            // Generate Gender
            if hash[0] % 2 == 0 {
                (hash, Gender::Male)
            } else {
                (hash, Gender::Female)
            }
        }

        // Helper to mint a kitty
        pub fn mint(
            owner: &T::AccountId,
            dna: [u8; 16],
            gender: Gender,
        ) -> Result<[u8; 16], DispatchError> {
            // Create a new object
            let kitty = Kitty::<T> { dna, price: None, gender, owner: owner.clone() };

            // Check if the kitty does not already exist in our storage map
            ensure!(!Kitties::<T>::contains_key(&kitty.dna), Error::<T>::DuplicateKitty);

            // Performs this operation first as it may fail
            let count = CountForKitties::<T>::get();
            let new_count = count.checked_add(1).ok_or(Error::<T>::Overflow)?;

            // Append kitty to KittiesOwned
            KittiesOwned::<T>::try_append(&owner, kitty.dna)
                .map_err(|_| Error::<T>::TooManyOwned)?;

            // Write new kitty to storage
            Kitties::<T>::insert(kitty.dna, kitty);
            CountForKitties::<T>::put(new_count);

            // Deposit our "Created" event.
            Self::deposit_event(Event::Created { kitty: dna, owner: owner.clone() });

            // Returns the DNA of the new kitty if this succeeds
            Ok(dna)
        }

		// Transfer kitty function
		// Update storage to transfer kitty
		pub fn do_transfer(
			kitty_id: [u8; 16],
			to: T::AccountId,
		) -> DispatchResult {
			// get kitty
			let mut kitty = Kittens::<T>::get(&kitty_id).ok_or(Error::<T>::NoKitty)?;
			let from = kitty.owner;

			ensure!(from != to, Error::<T>::TransferToSelf);
			let mut from_owned = KittiesOwned::<T>::get(&from);

			// Remove kitty from list of owned kitties. 
			if let Some(ind) = from_owned.iter().position(|&id| id == kitty_id) {
				from_owned.swap_remove(ind);
			}	else {
				return Err(Error::<T>::NoKitty.into())
			}

			// Add kitty to list of owned kitties
			let mut to_owned = KittiesOwned::<T>::get(&to);
			to_owned.try_push(kitty_id).map_err(|()|Error::T::TooMany)?;

			// Transfer succeeded update the kitty owner and reset the price to 'None'
			kitty.owner = to.clone();
			kitty.price = None; 

			// Write updates to storage 
			Kitties::<T>insert(&kitty_id, kitty);
			KittiesOwned::<T>::insert(&to, to_owned);
			KittiesOwned::<T>::insert(&from, from_owned);

			Self::deposit_event(Event::Transferred {from, to, kitty: kitty_id });

			Ok(())
		}
    }
}