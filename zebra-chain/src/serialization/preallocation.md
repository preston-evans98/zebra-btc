## Safe Preallocation for (most) `Vec<T>`

Currently, Zebra [doesn't preallocate Vectors](https://github.com/ZcashFoundation/zebra/blob/main/zebra-chain/src/serialization/zcash_deserialize.rs) when deserializing, since blind preallocations present a DOS vector. Instead, it relies on maximum block and transaction size to limit allocations. This is inefficient.

We can mitigate the DOS potential and allow for preallocation as follows:

1. Define a new trait `SafeAllocate`, defining the maximum length a Vec<T: SafeAllocate> can sensibly reach for each implementing type
1. Create a specialized `ZcashDeserialize` implementation for `SafeAllocate`rs.

```
pub trait SafeAllocate {
    const fn max_allocation() -> usize;
}

impl<T: ZcashDeserialize + SafeAllocate> ZcashDeserialize for Vec<T> {
    fn zcash_deserialize<R: io::Read>(mut reader: R) -> Result<Self, SerializationError> {
        let len = reader.read_compactsize()?;
        if len > T::max_allocation() {
            return Err(SerializationError::Parse(("Vector longer than max_allocation")));
        }
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(T::zcash_deserialize(&mut reader)?);
        }
        Ok(vec)
    }
}
```

This allows us to pre-allocate for certain types without presenting a DOS vector, while retaining the flexibilty to use the current (lazy) allocation strategy for types that defy safe blind allocation.

Note that deserialization using this method is guaranteed to fail during the deserialization of the first malicious vector. However, Block messages contain nested `Vec<T>` types, and it is possible for a Byzantine message to cause improper allocations at each level of nesting before being discovered. This is why the potential spike for a Block message is much higher than for other message types.

### Potential SafeAllocation Implementors

- `Arc<Transaction>` - Suggested limit: 49_000. A Zcash block is capped at 2MB. Since transparent Txs are the smallest allowable transactions, and each transparent Tx requires at least one Input with a minimum serialized size of 41 bytes, an upper bound the length of a `Vec<Arc<Transaction>>` is 2MB / 41 Bytes < 49_000. Note that `std::mem::size_of::<Arc<T>>()` is 8, so the maximum memory wasted by a Byzantine `Vec<Arc<Transaction>>` is 49_000 \* 8 Bytes < 400KB.

- `transparent::Input` - Suggested limit: 49_000. A Zcash block is capped at 2MB. Each Input has a minimum serialized size of 41 bytes, so an upper bound the length of a `Vec<transparent::Input>` is 2MB / 41 Bytes < 49_000. With Inputs taking less than 80 bytes on the stack (36 for the Outpoint, 26 for the Script, and 4 for the sequence), the maximum memory wasted by a Byzantine `Vec<transparent::Input>` is 49_000 \* 80 Bytes < 4MB.

- `transparent::Output` - Suggested limit: 225_000. A Zcash block is capped at 2MB. Outputs have a minimum serialized size of 9 bytes, so an upper bound the length of a `Vec<Arc<transparent::Output>>` is 2MB / 9 Bytes < 225_000. With Outputs taking less than 40 bytes on the stack (call it 10 for the value, 26 for the Script, for a total of 36 ),the maximum memory wasted by a Byzantine `Vec<Arc<transparent::Output>>` is 225_000 \* 40 Bytes = 9MB.

- `MetaAddr` - Suggested limit: 1000. No fancy math required, since this limit is in the Addr message [specification](https://developer.bitcoin.org/reference/p2p_networking.html#addr). Estimate a MetaAddr at a generous 100 bytes of stack space and you get max memory waste = 1000 \* 100B = 100KB.

- `block::Hash` - Suggested limit: 65536. We derive this limit as MAX_MESSAGE_SIZE / 32 = 2 \* 1024 \_ 1024 / 32. Since a block hash takes 32 bytes on the stack, the max waste here is MAX_MESSAGE_SIZE.

- `InventoryHash` - Suggested limit: 50,000. This limit is the listed in the [Inv message spec](https://developer.bitcoin.org/reference/p2p_networking.html#inv). Maximum waste: 50,000 \* 32 B = 1.6 MB.

- `block::CountedHeader` - Suggested limit: 2000. This limit is in the [Headers message spec](https://developer.bitcoin.org/reference/p2p_networking.html#headers). Per the most recent spec, each [Zcash header is less than 2kb](https://zips.z.cash/protocol/protocol.pdf#page=90&zoom=100,72,73). Maximum waste: 2000 \* 2000 Bytes < 4MB

- `u8` - Suggested limit: MAX_MESSAGE_SIZE. Since a u8 takes 1 byte on the stack, the maximum waste here is MAX_MESSAGE_SIZE = 2MB.

### Example attacks

Using all of these numbers, we calculate the total improper allocation caused by the worst-case instance of each Zcash Message as follows:

- Version: N.A.
- Verack: N.A.
- Ping: N.A.
- Pong: N.A.
- Reject: N.A.
- GetAddr: N.A.
- Addr: contains 1 `Vec<MetaAddr>` => **100KB** (see above)
- Get locks: contains 1 `Vec<block::Hash>` => **MAX_MESSAGE_SIZE** (2MB)
- Inv: contains 1 `Vec<InventoryHash>` => **1.6 MB** (see above)
- GetHeaders: contains 1 `Vec<block::Hash>` => **MAX_MESSAGE_SIZE** (2MB)
- Headers: contains 1 `Vec<block::CountedHeader>` => **4MB**
- GetData: contains 1 `Vec<InventoryHash>` => **1.6 MB** (see above)
- Block: The worst case block contains 1 `Vec<Arc<Transaction>`, 1 `Vec<transparent::Output>`, and 1 malicious `Script(Vec<u8>)` => 400KB + 9MB + 2MB = **11.4 MB**.
- - Note that a dishonest vector is discovered during its own deserialization, so a malicious `Vec<transparent::Input>` would be discovered before the malicious `Vec<transparent::Output>` was allocated for. Since Outputs can waste more memory than Inputs, a smart attacker will choose to make only his `Vec<transparent::Output>` malicious.
- Tx: N.A.
- NotFound: contains 1 `Vec<InventoryHash>` => **1.6 MB** (see above)
- Mempool: N.A.
- FilterLoad: contains 1 `Filter(Vec<u8>)` => **2MB**
- FilterAdd: contains 1 `Vec<u8>` => **2MB**
- FilterClear: N.A.

### Summary and Recommendations

With the `SafeAllocate` trait, we can allow preallocation for many Vector types with negligble risk of DOS attacks.

In the worst case, a malicious message can cause a short-lived spike in memory usage. The size of this spike depends on the max_allocation defined in `SafeAllocate` and the depth of nested `Vec<T: SafeAllocate>` types. Calculations of the maximum spike caused by each message are included above. Based on these calculations, I recommend implementing SafeAllocate for all types listed in the "Potential SafeAllocate Implementors" section, with the possible exception of `transparent::Input` and `transparent::Output`.

If this recommendation is adopted , the worst case memory spike that a malicious peer can cause will be roughly 4MB, or roughly double that peer connection's usual memory consumption.

If `transparent::Input`s and `transparent::Output`s are included, the worst case spike rises to 11.5Mb, or about six times a peer connection's usual memory consumption.
