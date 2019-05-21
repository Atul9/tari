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

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    ops::Range,
};

use crate::{
    connection::{
        net_address::ip::SocketAddress,
        zmq::curve_keypair,
        ConnectionError,
        Context,
        CurveEncryption,
        Direction,
        InprocAddress,
        NetAddress,
        PeerConnection,
        PeerConnectionContextBuilder,
        PeerConnectionError,
    },
    peer_manager::{node_id::NodeId, peer::Peer},
    utils,
};

use super::{Result, ConnectionManagerError};

pub(super) struct ConnectionWrapper<T> {
    pub(super) connection: T,
    pub(super) address: NetAddress,
}

impl<T> ConnectionWrapper<T> {
    pub fn unwrap(self) -> T {
        self.connection
    }
}

impl<T> From<(T, NetAddress)> for ConnectionWrapper<T> {
    fn from((data, address): (T, NetAddress)) -> Self {
        Self { connection: data, address }
    }
}

#[derive(Default)]
pub struct ConnectionContainer<'n> {
    connections: HashMap<&'n NodeId, ConnectionWrapper<PeerConnection>>,
}

impl<'n> ConnectionContainer<'n> {
     pub fn get_connection(&self, node_id: &NodeId) -> Option<ConnectionWrapper<&PeerConnection>> {
        self.connections
            .get(node_id)
    }

    pub fn remove_connection(&mut self, node_id: &NodeId) -> Result<ConnectionWrapper<&PeerConnection>> {
        self.connections
            .remove(node_id)
            .ok_or(ConnectionManagerError::PeerConnectionNotFound)
    }
}

//#[cfg(test)]
//mod test {
//    use super::*;
//    use crate::{
//        connection::{net_address::net_addresses::NetAddresses, zmq::curve_keypair},
//        peer_manager::peer::PeerFlags,
//    };
//    use tari_crypto::{
//        keys::SecretKey,
//        ristretto::{RistrettoPublicKey, RistrettoSecretKey},
//    };
//
//    fn make_node_id() -> NodeId {
//        let secret_key = RistrettoSecretKey::random(&mut rand::OsRng::new().unwrap());
//        let public_key = RistrettoPublicKey::from_secret_key(&secret_key);
//        NodeId::from_key(&public_key).unwrap()
//    }
//
//    fn make_connection_manager_config(consumer_address: InprocAddress) -> ConnectionManagerConfig {
//        ConnectionManagerConfig {
//            host: "127.0.0.1".parse().unwrap(),
//            port_range: 10000..20000,
//            socks_proxy_address: None,
//            consumer_address,
//            max_connect_retries: 5,
//            max_message_size: 512 * 1024,
//        }
//    }
//
//    #[test]
//    fn inbound_connection() {
//        let context = Context::new();
//        let consumer_address = InprocAddress::random();
//
//        let (secret_key, public_key) = curve_keypair::generate().unwrap();
//        let node_id = make_node_id();
//
//        let mut manager = ConnectionContainer::new(&context, make_connection_manager_config(consumer_address));
//        manager.establish_inbound_connection(&node_id, secret_key).unwrap();
//    }
//}
