#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::Randomness};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sp_io::hashing::blake2_128;
use sp_runtime::ArithmeticError;

// Struct for holding Kitty information.
// encode and decode: transform into binary data
// RuntimeDebug: allow to print the format of the kitty struct
// PartialEq to compare Kitty
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq)]
pub struct Kitty<Hash> {
    dna: [u8; 16],
    gender: Gender,
}

// Enum declaration for Gender.
#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub enum Gender {
    Male,
    Female,
}

// Implementation to handle Gender type in Kitty struct.
impl Default for Gender {
    fn default() -> Self {
        Gender::Male
    }
}

#[frame_support::pallet]
pub mod pallet {

    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_randomness_collective_flip::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// The type of Randomness we want to specify for this pallet.
		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;
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
        Kitty<T::Hash>,
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
        KittyCreated(T::AccountId, u32, Kitty<T::Hash>),
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Error for the kitties pallet.
    #[pallet::error]
    pub enum Error<T> {
        KittiesIdOverflow,
        KittyNotExist,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new kitty
        #[pallet::weight(1000)]
        pub fn create(origin: OriginFor<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            // TODO: ensure kitty id does not overflow
            NextKittyId::try_mutate(|next_id| -> DispatchResult {
                let current_id = *next_id;
                *next_id = next_id
                    .checked_add(1)
                    .ok_or(Error::<T>::KittiesIdOverflow)?;

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
            })?;
        }
    }

    // Helper function for Kitty struct
    impl<T: Config> Kitty<T> {
        pub fn gender(dna: T::Hash) -> Gender {
            if dna.as_ref()[0] % 2 == 0 {
                Gender::Male
            } else {
                Gender::Female
            }
        }
    }

    impl<T: Config> Pallet<T> {
        // Generate a random gender value
        fn gen_gender() -> Gender {
            let random = T::KittyRandomness::random(&b"gender"[..]).0;
            match random.as_ref()[0] % 2 {
                0 => Gender::Male,
                _ => Gender::Female,
            }
        }

        // Generate a random DNA value
        fn gen_dna() -> [u8; 16] {
            let payload = (
                T::KittyRandomness::random(&b"dna"[..]).0,
                <frame_system::Pallet<T>>::block_number(),
            );
            payload.using_encoded(blake2_128)
        }

        // Create new DNA with existing DNA
        pub fn breed_dna(kid1: &T::Hash, kid2: &T::Hash) -> Result<[u8; 16], Error<T>> {
            let dna1 = Self::kitties(kid1).ok_or(<Error<T>>::KittyNotExist)?.dna;
            let dna2 = Self::kitties(kid2).ok_or(<Error<T>>::KittyNotExist)?.dna;

            let mut new_dna = Self::gen_dna();
            for i in 0..new_dna.len() {
                new_dna[i] = (new_dna[i] & dna1[i]) | (!new_dna[i] & dna2[i]);
            }
            Ok(new_dna)
        }
    }
}
