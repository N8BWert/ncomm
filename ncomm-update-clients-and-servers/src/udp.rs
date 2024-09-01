//!
//! Udp Datagram Update Clients and Servers
//! 
//! Udp Update Clients and Servers utilize Udp Datagrams to send requests
//! to clients for longrunning operations.  The servers are then responsible
//! for sending back periodic updates on the status of requests before
//! eventually sending responses back to the client.
//! 

use std::{
    net::{SocketAddr, UdpSocket},
    marker::PhantomData,
    io::Error,
};

use packed_struct::{
    PackedStruct,
    PackedStructSlice,
    PackingError,
    types::bits::ByteArray,
};

use ncomm_core::{UpdateClient, UpdateServer};

/// An error with sending udp packets
pub enum UdpUpdateClientServerError<Data: PackedStruct> {
    /// std::io::Error
    IOError(Error),
    /// An error with packing the data
    PackingError(PackingError),
    /// Request from an unknown client
    UnknownRequester((Data, SocketAddr)),
    /// The client you are sending data to is unknown
    UnknownClient,
}

/// A Udp update client that sends request via a UdpSocket to a specific IP, receives
/// periodic updates, and finally receives a response via a bound UdpSocket
pub struct UdpUpdateClient<Req: PackedStruct, Updt: PackedStruct, Res: PackedStruct> {
    /// The Udp Socket bound for transmitting requests and receiving responses
    socket: UdpSocket,
    /// The address to send data to
    address: SocketAddr,
    /// A PhantomData to bind the specific request, update, and response types to
    /// the update client
    _phantom: PhantomData<(Req, Updt, Res)>,
}

impl<Req: PackedStruct, Updt: PackedStruct, Res: PackedStruct,> UdpUpdateClient<Req, Updt, Res> {
    /// Create a new Udp Update Client
    pub fn new(bind_address: SocketAddr, server_address: SocketAddr) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_address)?;
        socket.set_nonblocking(true)?;
        socket.connect(server_address)?;
        Ok(Self {
            socket,
            address: server_address,
            _phantom: PhantomData,
        })
    }
}

impl<Req: PackedStruct, Updt: PackedStruct, Res: PackedStruct,> UpdateClient
    for UdpUpdateClient<Req, Updt, Res> {
    type Request = Req;
    type Update = Updt;
    type Response = Res;
    type Error = UdpUpdateClientServerError<Req>;

    fn send_request(&mut self, request: Self::Request) -> Result<(), Self::Error> {
        let packed_data = request.pack().map_err(UdpUpdateClientServerError::PackingError)?;
        let packed_data = packed_data.as_bytes_slice();

        self.socket.send_to(packed_data, self.address).map_err(UdpUpdateClientServerError::IOError)?;
        Ok(())
    }

    fn poll_for_updates(&mut self) -> Vec<Result<(Self::Request, Self::Update), Self::Error>> {
        let mut updates = Vec::new();

        let mut buffer = vec![0u8; Req::ByteArray::len() + Updt::ByteArray::len()];
        loop {
            let (req, updt) = match self.socket.recv(&mut buffer) {
                Ok(_received) => (
                    Req::unpack_from_slice(&buffer[..Req::ByteArray::len()]),
                    Updt::unpack_from_slice(&buffer[Req::ByteArray::len()..]),
                ),
                Err(_) => break,
            };

            if req.is_ok() && updt.is_ok() {
                updates.push(Ok((req.unwrap(), updt.unwrap())));
            }
        }

        updates
    }

    fn poll_for_responses(&mut self) -> Vec<Result<(Self::Request, Self::Response), Self::Error>> {
        let mut responses = Vec::new();

        let mut buffer = vec![0u8; Req::ByteArray::len() + Res::ByteArray::len()];
        loop {
            let (req, res) = match self.socket.recv(&mut buffer) {
                Ok(_received) => (
                    Req::unpack_from_slice(&buffer[..Req::ByteArray::len()]),
                    Res::unpack_from_slice(&buffer[Req::ByteArray::len()..]),
                ),
                Err(_) => break,
            };

            if req.is_ok() && res.is_ok() {
                responses.push(Ok((req.unwrap(), res.unwrap())));
            }
        }

        responses
    }
}

/// A Udp Update server that receives requests via a Udp Socket and sends updates and
/// responses via the same Udp Socket to a given client identifiable by K.
pub struct UdpUpdateServer<Req: PackedStruct + Clone, Updt: PackedStruct, Res: PackedStruct, K: Eq + Clone> {
    /// The socket bound to by the UdpUpdateServer
    socket: UdpSocket,
    /// A Map between client identifiers and their addresses
    client_addresses: Vec<(K, SocketAddr)>,
    /// Bind the specific request, update, and response type to the update server
    _phantom: PhantomData<(Req, Updt, Res)>,
}

impl<Req: PackedStruct + Clone, Updt: PackedStruct, Res: PackedStruct, K: Eq + Clone> UdpUpdateServer<Req, Updt, Res, K> {
    /// Create a new Udp Update Server
    pub fn new(bind_address: SocketAddr) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_address)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            client_addresses: Vec::new(),
            _phantom: PhantomData
        })
    }
}

impl<Req: PackedStruct + Clone, Updt: PackedStruct, Res: PackedStruct, K: Eq + Clone> UpdateServer for UdpUpdateServer<Req, Updt, Res, K> {
    type Request = Req;
    type Update = Updt;
    type Response = Res;
    type Key = K;
    type Error = UdpUpdateClientServerError<Req>;

    fn poll_for_requests(&mut self) -> Vec<Result<(Self::Key, Self::Request), Self::Error>> {
        let mut requests = Vec::new();

        let mut buffer = vec![0u8; Req::ByteArray::len()];
        loop {
            let address = match self.socket.recv_from(&mut buffer) {
                Ok((_request_size, address)) => address,
                Err(_) => break,
            };

            match Req::unpack_from_slice(&buffer[..]) {
                Ok(data) => {
                    if let Some((k, _)) = self.client_addresses.iter().find(|v| v.1 == address) {
                        requests.push(Ok((k.clone(), data)));
                    } else {
                        requests.push(Err(UdpUpdateClientServerError::UnknownRequester((data, address))));
                    }
                },
                Err(err) => requests.push(Err(UdpUpdateClientServerError::PackingError(err))),
            }
        }

        requests
    }

    fn send_update(&mut self, client_key: Self::Key, request: &Self::Request, update: Self::Update) -> Result<(), Self::Error> {
        if let Some((_, address)) = self.client_addresses.iter().find(|v| v.0 == client_key) {
            let mut buffer = vec![0u8; Req::ByteArray::len() + Updt::ByteArray::len()];

            let packed_request = request.clone().pack().map_err(UdpUpdateClientServerError::PackingError)?;
            let packed_request = packed_request.as_bytes_slice();
            buffer[..Req::ByteArray::len()].copy_from_slice(packed_request);

            let packed_update = update.pack().map_err(UdpUpdateClientServerError::PackingError)?;
            let packed_update = packed_update.as_bytes_slice();
            buffer[Req::ByteArray::len()..].copy_from_slice(packed_update);

            self.socket.send_to(&buffer, address).map_err(UdpUpdateClientServerError::IOError)?;
            Ok(())
        } else {
            Err(UdpUpdateClientServerError::UnknownClient)
        }
    }

    fn send_response(&mut self, client_key: Self::Key, request: Self::Request, response: Self::Response) -> Result<(), Self::Error> {
        if let Some((_, address)) = self.client_addresses.iter().find(|v| v.0 == client_key) {
            let mut buffer = vec![0u8; Req::ByteArray::len() + Res::ByteArray::len()];

            let packed_request = request.clone().pack().map_err(UdpUpdateClientServerError::PackingError)?;
            let packed_request = packed_request.as_bytes_slice();
            buffer[..Req::ByteArray::len()].copy_from_slice(packed_request);

            let packed_response = response.pack().map_err(UdpUpdateClientServerError::PackingError)?;
            let packed_response = packed_response.as_bytes_slice();
            buffer[Req::ByteArray::len()..].copy_from_slice(packed_response);

            self.socket.send_to(&buffer, address).map_err(UdpUpdateClientServerError::IOError)?;
            Ok(())
        } else {
            Err(UdpUpdateClientServerError::UnknownClient)
        }
    }

}
