//!
//! Udp Datagram Update Clients and Servers
//!
//! Udp Update Clients and Servers utilize Udp Datagrams to send requests
//! to clients for longrunning operations.  The servers are then responsible
//! for sending back periodic updates on the status of requests before
//! eventually sending responses back to the client.
//!

use std::{
    io::Error,
    marker::PhantomData,
    net::{SocketAddr, UdpSocket},
};

use ncomm_core::{UpdateClient, UpdateServer};
use ncomm_utils::packing::{Packable, PackingError};

/// An error with sending udp packets
#[derive(Debug)]
pub enum UdpUpdateClientServerError<Data: Packable> {
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
///
/// Note: If Update and Response Packets are the same length, there is a chance
/// that when polling for updates a response will be received and processed.
/// This is, obviously, suboptimal and I will fix this in a later version but
/// for now I'd like to get version 1.0 out.
pub struct UdpUpdateClient<Req: Packable, Updt: Packable, Res: Packable> {
    /// The Udp Socket bound for transmitting requests and receiving responses
    socket: UdpSocket,
    /// The address to send data to
    address: SocketAddr,
    /// A buffer to keep any updates received when polling for responses
    update_buffer: Vec<Result<(Req, Updt), UdpUpdateClientServerError<Req>>>,
    /// A buffer to keep any responses received when polling for updates
    response_buffer: Vec<Result<(Req, Res), UdpUpdateClientServerError<Req>>>,
    /// A PhantomData to bind the specific request, update, and response types to
    /// the update client
    _phantom: PhantomData<(Req, Updt, Res)>,
}

impl<Req: Packable, Updt: Packable, Res: Packable> UdpUpdateClient<Req, Updt, Res> {
    /// Create a new Udp Update Client
    pub fn new(bind_address: SocketAddr, server_address: SocketAddr) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_address)?;
        socket.set_nonblocking(true)?;
        socket.connect(server_address)?;
        Ok(Self {
            socket,
            address: server_address,
            update_buffer: Vec::new(),
            response_buffer: Vec::new(),
            _phantom: PhantomData,
        })
    }
}

impl<Req: Packable, Updt: Packable, Res: Packable> UpdateClient
    for UdpUpdateClient<Req, Updt, Res>
{
    type Request = Req;
    type Update = Updt;
    type Response = Res;
    type Error = UdpUpdateClientServerError<Req>;

    fn send_request(&mut self, request: Self::Request) -> Result<(), Self::Error> {
        let mut buffer = vec![0u8; Req::len()];
        request
            .pack(&mut buffer)
            .map_err(UdpUpdateClientServerError::PackingError)?;

        self.socket
            .send_to(&buffer, self.address)
            .map_err(UdpUpdateClientServerError::IOError)?;
        Ok(())
    }

    fn poll_for_update(&mut self) -> Result<Option<(Self::Request, Self::Update)>, Self::Error> {
        let mut buffer = vec![0u8; Req::len() + std::cmp::max(Updt::len(), Res::len())];
        loop {
            let (req, updt) = match self.socket.recv(&mut buffer) {
                Ok(received) => {
                    if received - Req::len() == Updt::len() {
                        (
                            Req::unpack(&buffer[..Req::len()]),
                            Updt::unpack(&buffer[Req::len()..]),
                        )
                    } else if received - Req::len() == Res::len() {
                        let (req, res) = (
                            Req::unpack(&buffer[..Req::len()]),
                            Res::unpack(&buffer[Req::len()..]),
                        );

                        if let (Ok(req), Ok(res)) = (req, res) {
                            self.response_buffer.push(Ok((req, res)));
                        }
                        buffer.iter_mut().for_each(|v| *v = 0);
                        continue;
                    } else {
                        break;
                    }
                }
                Err(_) => break,
            };

            if let (Ok(req), Ok(updt)) = (req, updt) {
                return Ok(Some((req, updt)));
            }
        }
        Ok(None)
    }

    fn poll_for_updates(&mut self) -> Vec<Result<(Self::Request, Self::Update), Self::Error>> {
        let mut updates = Vec::new();
        updates.append(&mut self.update_buffer);

        let mut buffer = vec![0u8; Req::len() + std::cmp::max(Updt::len(), Res::len())];
        loop {
            let (req, updt) = match self.socket.recv(&mut buffer) {
                Ok(received) => {
                    if received - Req::len() == Updt::len() {
                        (
                            Req::unpack(&buffer[..Req::len()]),
                            Updt::unpack(&buffer[Req::len()..]),
                        )
                    } else if received - Req::len() == Res::len() {
                        let (req, res) = (
                            Req::unpack(&buffer[..Req::len()]),
                            Res::unpack(&buffer[Req::len()..]),
                        );

                        if let (Ok(req), Ok(res)) = (req, res) {
                            self.response_buffer.push(Ok((req, res)));
                        }
                        buffer.iter_mut().for_each(|v| *v = 0);
                        continue;
                    } else {
                        (
                            Req::unpack(&buffer[..Req::len()]),
                            Err(PackingError::InvalidBufferSize),
                        )
                    }
                }
                Err(_) => break,
            };

            if let (Ok(req), Ok(updt)) = (req, updt) {
                updates.push(Ok((req, updt)));
            }
            buffer.iter_mut().for_each(|v| *v = 0);
        }

        updates
    }

    fn poll_for_response(
        &mut self,
    ) -> Result<Option<(Self::Request, Self::Response)>, Self::Error> {
        let mut buffer = vec![0u8; Req::len() + std::cmp::max(Updt::len(), Res::len())];
        loop {
            let (req, res) = match self.socket.recv(&mut buffer) {
                Ok(received) => {
                    if received - Req::len() == Updt::len() {
                        let (req, updt) = (
                            Req::unpack(&buffer[..Req::len()]),
                            Updt::unpack(&buffer[Req::len()..]),
                        );

                        if let (Ok(req), Ok(updt)) = (req, updt) {
                            self.update_buffer.push(Ok((req, updt)));
                        }

                        buffer.iter_mut().for_each(|v| *v = 0);
                        continue;
                    } else if received - Req::len() == Res::len() {
                        (
                            Req::unpack(&buffer[..Req::len()]),
                            Res::unpack(&buffer[Req::len()..]),
                        )
                    } else {
                        break;
                    }
                }
                Err(_) => break,
            };

            if let (Ok(req), Ok(res)) = (req, res) {
                return Ok(Some((req, res)));
            }
        }
        Ok(None)
    }

    fn poll_for_responses(&mut self) -> Vec<Result<(Self::Request, Self::Response), Self::Error>> {
        let mut responses = Vec::new();
        responses.append(&mut self.response_buffer);

        let mut buffer = vec![0u8; Req::len() + std::cmp::max(Updt::len(), Res::len())];
        loop {
            let (req, res) = match self.socket.recv(&mut buffer) {
                Ok(received) => {
                    if received - Req::len() == Updt::len() {
                        let (req, updt) = (
                            Req::unpack(&buffer[..Req::len()]),
                            Updt::unpack(&buffer[Req::len()..]),
                        );

                        if let (Ok(req), Ok(updt)) = (req, updt) {
                            self.update_buffer.push(Ok((req, updt)));
                        }
                        continue;
                    } else if received - Req::len() == Res::len() {
                        (
                            Req::unpack(&buffer[..Req::len()]),
                            Res::unpack(&buffer[Req::len()..]),
                        )
                    } else {
                        (
                            Req::unpack(&buffer[..Req::len()]),
                            Err(PackingError::InvalidBufferSize),
                        )
                    }
                }
                Err(_) => break,
            };

            if let (Ok(req), Ok(res)) = (req, res) {
                responses.push(Ok((req, res)));
            }
        }

        responses
    }
}

/// A Udp Update server that receives requests via a Udp Socket and sends updates and
/// responses via the same Udp Socket to a given client identifiable by K.
pub struct UdpUpdateServer<Req: Packable + Clone, Updt: Packable, Res: Packable, K: Eq + Clone> {
    /// The socket bound to by the UdpUpdateServer
    socket: UdpSocket,
    /// A Map between client identifiers and their addresses
    client_addresses: Vec<(K, SocketAddr)>,
    /// Bind the specific request, update, and response type to the update server
    _phantom: PhantomData<(Req, Updt, Res)>,
}

impl<Req: Packable + Clone, Updt: Packable, Res: Packable, K: Eq + Clone>
    UdpUpdateServer<Req, Updt, Res, K>
{
    /// Create a new Udp Update Server
    pub fn new(bind_address: SocketAddr) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_address)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            client_addresses: Vec::new(),
            _phantom: PhantomData,
        })
    }

    /// Create a new Udp Update Server with a list of clients and addresses
    pub fn new_with(
        bind_address: SocketAddr,
        clients: Vec<(K, SocketAddr)>,
    ) -> Result<Self, Error> {
        let socket = UdpSocket::bind(bind_address)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            client_addresses: clients,
            _phantom: PhantomData,
        })
    }

    /// Add a list of known clients and their socket addresses
    pub fn add_clients(&mut self, mut clients: Vec<(K, SocketAddr)>) {
        self.client_addresses.append(&mut clients);
    }
}

impl<Req: Packable + Clone, Updt: Packable, Res: Packable, K: Eq + Clone> UpdateServer
    for UdpUpdateServer<Req, Updt, Res, K>
{
    type Request = Req;
    type Update = Updt;
    type Response = Res;
    type Key = K;
    type Error = UdpUpdateClientServerError<Req>;

    fn poll_for_request(&mut self) -> Result<Option<(Self::Key, Self::Request)>, Self::Error> {
        let mut buffer = vec![0u8; Req::len()];
        let address = match self.socket.recv_from(&mut buffer) {
            Ok((_requset_size, address)) => address,
            Err(_) => return Ok(None),
        };

        match Req::unpack(&buffer[..]) {
            Ok(data) => {
                if let Some((k, _)) = self.client_addresses.iter().find(|v| v.1 == address) {
                    Ok(Some((k.clone(), data)))
                } else {
                    Err(UdpUpdateClientServerError::UnknownRequester((
                        data, address,
                    )))
                }
            }
            Err(err) => Err(UdpUpdateClientServerError::PackingError(err)),
        }
    }

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
                        requests.push(Err(UdpUpdateClientServerError::UnknownRequester((
                            data, address,
                        ))));
                    }
                }
                Err(err) => requests.push(Err(UdpUpdateClientServerError::PackingError(err))),
            }
        }

        requests
    }

    fn send_update(
        &mut self,
        client_key: Self::Key,
        request: &Self::Request,
        update: Self::Update,
    ) -> Result<(), Self::Error> {
        if let Some((_, address)) = self.client_addresses.iter().find(|v| v.0 == client_key) {
            let mut buffer = vec![0u8; Req::len() + Updt::len()];

            request
                .clone()
                .pack(&mut buffer[0..Req::len()])
                .map_err(UdpUpdateClientServerError::PackingError)?;
            update
                .pack(&mut buffer[Req::len()..])
                .map_err(UdpUpdateClientServerError::PackingError)?;

            self.socket
                .send_to(&buffer, address)
                .map_err(UdpUpdateClientServerError::IOError)?;
            Ok(())
        } else {
            Err(UdpUpdateClientServerError::UnknownClient)
        }
    }

    fn send_response(
        &mut self,
        client_key: Self::Key,
        request: Self::Request,
        response: Self::Response,
    ) -> Result<(), Self::Error> {
        if let Some((_, address)) = self.client_addresses.iter().find(|v| v.0 == client_key) {
            let mut buffer = vec![0u8; Req::len() + Res::len()];

            request
                .clone()
                .pack(&mut buffer[0..Req::len()])
                .map_err(UdpUpdateClientServerError::PackingError)?;
            response
                .pack(&mut buffer[Req::len()..])
                .map_err(UdpUpdateClientServerError::PackingError)?;

            self.socket
                .send_to(&buffer, address)
                .map_err(UdpUpdateClientServerError::IOError)?;
            Ok(())
        } else {
            Err(UdpUpdateClientServerError::UnknownClient)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        net::{Ipv4Addr, SocketAddrV4},
        thread::sleep,
        time::Duration,
    };

    use rand::random;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Request {
        num: u64,
    }

    impl Request {
        pub fn new() -> Self {
            Self { num: random() }
        }
    }

    impl Packable for Request {
        fn len() -> usize {
            8
        }

        fn pack(self, buffer: &mut [u8]) -> Result<(), PackingError> {
            if buffer.len() < 8 {
                Err(PackingError::InvalidBufferSize)
            } else {
                Ok(buffer[..8].copy_from_slice(&self.num.to_le_bytes()))
            }
        }

        fn unpack(data: &[u8]) -> Result<Self, PackingError> {
            if data.len() < 8 {
                Err(PackingError::InvalidBufferSize)
            } else {
                Ok(Self {
                    num: u64::from_le_bytes(data[..8].try_into().unwrap()),
                })
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Update {
        num: u128,
    }

    impl Update {
        pub fn new(request: Request) -> Self {
            Self {
                num: (request.num as u128).wrapping_mul(4),
            }
        }
    }

    impl Packable for Update {
        fn len() -> usize {
            16
        }

        fn pack(self, buffer: &mut [u8]) -> Result<(), PackingError> {
            if buffer.len() < 8 {
                Err(PackingError::InvalidBufferSize)
            } else {
                Ok(buffer[..16].copy_from_slice(&self.num.to_le_bytes()))
            }
        }

        fn unpack(data: &[u8]) -> Result<Self, PackingError> {
            if data.len() < 16 {
                Err(PackingError::InvalidBufferSize)
            } else {
                Ok(Self {
                    num: u128::from_le_bytes(data[..16].try_into().unwrap()),
                })
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Response {
        num: u64,
    }

    impl Response {
        pub fn new(request: Request) -> Self {
            Self {
                num: request.num.wrapping_mul(2),
            }
        }
    }

    impl Packable for Response {
        fn len() -> usize {
            8
        }

        fn pack(self, buffer: &mut [u8]) -> Result<(), PackingError> {
            if buffer.len() < 8 {
                Err(PackingError::InvalidBufferSize)
            } else {
                Ok(buffer[..8].copy_from_slice(&self.num.to_le_bytes()))
            }
        }

        fn unpack(data: &[u8]) -> Result<Self, PackingError> {
            if data.len() < 8 {
                Err(PackingError::InvalidBufferSize)
            } else {
                Ok(Self {
                    num: u64::from_le_bytes(data[..8].try_into().unwrap()),
                })
            }
        }
    }

    #[test]
    fn test_udp_update_client_server() {
        let mut server: UdpUpdateServer<Request, Update, Response, i32> =
            UdpUpdateServer::new_with(
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7000)),
                vec![
                    (
                        0,
                        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7001)),
                    ),
                    (
                        1,
                        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7002)),
                    ),
                ],
            )
            .unwrap();

        let mut client_one: UdpUpdateClient<Request, Update, Response> = UdpUpdateClient::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7001)),
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7000)),
        )
        .unwrap();

        let mut client_two: UdpUpdateClient<Request, Update, Response> = UdpUpdateClient::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7002)),
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7000)),
        )
        .unwrap();

        let client_one_request = Request::new();
        let client_one_update = Update::new(client_one_request.clone());
        let client_one_response = Response::new(client_one_request.clone());
        let client_two_request = Request::new();
        let client_two_update = Update::new(client_two_request.clone());
        let client_two_response = Response::new(client_two_request.clone());

        client_one.send_request(client_one_request).unwrap();
        client_two.send_request(client_two_request).unwrap();

        sleep(Duration::from_millis(50));

        for request in server.poll_for_requests() {
            if let Ok(request) = request {
                match request.0 {
                    0 => assert_eq!(request.1, client_one_request),
                    _ => assert_eq!(request.1, client_two_request),
                }
                server
                    .send_update(request.0, &request.1, Update::new(request.1.clone()))
                    .unwrap();
                server
                    .send_response(request.0, request.1, Response::new(request.1.clone()))
                    .unwrap();
            }
        }

        sleep(Duration::from_millis(50));

        let updates = client_one.poll_for_updates();
        assert_eq!(updates.len(), 1);
        for update in updates {
            assert!(update.is_ok());
            let update = update.unwrap();
            assert_eq!(update.0, client_one_request);
            assert_eq!(update.1, client_one_update);
        }
        let responses = client_one.poll_for_responses();
        assert_eq!(responses.len(), 1);
        for response in responses {
            assert!(response.is_ok());
            let response = response.unwrap();
            assert_eq!(response.0, client_one_request);
            assert_eq!(response.1, client_one_response);
        }

        let updates = client_two.poll_for_updates();
        assert_eq!(updates.len(), 1);
        for update in updates {
            assert!(update.is_ok());
            let update = update.unwrap();
            assert_eq!(update.0, client_two_request);
            assert_eq!(update.1, client_two_update);
        }
        let responses = client_two.poll_for_responses();
        assert_eq!(responses.len(), 1);
        for response in responses {
            assert!(response.is_ok());
            let response = response.unwrap();
            assert_eq!(response.0, client_two_request);
            assert_eq!(response.1, client_two_response);
        }
    }

    #[test]
    fn test_udp_update_client_server_singular_request() {
        let mut server: UdpUpdateServer<Request, Update, Response, i32> =
            UdpUpdateServer::new_with(
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7008)),
                vec![(
                    0,
                    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7009)),
                )],
            )
            .unwrap();

        let mut client_one: UdpUpdateClient<Request, Update, Response> = UdpUpdateClient::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7009)),
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 7008)),
        )
        .unwrap();

        let client_one_request = Request::new();
        let client_one_update = Update::new(client_one_request.clone());
        let client_one_response = Response::new(client_one_request.clone());

        client_one.send_request(client_one_request).unwrap();

        sleep(Duration::from_millis(50));

        if let Ok(Some((client, request))) = server.poll_for_request() {
            assert_eq!(request, client_one_request);
            server
                .send_update(client, &request, Update::new(request.clone()))
                .unwrap();
            server
                .send_response(client, request, Response::new(request.clone()))
                .unwrap();
        } else {
            assert!(false, "Expected a request");
        }

        sleep(Duration::from_millis(50));

        if let Ok(Some((request, update))) = client_one.poll_for_update() {
            assert_eq!(request, client_one_request);
            assert_eq!(update, client_one_update);
        } else {
            assert!(false, "Expected an update");
        }

        if let Ok(Some((request, response))) = client_one.poll_for_response() {
            assert_eq!(request, client_one_request);
            assert_eq!(response, client_one_response);
        } else {
            assert!(false, "Expected a response");
        }
    }
}
