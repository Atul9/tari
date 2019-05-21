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

use tari_crypto::keys::PublicKey;

use crate::{
    connection::{
        curve_keypair::{CurvePublicKey, CurveSecretKey},
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
use super::{error::ConnectionManagerError,
            container::ConnectionContainer};


pub struct ConnectionWrapper<T> {
    connection: T,
    address: NetAddress,
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

pub struct ConnectionManager<'c, 'n> {
    context: &'c Context,
    container: ConnectionContainer<'n>,
    config: ConnectionManagerConfig,
    port_allocations: Vec<u16>,
}

impl<'c, 'n> ConnectionManager<'c, 'n> {
    fn new(context: &'c Context, config: ConnectionManagerConfig) -> Self {
        Self {
            context,
            config,
            container: ConnectionContainer::default(),
            port_allocations: Vec::new(),
        }
    }

     pub fn get_connection(&mut self, node_id: &NodeId) -> Option<&PeerConnection> {
        self.container.get_connection(node_id)
            .filter(|wrapped| {
                 if wrapped.connection.is_active() {
                    true
                } else {
                     // Side-effect: release the port and remove the connection
                     // if the connection is no longer active
                    if let Some(port) = wrapped.address.maybe_port() {
                        self.release_port(port);
                    }
                    self.container.remove_connection(conn);
                    false
                }
            })
            .map(|wrapped| wrapped.connection)
    }

    pub fn establish_outbound_connection(
        &mut self,
        node_id: &'n NodeId,
        address: NetAddress,
        server_public_key: CurvePublicKey,
    ) -> Result<()>
    {
        let (secret_key, public_key) = curve_keypair::generate()?;

        let context = self
            .new_context_builder()
            .set_id(node_id)
            .set_direction(Direction::Outbound)
            .set_address(address.clone())
            .set_curve_encryption(CurveEncryption::Client {
                secret_key,
                public_key,
                server_public_key,
            })
            .build()?;

        let connection = PeerConnection::new();
        connection.start(context)?;

        self.connections.insert(&node_id, (connection.clone(), address).into());

        Ok(())
    }

    pub fn establish_inbound_connection(&mut self, node_id: &'n NodeId, secret_key: CurveSecretKey) -> Result<()> {
        let address = self
            .allocate_net_address()
            .ok_or(ConnectionManagerError::NoAvailablePeerConnectionPort)?;

        let context = self
            .new_context_builder()
            .set_id(node_id)
            .set_direction(Direction::Inbound)
            .set_address(address.clone())
            .set_curve_encryption(CurveEncryption::Server { secret_key })
            .build()?;

        let connection = PeerConnection::new();
        connection.start(context)?;

        self.connections.insert(&node_id, (connection.clone(), address).into());

        Ok(())
    }

    pub fn drop_connection(&mut self, node_id: &NodeId) -> Result<()> {
        let wrapper = self
            .connections
            .get(node_id)
            .ok_or(ConnectionManagerError::PeerConnectionNotFound)?;

        if let Some(port) = wrapper.address.maybe_port() {
            self.release_port(port);
        }

        self.connections
            .remove(node_id)
            .ok_or(ConnectionManagerError::PeerConnectionNotFound)?;

        Ok(())
    }

    fn release_port(&mut self, port: u16) -> Option<u16> {
        self.port_allocations
            .iter()
            .position(|p| *p == port)
            .map(|idx| self.port_allocations.remove(idx))
    }


    fn new_context_builder(&self) -> PeerConnectionContextBuilder {
        let config = &self.config;

        let mut builder = PeerConnectionContextBuilder::new()
            .set_context(self.context)
            .set_max_msg_size(config.max_message_size)
            .set_consumer_address(config.consumer_address.clone())
            .set_max_retry_attempts(config.max_connect_retries);

        if let Some(ref addr) = config.socks_proxy_address {
            builder = builder.set_socks_proxy(addr.clone());
        }

        builder
    }

    fn allocate_net_address(&mut self) -> Option<NetAddress> {
        let config = &self.config;
        let (port_start, port_end) = (config.port_range.start, config.port_range.end);
        let num_available_ports = (port_end - port_start) - self.port_allocations.len() as u16;

        if num_available_ports == 0 {
            return None;
        }

        for port in config.port_range.clone() {
            if !self.port_allocations.contains(&port) {
                let address: SocketAddress = (config.host, port).into();
                if utils::is_address_available(&address) {
                    self.port_allocations.push(port);
                    return Some(address.into());
                }
            }
        }
        None
    }
}

pub struct ConnectionManagerConfig {
    max_message_size: u64,
    max_connect_retries: u16,
    socks_proxy_address: Option<SocketAddress>,
    consumer_address: InprocAddress,
    port_range: Range<u16>,
    host: IpAddr,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        connection::{net_address::net_addresses::NetAddresses, zmq::curve_keypair},
        peer_manager::peer::PeerFlags,
    };
    use tari_crypto::{
        keys::SecretKey,
        ristretto::{RistrettoPublicKey, RistrettoSecretKey},
    };

    fn make_node_id() -> NodeId {
        let secret_key = RistrettoSecretKey::random(&mut rand::OsRng::new().unwrap());
        let public_key = RistrettoPublicKey::from_secret_key(&secret_key);
        NodeId::from_key(&public_key).unwrap()
    }

    fn make_connection_manager_config(consumer_address: InprocAddress) -> ConnectionManagerConfig {
        ConnectionManagerConfig {
            host: "127.0.0.1".parse().unwrap(),
            port_range: 10000..20000,
            socks_proxy_address: None,
            consumer_address,
            max_connect_retries: 5,
            max_message_size: 512 * 1024,
        }
    }

    #[test]
    fn inbound_connection() {
        let context = Context::new();
        let consumer_address = InprocAddress::random();

        let (secret_key, public_key) = curve_keypair::generate().unwrap();
        let node_id = make_node_id();

        let mut manager = ConnectionManager::new(&context, make_connection_manager_config(consumer_address));
        manager.establish_inbound_connection(&node_id, secret_key).unwrap();
    }
}
