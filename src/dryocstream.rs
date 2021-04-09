//! # Encrypted streams
//!
//! [`DryocStream`] implements libsodium's secret-key authenticated stream
//! encryption, also known as a _secretstream_. This implementation uses the
//! XSalsa20 stream cipher, and Poly1305 for message authentication.
//!
//! You should use a [`DryocStream`] when you want to:
//!
//! * read and write messages to a file or network socket
//! * exchange messages between two parties
//! * send messages in a particular sequence, and authenticate the order of
//!   messages
//! * provide a way to determine the start and end of a sequence of messages
//! * use a shared secret, such as a passphrase, which can be used to derive a
//!   secret key using `crypto_pwhash_*`
//!
//! # Rustaceous API example
//!
//! ```
//! use dryoc::dryocstream::*;
//! let message1 = b"Arbitrary data to encrypt";
//! let message2 = b"split into";
//! let message3 = b"three messages";
//!
//! // Generate a random secret key for this stream
//! let key = Key::gen();
//!
//! // Initialize the push side, type annotations required on return type
//! let (mut push_stream, header): (_, Header) = DryocStream::init_push(&key);

//! // Encrypt a series of messages
//! let c1 = push_stream
//! .push_to_vec(message1, None, Tag::MESSAGE)
//! .expect("Encrypt failed");
//! let c2 = push_stream
//! .push_to_vec(message2, None, Tag::MESSAGE)
//! .expect("Encrypt failed");
//! let c3 = push_stream
//! .push_to_vec(message3, None, Tag::FINAL)
//! .expect("Encrypt failed");
//!
//! // Initialize the pull side using header generated by the push side
//! let mut pull_stream = DryocStream::init_pull(&key, &header);
//!
//! // Decrypt the encrypted messages, type annotations required
//! let (m1, tag1) = pull_stream.pull_to_vec(&c1, None).expect("Decrypt
//! failed"); let (m2, tag2) = pull_stream.pull_to_vec(&c2,
//! None).expect("Decrypt failed"); let (m3, tag3) =
//! pull_stream.pull_to_vec(&c3, None).expect("Decrypt failed");
//!
//! assert_eq!(message1, m1.as_slice());
//! assert_eq!(message2, m2.as_slice());
//! assert_eq!(message3, m3.as_slice());
//!
//! assert_eq!(tag1, Tag::MESSAGE);
//! assert_eq!(tag2, Tag::MESSAGE);
//! assert_eq!(tag3, Tag::FINAL);
//! ```
//! 
//! ## Additional resources
//!
//! * See <https://libsodium.gitbook.io/doc/secret-key_cryptography/secretstream>
//!   for additional details on secret streams
//! * For public-key based encryption, see [`DryocBox`](crate::dryocbox)
//! * For stream encryption, see [`DryocStream`](crate::dryocstream)
//! * See [protected] for an example using the protected memory features with
//!   [`DryocStream`]

use bitflags::bitflags;
use zeroize::Zeroize;

use crate::classic::crypto_secretstream_xchacha20poly1305::{
    crypto_secretstream_xchacha20poly1305_init_pull,
    crypto_secretstream_xchacha20poly1305_init_push, crypto_secretstream_xchacha20poly1305_pull,
    crypto_secretstream_xchacha20poly1305_push, crypto_secretstream_xchacha20poly1305_rekey, State,
};
use crate::constants::{
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_HEADERBYTES,
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_KEYBYTES,
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_MESSAGE,
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_PUSH,
    CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_REKEY, CRYPTO_STREAM_CHACHA20_IETF_NONCEBYTES,
};
use crate::error::Error;
pub use crate::types::*;

/// Stream mode marker trait
pub trait Mode {}
/// Indicates a push stream
pub struct Push;
/// Indicates a pull stream
pub struct Pull;

impl Mode for Push {}
impl Mode for Pull {}

/// Stack-allocated secret for authenticated secret streams.
pub type Key = StackByteArray<CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_KEYBYTES>;
/// Stack-allocated nonce for authenticated secret streams.
pub type Nonce = StackByteArray<CRYPTO_STREAM_CHACHA20_IETF_NONCEBYTES>;
/// Stack-allocated header data for authenticated secret streams.
pub type Header = StackByteArray<CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_HEADERBYTES>;

#[cfg(any(feature = "nightly", all(doc, not(doctest))))]
#[cfg_attr(all(feature = "nightly", doc), doc(cfg(feature = "nightly")))]
pub mod protected {
    //! #  Protected memory type aliases for [`DryocStream`]
    //!
    //! This mod provides re-exports of type aliases for protected memory usage
    //! with [`DryocStream`]. These type aliases are provided for convenience.
    //!
    //! ## Example
    //! ```
    //! use dryoc::dryocstream::protected::*;
    //! use dryoc::dryocstream::{DryocStream, Tag};
    //!
    //! // Load some message into locked readonly memory.
    //! let message1 = HeapBytes::from_slice_into_readonly_locked(b"Arbitrary data to encrypt")
    //!     .expect("from slice failed");
    //! let message2 =
    //!     HeapBytes::from_slice_into_readonly_locked(b"split into").expect("from slice failed");
    //! let message3 =
    //!     HeapBytes::from_slice_into_readonly_locked(b"three messages").expect("from slice failed");
    //!
    //! // Generate a random key into locked readonly memory.
    //! let key = Key::gen_readonly_locked().expect("key failed");
    //!
    //! // Initialize the push stream, place the header into locked memory
    //! let (mut push_stream, header): (_, Locked<Header>) = DryocStream::init_push(&key);
    //!
    //! // Encrypt the set of messages, placing everything into locked memory.
    //! let c1: LockedBytes = push_stream
    //!     .push(&message1, None, Tag::MESSAGE)
    //!     .expect("Encrypt failed");
    //! let c2: LockedBytes = push_stream
    //!     .push(&message2, None, Tag::MESSAGE)
    //!     .expect("Encrypt failed");
    //! let c3: LockedBytes = push_stream
    //!     .push(&message3, None, Tag::FINAL)
    //!     .expect("Encrypt failed");
    //!
    //! // Initialized the pull stream
    //! let mut pull_stream = DryocStream::init_pull(&key, &header);
    //!
    //! // Decrypt the set of messages, putting everything into locked memory
    //! let (m1, tag1): (LockedBytes, Tag) = pull_stream.pull(&c1, None).expect("Decrypt failed");
    //! let (m2, tag2): (LockedBytes, Tag) = pull_stream.pull(&c2, None).expect("Decrypt failed");
    //! let (m3, tag3): (LockedBytes, Tag) = pull_stream.pull(&c3, None).expect("Decrypt failed");
    //!
    //! assert_eq!(message1.as_slice(), m1.as_slice());
    //! assert_eq!(message2.as_slice(), m2.as_slice());
    //! assert_eq!(message3.as_slice(), m3.as_slice());
    //!
    //! assert_eq!(tag1, Tag::MESSAGE);
    //! assert_eq!(tag2, Tag::MESSAGE);
    //! assert_eq!(tag3, Tag::FINAL);
    //! ```
    use super::*;
    pub use crate::protected::*;
    pub use crate::types::*;

    /// Heap-allocated, page-aligned secret key for authenticated secret
    /// streams, for use with protected memory
    pub type Key = HeapByteArray<CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_KEYBYTES>;
    /// Heap-allocated, page-aligned nonce for authenticated secret
    /// streams, for use with protected memory
    pub type Nonce = HeapByteArray<CRYPTO_STREAM_CHACHA20_IETF_NONCEBYTES>;
    /// Heap-allocated, page-aligned header for authenticated secret
    /// streams, for use with protected memory
    pub type Header = HeapByteArray<CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_HEADERBYTES>;
}

bitflags! {
    /// Message tag definitions
    pub struct Tag: u8 {
        /// Describes a normal message in a stream.
        const MESSAGE = CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_MESSAGE;
        /// Indicates the message marks the end of a series of messages in a
        /// stream, but not the end of the stream.
        const PUSH = CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_PUSH;
        /// Derives a new key for the stream.
        const REKEY = CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_TAG_REKEY;
        /// Indicates the end of the stream.
        const FINAL = Self::PUSH.bits | Self::REKEY.bits;
    }
}

impl From<u8> for Tag {
    fn from(other: u8) -> Self {
        Self::from_bits(other).expect("Unable to parse tag")
    }
}

/// Secret-key authenticated encrypted streams
#[derive(PartialEq, Clone, Zeroize)]
pub struct DryocStream<Mode> {
    #[zeroize(drop)]
    state: State,
    phantom: std::marker::PhantomData<Mode>,
}

impl<M> DryocStream<M> {
    /// Manually rekeys the stream. Both the push and pull sides of the stream
    /// need to manually rekey if you use this function (i.e., it's not handled
    /// by the library).
    ///
    /// Automatic rekeying will occur normally, and you generally should need to
    /// manually rekey.
    ///
    /// Refer to the [libsodium
    /// docs](https://libsodium.gitbook.io/doc/secret-key_cryptography/secretstream#rekeying)
    /// for details.
    pub fn rekey(&mut self) {
        crypto_secretstream_xchacha20poly1305_rekey(&mut self.state)
    }
}

impl DryocStream<Push> {
    /// Returns a new push stream, initialized from `key`.
    pub fn init_push<
        Key: ByteArray<CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_KEYBYTES>,
        Header: NewByteArray<CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_HEADERBYTES>,
    >(
        key: &Key,
    ) -> (Self, Header) {
        let mut state = State::new();
        let mut header = Header::new_byte_array();
        crypto_secretstream_xchacha20poly1305_init_push(
            &mut state,
            header.as_mut_array(),
            key.as_array(),
        );
        (
            Self {
                state,
                phantom: std::marker::PhantomData,
            },
            header,
        )
    }

    /// Encrypts `message` for this stream with `associated_data` and `tag`,
    /// returning the ciphertext.
    pub fn push<Input: Bytes, Output: NewBytes + ResizableBytes>(
        &mut self,
        message: &Input,
        associated_data: Option<&Input>,
        tag: Tag,
    ) -> Result<Output, Error> {
        use crate::constants::CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_ABYTES;
        let mut ciphertext = Output::new_bytes();
        ciphertext.resize(
            message.as_slice().len() + CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_ABYTES,
            0,
        );
        crypto_secretstream_xchacha20poly1305_push(
            &mut self.state,
            ciphertext.as_mut_slice(),
            message.as_slice(),
            associated_data.map(|aad| aad.as_slice()),
            tag.bits(),
        )?;
        Ok(ciphertext)
    }

    /// Encrypts `message` for this stream with `associated_data` and `tag`,
    /// returning the ciphertext.
    pub fn push_to_vec<Input: Bytes>(
        &mut self,
        message: &Input,
        associated_data: Option<&Input>,
        tag: Tag,
    ) -> Result<Vec<u8>, Error> {
        self.push(message, associated_data, tag)
    }
}

impl DryocStream<Pull> {
    /// Returns a new pull stream, initialized from `key` and `header`.
    pub fn init_pull<
        Key: ByteArray<CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_KEYBYTES>,
        Header: ByteArray<CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_HEADERBYTES>,
    >(
        key: &Key,
        header: &Header,
    ) -> Self {
        let mut state = State::new();
        crypto_secretstream_xchacha20poly1305_init_pull(
            &mut state,
            header.as_array(),
            key.as_array(),
        );
        Self {
            state,
            phantom: std::marker::PhantomData,
        }
    }

    /// Decrypts `ciphertext` for this stream with `associated_data`, returning
    /// the decrypted message and tag.
    pub fn pull<Input: Bytes, Output: MutBytes + Default + ResizableBytes>(
        &mut self,
        ciphertext: &Input,
        associated_data: Option<&Input>,
    ) -> Result<(Output, Tag), Error> {
        use crate::constants::CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_ABYTES;
        let mut message = Output::default();
        message.resize(
            ciphertext.as_slice().len() - CRYPTO_SECRETSTREAM_XCHACHA20POLY1305_ABYTES,
            0,
        );
        let mut tag = 0u8;
        crypto_secretstream_xchacha20poly1305_pull(
            &mut self.state,
            message.as_mut_slice(),
            &mut tag,
            ciphertext.as_slice(),
            associated_data.map(|aad| aad.as_slice()),
        )?;

        Ok((message, Tag::from_bits(tag).expect("invalid tag")))
    }

    /// Decrypts `ciphertext` for this stream with `associated_data`, returning
    /// the decrypted message and tag into a [`Vec`].
    pub fn pull_to_vec<Input: Bytes>(
        &mut self,
        ciphertext: &Input,
        associated_data: Option<&Input>,
    ) -> Result<(Vec<u8>, Tag), Error> {
        self.pull(ciphertext, associated_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_push() {
        use sodiumoxide::crypto::secretstream::{
            Header as SOHeader, Key as SOKey, Stream as SOStream, Tag as SOTag,
        };

        let message1 = b"Arbitrary data to encrypt";
        let message2 = b"split into";
        let message3 = b"three messages";

        // Generate a random secret key for this stream
        let key = Key::gen();

        // Initialize the push side, type annotations required on return type
        let (mut push_stream, header): (_, Header) = DryocStream::init_push(&key);
        // Encrypt a series of messages
        let c1: Vec<u8> = push_stream
            .push(message1, None, Tag::MESSAGE)
            .expect("Encrypt failed");
        let c2: Vec<u8> = push_stream
            .push(message2, None, Tag::MESSAGE)
            .expect("Encrypt failed");
        let c3: Vec<u8> = push_stream
            .push(message3, None, Tag::FINAL)
            .expect("Encrypt failed");

        // Initialize the pull side using header generated by the push side
        let mut so_stream_pull = SOStream::init_pull(
            &SOHeader::from_slice(header.as_slice()).expect("header failed"),
            &SOKey::from_slice(key.as_slice()).expect("key failed"),
        )
        .expect("pull init failed");

        let (m1, tag1) = so_stream_pull.pull(&c1, None).expect("decrypt failed");
        let (m2, tag2) = so_stream_pull.pull(&c2, None).expect("decrypt failed");
        let (m3, tag3) = so_stream_pull.pull(&c3, None).expect("decrypt failed");

        assert_eq!(message1, m1.as_slice());
        assert_eq!(message2, m2.as_slice());
        assert_eq!(message3, m3.as_slice());

        assert_eq!(tag1, SOTag::Message);
        assert_eq!(tag2, SOTag::Message);
        assert_eq!(tag3, SOTag::Final);
    }

    #[test]
    fn test_stream_pull() {
        use std::convert::TryFrom;

        use sodiumoxide::crypto::secretstream::{Key as SOKey, Stream as SOStream, Tag as SOTag};

        let message1 = b"Arbitrary data to encrypt";
        let message2 = b"split into";
        let message3 = b"three messages";

        // Generate a random secret key for this stream
        let key = Key::gen();

        // Initialize the push side, type annotations required on return type
        let (mut so_push_stream, so_header) =
            SOStream::init_push(&SOKey::from_slice(key.as_slice()).expect("key failed"))
                .expect("init push failed");
        // Encrypt a series of messages
        let c1: Vec<u8> = so_push_stream
            .push(message1, None, SOTag::Message)
            .expect("Encrypt failed");
        let c2: Vec<u8> = so_push_stream
            .push(message2, None, SOTag::Message)
            .expect("Encrypt failed");
        let c3: Vec<u8> = so_push_stream
            .push(message3, None, SOTag::Final)
            .expect("Encrypt failed");

        // Initialize the pull side using header generated by the push side
        let mut pull_stream =
            DryocStream::init_pull(&key, &Header::try_from(so_header.as_ref()).expect("header"));

        // Decrypt the encrypted messages, type annotations required
        let (m1, tag1): (Vec<u8>, Tag) = pull_stream.pull(&c1, None).expect("Decrypt failed");
        let (m2, tag2): (Vec<u8>, Tag) = pull_stream.pull(&c2, None).expect("Decrypt failed");
        let (m3, tag3): (Vec<u8>, Tag) = pull_stream.pull(&c3, None).expect("Decrypt failed");

        assert_eq!(message1, m1.as_slice());
        assert_eq!(message2, m2.as_slice());
        assert_eq!(message3, m3.as_slice());

        assert_eq!(tag1, Tag::MESSAGE);
        assert_eq!(tag2, Tag::MESSAGE);
        assert_eq!(tag3, Tag::FINAL);
    }

    #[cfg(feature = "nightly")]
    #[test]
    fn test_protected_memory() {
        use crate::protected::*;

        let message1 = b"Arbitrary data to encrypt";
        let message2 = b"split into";
        let message3 = b"three messages";

        // Generate a random secret key for this stream
        let key = protected::Key::gen_locked().expect("gen locked");

        // Initialize the push side, type annotations required on return type
        let (mut push_stream, header): (_, Header) = DryocStream::init_push(&key);

        // Set secret key memory to no-access, but it must be unlocked first
        let key = key
            .munlock()
            .expect("munlock")
            .mprotect_noaccess()
            .expect("mprotect");

        // Encrypt a series of messages
        let c1: Locked<HeapBytes> = push_stream
            .push(message1, None, Tag::MESSAGE)
            .expect("Encrypt failed");
        let c2: Vec<u8> = push_stream
            .push(message2, None, Tag::MESSAGE)
            .expect("Encrypt failed");
        let c3: Vec<u8> = push_stream
            .push(message3, None, Tag::FINAL)
            .expect("Encrypt failed");

        // allow access again
        let key = key.mprotect_readonly().expect("mprotect");

        // Initialize the pull side using header generated by the push side
        let mut pull_stream = DryocStream::init_pull(&key, &header);

        // Set secret key memory to no-access
        let _key = key.mprotect_noaccess().expect("mprotect");

        // Decrypt the encrypted messages, type annotations required
        let (m1, tag1): (Locked<HeapBytes>, Tag) =
            pull_stream.pull(&c1, None).expect("Decrypt failed");
        let (m2, tag2): (Locked<HeapBytes>, Tag) =
            pull_stream.pull(&c2, None).expect("Decrypt failed");
        let (m3, tag3): (Locked<HeapBytes>, Tag) =
            pull_stream.pull(&c3, None).expect("Decrypt failed");

        assert_eq!(message1, m1.as_slice());
        assert_eq!(message2, m2.as_slice());
        assert_eq!(message3, m3.as_slice());

        assert_eq!(tag1, Tag::MESSAGE);
        assert_eq!(tag2, Tag::MESSAGE);
        assert_eq!(tag3, Tag::FINAL);
    }
}
