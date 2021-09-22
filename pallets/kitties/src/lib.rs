#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::Randomness};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sp_io::hashing::blake2_128;
use sp_runtime::ArithmeticError;

#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq)]
pub enum KittyGender {
    Male,
    Female,
}

// Struct for holding Kitty information.
// encode and decode: transform into binary data
// RuntimeDebug: allow to print the format of the kitty struct
// PartialEq to compare Kitty
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Kitty(pub [u8; 16]);

impl Kitty {
    pub fn gender(&self) -> KittyGender {
        if self.0[0] % 2 == 0 {
            KittyGender::Male
        } else {
            KittyGender::Female
        }
    }
}

// Enum declaration for Gender.
#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub enum Gender {
    Male,
    Female,
}

#[frame_support::pallet]
pub mod pallet {

    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_randomness_collective_flip::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    // blake2 128 bit secure hasher is the default to keep it simple
    /// Stores all the kitties. Key is (user, kitty_id).
    #[pallet::storage]
    #[pallet::getter(fn kitties)]
    pub type Kitties<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        u32,
        Kitty,
        OptionQuery,
    >;

    /// Stores the next kitty Id.
    #[pallet::storage]
    #[pallet::getter(fn next_kitty_id)]
    pub type NextKittyId<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        /// A kitty is created. \[owner, kitty_id, kitty\]
        KittyCreated(T::AccountId, u32, Kitty),
        KittyBred(T::AccountId, u32, Kitty),
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Error for the kitties pallet.
    #[pallet::error]
    pub enum Error<T> {
        SameGender,
        InvalidKittyId,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new kitty
        #[pallet::weight(1000)]
        pub fn create(origin: OriginFor<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            NextKittyId::<T>::try_mutate(|next_id| -> DispatchResult {
                let current_id = *next_id;
                *next_id = next_id.checked_add(1).ok_or(ArithmeticError::Overflow)?;

                // Generate a random 128bit value
                let payload = (
                    <pallet_randomness_collective_flip::Pallet<T> as Randomness<
                        T::Hash,
                        T::BlockNumber,
                    >>::random_seed()
                    .0,
                    &sender,
                    <frame_system::Pallet<T>>::extrinsic_index(),
                );
                let dna = payload.using_encoded(blake2_128);

                // Create and store kitty
                let kitty = Kitty(dna);
                Kitties::<T>::insert(&sender, current_id, &kitty);

                // Emit event
                Self::deposit_event(Event::KittyCreated(sender, current_id, kitty));

                Ok(())
            })
        }

        /// Breed kitties
        #[pallet::weight(1000)]
        pub fn breed(origin: OriginFor<T>, kitty_id_1: u32, kitty_id_2: u32) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let kitty1 = Self::kitties(&sender, kitty_id_1).ok_or(Error::<T>::InvalidKittyId)?;
            let kitty2 = Self::kitties(&sender, kitty_id_2).ok_or(Error::<T>::InvalidKittyId)?;

            ensure!(kitty1.gender() != kitty2.gender(), Error::<T>::SameGender);

            NextKittyId::<T>::try_mutate(|next_id| -> DispatchResult {
                let kitty_id = *next_id;
                *next_id = next_id.checked_add(1).ok_or(ArithmeticError::Overflow)?;

                let kitty1_dna = kitty1.0;
                let kitty2_dna = kitty2.0;

                let payload = (
                    <pallet_randomness_collective_flip::Pallet<T> as Randomness<
                        T::Hash,
                        T::BlockNumber,
                    >>::random_seed()
                    .0,
                    &sender,
                    <frame_system::Pallet<T>>::extrinsic_index(),
                );
                let selector = payload.using_encoded(blake2_128);

                let mut new_dna = [0u8; 16];

                // Combine parents and selector to create new kitty
                for i in 0..kitty1_dna.len() {
                    new_dna[i] = (selector[i] & kitty1_dna[i]) | (!selector[i] & kitty2_dna[i]);
                }

                let new_kitty = Kitty(new_dna);

                Kitties::<T>::insert(&sender, kitty_id, &new_kitty);

                Self::deposit_event(Event::KittyBred(sender, kitty_id, new_kitty));
                Ok(())
            })
        }
    }
}
