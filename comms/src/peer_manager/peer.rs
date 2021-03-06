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

use crate::{connection::net_address::net_addresses::NetAddresses, peer_manager::node_id::NodeId};
use bitflags::*;
use chrono::prelude::*;
use tari_crypto::keys::PublicKey;

// TODO reputation metric?

bitflags! {
    #[derive(Default)]
    pub struct PeerFlags: u8 {
        const BANNED = 0b00000001;
    }
}

#[derive(Debug)]
pub struct Peer<K: PublicKey> {
    pub public_key: K,
    pub node_id: NodeId,
    pub addresses: NetAddresses,
    pub flags: PeerFlags,
}

impl<K> Peer<K>
where K: PublicKey
{
    /// Constructs a new peer
    pub fn new(public_key: K, node_id: NodeId, addresses: NetAddresses, flags: PeerFlags) -> Peer<K> {
        Peer {
            public_key,
            node_id,
            addresses,
            flags,
        }
    }

    /// Provides that date time of the last successful interaction with the peer
    pub fn last_seen(&self) -> Option<DateTime<Utc>> {
        self.addresses.last_seen()
    }

    /// Returns the ban status of the peer
    pub fn is_banned(&self) -> bool {
        self.flags.contains(PeerFlags::BANNED)
    }

    /// Changes the ban flag bit of the peer
    pub fn set_banned(&mut self, ban_flag: bool) {
        self.flags.set(PeerFlags::BANNED, ban_flag);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        connection::{net_address::net_addresses::NetAddresses, NetAddress},
        peer_manager::node_id::NodeId,
    };
    use tari_crypto::{
        keys::{PublicKey, SecretKey},
        ristretto::{RistrettoPublicKey, RistrettoSecretKey},
    };

    #[test]
    fn test_is_and_set_banned() {
        let mut rng = rand::OsRng::new().unwrap();
        let sk = RistrettoSecretKey::random(&mut rng);
        let pk = RistrettoPublicKey::from_secret_key(&sk);
        let node_id = NodeId::from_key(&pk).unwrap();
        let addresses = NetAddresses::from("123.0.0.123:8000".parse::<NetAddress>().unwrap());
        let mut peer: Peer<RistrettoPublicKey> =
            Peer::<RistrettoPublicKey>::new(pk, node_id, addresses, PeerFlags::default());
        assert_eq!(peer.is_banned(), false);
        peer.set_banned(true);
        assert_eq!(peer.is_banned(), true);
        peer.set_banned(false);
        assert_eq!(peer.is_banned(), false);
    }
}
