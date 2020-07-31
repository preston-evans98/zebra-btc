use proptest::{arbitrary::any, collection::vec, prelude::*};

use crate::notes::sprout;

impl Arbitrary for sprout::EncryptedCiphertext {
    type Parameters = ();

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (vec(any::<u8>(), 601))
            .prop_map(|v| {
                let mut bytes = [0; 601];
                bytes.copy_from_slice(v.as_slice());
                Self(bytes)
            })
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}