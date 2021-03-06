//  Copyright 2019 The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use crate::peer_manager::node_id::*;
use bitflags::*;
use derive_error::Error;
use rmp_serde;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use tari_crypto::keys::PublicKey;

/// Represents a single message frame.
pub type Frame = Vec<u8>;
/// Represents a collection of frames which make up a multipart message.
pub type FrameSet = Vec<Frame>;

#[derive(Error, Debug)]
pub enum MessageError {
    /// Multipart message is malformed
    MalformedMultipart,
    /// Failed to serialize message
    SerializeFailed,
    /// Failed to deserialize message
    DeserializeFailed,
    /// An error occurred serialising an object into binary
    BinarySerializeError,
    /// An error occurred deserialising binary data into an object
    BinaryDeserializeError,
}

bitflags! {
    #[derive(Deserialize, Serialize)]
    pub struct IdentityFlags: u8 {
        const ENCRYPTED = 0b00000001;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum NodeDestination<PubKey> {
    Unknown,
    PublicKey(PubKey),
    NodeId(NodeId),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MessageEnvelopeHeader<PubKey> {
    pub version: u8,
    pub source: PubKey,
    pub dest: NodeDestination<PubKey>,
    pub signature: Vec<u8>,
    pub flags: IdentityFlags,
}

impl<PubKey: PublicKey> MessageEnvelopeHeader<PubKey> {
    /// Construct a new MessageEnvelopeHeader from its member variables
    pub fn new(
        version: u8,
        source: PubKey,
        dest: NodeDestination<PubKey>,
        signature: Vec<u8>,
        flags: IdentityFlags,
    ) -> MessageEnvelopeHeader<PubKey>
    {
        MessageEnvelopeHeader {
            version,
            source,
            dest,
            signature,
            flags,
        }
    }

    /// Serialize a MessageEnvelopeHeader into a single frame
    pub fn to_frame(&self) -> Result<Frame, MessageError> {
        let mut buf: Vec<u8> = Vec::new();
        match self.serialize(&mut rmp_serde::Serializer::new(&mut buf)) {
            Ok(_) => Ok(buf.to_vec()),
            Err(_) => Err(MessageError::SerializeFailed),
        }
    }
}

impl<PubKey: PublicKey> TryFrom<Frame> for MessageEnvelopeHeader<PubKey> {
    type Error = MessageError;

    /// Returns a MessageEnvelopeHeader from a Frame
    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        let mut de = rmp_serde::Deserializer::new(frame.as_slice());
        match Deserialize::deserialize(&mut de) {
            Ok(message_envelope_header) => Ok(message_envelope_header),
            Err(_) => Err(MessageError::DeserializeFailed),
        }
    }
}

const FRAMES_PER_MESSAGE: usize = 3;

/// Represents a message which is about to go on or has just come off the wire.
#[derive(Deserialize, Serialize)]
pub struct MessageEnvelope {
    frames: FrameSet,
}

impl MessageEnvelope {
    /// Create a new MessageEnvelope from four frames
    pub fn new(version: Frame, header: Frame, body: Frame) -> Self {
        MessageEnvelope {
            frames: vec![version, header, body],
        }
    }

    /// Returns the frame that is expected to be version frame
    pub fn version(&self) -> &Frame {
        &self.frames[0]
    }

    /// Returns the frame that is expected to be header frame
    pub fn header(&self) -> &Frame {
        &self.frames[1]
    }

    /// Returns the frame that is expected to be body frame
    pub fn body(&self) -> &Frame {
        &self.frames[2]
    }

    /// Serialize a MessageEnvelope into a frame set
    pub fn to_frame_set(&self) -> Result<FrameSet, MessageError> {
        Ok(self.frames.clone())
    }
}

impl TryFrom<FrameSet> for MessageEnvelope {
    type Error = MessageError;

    /// Returns a MessageEnvelope from a FrameSet
    fn try_from(frames: FrameSet) -> Result<Self, Self::Error> {
        if frames.len() != FRAMES_PER_MESSAGE {
            return Err(MessageError::MalformedMultipart);
        }

        Ok(MessageEnvelope { frames })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand;
    use rmp_serde;
    use serde::{Deserialize, Serialize};
    use tari_crypto::{
        keys::SecretKey,
        ristretto::{RistrettoPublicKey, RistrettoSecretKey},
    };

    use std::convert::TryInto;
    #[test]
    fn try_from_valid() {
        let example = vec![vec![0u8], vec![1u8], vec![2u8]];

        let raw_message: Result<MessageEnvelope, MessageError> = example.try_into();

        assert!(raw_message.is_ok());
        let raw_message = raw_message.unwrap();
        assert_eq!(raw_message.version(), &[0u8]);
        assert_eq!(raw_message.header(), &[1u8]);
        assert_eq!(raw_message.body(), &[2u8]);
    }

    #[test]
    fn try_from_invalid() {
        let example = vec![vec![0u8], vec![1u8]];

        let raw_message: Result<MessageEnvelope, MessageError> = example.try_into();

        assert!(raw_message.is_err());
        let error = raw_message.err().unwrap();
        match error {
            MessageError::MalformedMultipart => {},
            _ => panic!("Unexpected MessageError {:?}", error),
        }
    }

    #[test]
    fn test_ser_des() {
        let version = 0;
        let mut rng = rand::OsRng::new().unwrap();
        let k = RistrettoSecretKey::random(&mut rng);
        let p = RistrettoPublicKey::from_secret_key(&k);
        let source = p;
        let dest: NodeDestination<RistrettoPublicKey> = NodeDestination::Unknown;
        let signature = vec![0];
        let flags = IdentityFlags::ENCRYPTED;
        let header: MessageEnvelopeHeader<RistrettoPublicKey> = MessageEnvelopeHeader {
            version,
            source,
            dest,
            signature,
            flags,
        };

        let mut buf = Vec::new();
        header.serialize(&mut rmp_serde::Serializer::new(&mut buf)).unwrap();
        let serialized = buf.to_vec();
        let mut de = rmp_serde::Deserializer::new(serialized.as_slice());
        let deserialized: MessageEnvelopeHeader<RistrettoPublicKey> = Deserialize::deserialize(&mut de).unwrap();
        assert_eq!(deserialized, header);
    }
}
