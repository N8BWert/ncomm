//!
//! Udp Datagram Clients and Servers
//! 
//! Udp Clients and Servers utilize Udp Datagrams to send
//! requests from clients to servers and then to send responses
//! back from servers to clients
//! 

use std::{
    net::{SocketAddr, UdpSocket},
    marker::PhantomData,
    io::Error,
};

use ncomm_core::{Client, Server};
use ncomm_utils::packing::{Packable, PackingError};

/// An error with sending udp packets
pub enum UdpClientServerError<Data: Packable> {
    /// std::io::Error
    IOError(Error),
    /// An error with packing the data
    PackingError(PackingError),
    /// Request from an unknown client
    UnknownRequester(Data),
    /// The client you are sending data to is unknown
    UnknownClient,
}

/// A udp client that sends requests via a UdpSocket to a specific
/// IP and receives data via a bound UdpSocket
pub struct UdpClient<Req: Packable, Res: Packable> {
    /// The Udp Socket bound for transmitting requests and receiving responses
    socket: UdpSocket,
    /// The address to send data to
    address: SocketAddr,
    /// A PhantomData to bind the specific request and response type to the
    /// client
    phantom: PhantomData<(Req, Res)>,
}

impl<Req: Packable, Res: Packable> UdpClient<Req, Res> {
    /// Create a new Udp Client
    pub fn new(bind_address: SocketAddr, server_address: SocketAddr) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_address)?;
        socket.set_nonblocking(true)?;
        socket.connect(server_address)?;
        Ok(Self {
            socket,
            address: server_address,
            phantom: PhantomData,
        })
    }
}

impl<Req: Packable, Res: Packable> Client for UdpClient<Req, Res> {
    type Request = Req;
    type Response = Res;
    type Error = UdpClientServerError<Req>;

    fn send_request(&mut self, request: Self::Request) -> Result<(), Self::Error> {
        let mut buffer = vec![0u8; Req::len()];
        request.pack(&mut buffer).map_err(UdpClientServerError::PackingError)?;

        self.socket.send_to(&buffer, self.address).map_err(UdpClientServerError::IOError)?;
        Ok(()) 
    }

    /// Check the UDP socket for incoming Datagrams.
    /// 
    /// Note: Incoming data will be in the form:
    /// [request[0], request[1], ..., request[-1], response[0], response[1], ...]
    fn poll_for_responses(&mut self) -> Vec<Result<(Self::Request, Self::Response), Self::Error>> {
        let mut responses = Vec::new();

        let mut buffer = vec![0u8; Res::len() + Res::len()];
        loop {
            let (req, res) = match self.socket.recv(&mut buffer) {
                Ok(_received) => (
                    Req::unpack(&buffer[..Req::len()]),
                    Res::unpack(&buffer[Req::len()..]),
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

/// A udp server that receives requests via a Udp Socket and sends response
/// via a the same Udp Socket to given addresses
/// 
/// Notes:
///     * REQ_SIZE is the size of the request
///     * RES_SIZE is the total size of the response (i.e. the sum of the size of the
///         request and the size of the response to the request)
pub struct UdpServer<Req: Packable, Res: Packable, K: Eq + Clone> {
    /// The socket bound to by the UdpServer
    socket: UdpSocket,
    /// A Map between client identifiers and their addresses
    client_addresses: Vec<(K, SocketAddr)>,
    /// A holder for the request and response types
    _phantom: PhantomData<(Req, Res)>,
}

impl<Req: Packable, Res: Packable, K: Eq + Clone> UdpServer<Req, Res, K> {
    /// Create a new Udp Server
    pub fn new(bind_address: SocketAddr) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_address)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            client_addresses: Vec::new(),
            _phantom: PhantomData,
        })
    }
}

impl<Req: Packable, Res: Packable, K: Eq + Clone> Server for UdpServer<Req, Res, K> {
    type Request = Req;
    type Response = Res;
    type Key = K;
    type Error = UdpClientServerError<Req>;

    fn poll_for_requests(&mut self) -> Vec<Result<(Self::Key, Self::Request), Self::Error>> {
        let mut requests = Vec::new();

        let mut buffer = vec![0u8; Req::len()];
        loop {
            let address = match self.socket.recv_from(&mut buffer) {
                Ok((_request_size, address)) => address,
                Err(_) => break,
            };

            match Req::unpack(&buffer[..]) {
                Ok(data) => {
                    if let Some((k, _)) = self.client_addresses.iter().find(|v| v.1 == address) {
                        requests.push(Ok((k.clone(), data)));
                    } else {
                        requests.push(Err(UdpClientServerError::UnknownRequester(data)));
                    }
                },
                Err(err) => requests.push(Err(UdpClientServerError::PackingError(err))),
            }
        }

        requests
    }

    fn send_response(&mut self, client_key: Self::Key, request: Self::Request, response: Self::Response) -> Result<(), Self::Error> {
        if let Some((_, address)) = self.client_addresses.iter().find(|v| v.0 == client_key) {
            let mut buffer = vec![0u8; Req::len() + Res::len()];

            request.pack(&mut buffer[0..Req::len()]).map_err(UdpClientServerError::PackingError)?;
            response.pack(&mut buffer[Req::len()..]).map_err(UdpClientServerError::PackingError)?;

            self.socket.send_to(&buffer, address)
                .map_err(UdpClientServerError::IOError)?;
            Ok(())
        } else {
            return Err(UdpClientServerError::UnknownClient);
        }
    }
}