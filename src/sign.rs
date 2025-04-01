/// Specifies the recipient of an invoice.
///
/// This indicates to [`NodeSigner::sign_invoice`] what node secret key should be used to sign
/// the invoice.
#[derive(Clone, Copy)]
pub enum Recipient {
    /// The invoice should be signed with the local node secret key.
    Node,
    /// The invoice should be signed with the phantom node secret key. This secret key must be the
    /// same for all nodes participating in the [phantom node payment].
    ///
    /// [phantom node payment]: PhantomKeysManager
    PhantomNode,
}

/// A trait that describes a source of entropy.
pub trait EntropySource {
    /// Gets a unique, cryptographically-secure, random 32-byte value. This method must return a
    /// different value each time it is called.
    fn get_secure_random_bytes(&self) -> [u8; 32];
}
