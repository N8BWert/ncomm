//!
//! A Network Tcp-Based Publisher and Subscriber
//!
//! The TCP Publisher sends data via a TCP Stream to a bound
//! listener on the Subscriber end
//!

use std::{
    collections::HashMap,
    io::{Error, Read, Write},
    marker::PhantomData,
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    time::{Duration, Instant},
};

use ncomm_core::{Publisher, Subscriber};
use ncomm_utils::packing::{Packable, PackingError};

/// An Error when attempting to publish data over a Tcp Publisher
#[derive(Debug)]
pub enum TcpPublishError {
    /// std::io::Error occurred (this can occur on multiple addresses)
    IOError(Vec<Error>),
    /// An error occurred with packing the data
    PackingError(PackingError),
}

/// A Tcp Publisher that publishes data via packing the data
/// according to the data's Packable implementation
pub struct TcpPublisher<Data: Packable> {
    /// The list of addresses to publish to
    pub addresses: Vec<SocketAddr>,
    /// A marker to bind the specific type of data to send to
    /// he publisher
    phantom: PhantomData<Data>,
    /// The amount of time to block when sending data
    write_timeout: Option<Duration>,
}

impl<Data: Packable> TcpPublisher<Data> {
    /// Create a new TcpPublisher
    pub fn new(send_addresses: Vec<SocketAddr>, write_timeout: Option<Duration>) -> Self {
        Self {
            addresses: send_addresses,
            write_timeout,
            phantom: PhantomData,
        }
    }
}

impl<Data: Packable> Publisher for TcpPublisher<Data> {
    type Data = Data;
    type Error = TcpPublishError;

    fn publish(&mut self, data: Self::Data) -> Result<(), Self::Error> {
        let mut packed_data = vec![0u8; Data::len()];
        data.pack(&mut packed_data)
            .map_err(TcpPublishError::PackingError)?;

        let mut publish_errors = Vec::new();
        for address in self.addresses.iter() {
            match TcpStream::connect(address) {
                Ok(mut stream) => {
                    if let Err(err) = stream.set_write_timeout(self.write_timeout) {
                        publish_errors.push(err);
                    }

                    if let Err(err) = stream.write(&packed_data) {
                        publish_errors.push(err);
                    }
                }
                Err(err) => publish_errors.push(err),
            }
        }

        if publish_errors.is_empty() {
            Ok(())
        } else {
            Err(TcpPublishError::IOError(publish_errors))
        }
    }
}

/// A Tcp Subscriber that is set to nonblocking and and listens
/// to incoming data.  If data comes from an unknown IP address,
/// the subscriber will reject the incoming data.
pub struct TcpSubscriber<Data: Packable> {
    /// The optional list of whitelisted IPs to listen to
    pub whitelist: Option<Vec<IpAddr>>,
    /// The Tcp Listener for incoming data
    listener: TcpListener,
    /// The current data stored in the subscriber
    data: Option<Data>,
}

impl<Data: Packable> TcpSubscriber<Data> {
    /// Create a new TcpSubscriber bound to a specific address
    pub fn new(bind_address: SocketAddr) -> Result<Self, Error> {
        let listener = TcpListener::bind(bind_address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            whitelist: None,
            listener,
            data: None,
        })
    }

    /// Create a new TcpSubscriber bound to a specific address with
    /// a specific whitelist
    pub fn new_with_whitelist(
        bind_address: SocketAddr,
        whitelist: Vec<IpAddr>,
    ) -> Result<Self, Error> {
        let listener = TcpListener::bind(bind_address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            whitelist: Some(whitelist),
            listener,
            data: None,
        })
    }

    /// Add an address to the whitelist.
    ///
    /// Note: the whitelist is publicly accessible so this
    /// method is purely for convenience.
    pub fn add_address_to_whitelist(&mut self, address: IpAddr) {
        if let Some(whitelist) = self.whitelist.as_mut() {
            whitelist.push(address);
        } else {
            self.whitelist = Some(vec![address]);
        }
    }

    /// Remove an address from the whitelist.
    ///
    /// Note: the whitelist is publicly accessible so this
    /// method is purely for convenience.
    pub fn remove_address_from_whitelist(&mut self, address: IpAddr) -> Option<IpAddr> {
        if let Some(whitelist) = self.whitelist.as_mut() {
            whitelist
                .iter()
                .position(|v| v.eq(&address))
                .map(|idx| whitelist.remove(idx))
        } else {
            None
        }
    }
}

impl<Data: Packable> Subscriber for TcpSubscriber<Data> {
    type Target = Option<Data>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = vec![0u8; Data::len()];
        while let Ok((mut stream, socket_addr)) = self.listener.accept() {
            if let Some(whitelist) = self.whitelist.as_ref() {
                if !whitelist.contains(&socket_addr.ip()) {
                    continue;
                }
            }

            if stream.read(&mut buffer).is_ok() {
                let data = Data::unpack(&buffer).unwrap();
                self.data = Some(data);
            }
            buffer.iter_mut().for_each(|v| *v = 0);
        }

        &self.data
    }
}

/// A Tcp Subscriber that stores incoming data into a clearable buffer
pub struct TcpBufferedSubscriber<Data: Packable> {
    /// The list of whitelisted IPs to accept data from
    pub whitelist: Option<Vec<IpAddr>>,
    /// The Tcp Listener for incoming data
    listener: TcpListener,
    /// The data buffer
    buffer: Vec<Data>,
}

impl<Data: Packable> TcpBufferedSubscriber<Data> {
    /// Create a new TcpBufferedSubscriber bound to an address
    pub fn new(bind_address: SocketAddr) -> Result<Self, Error> {
        let listener = TcpListener::bind(bind_address)?;
        listener.set_nonblocking(true)?;

        Ok(Self {
            whitelist: None,
            listener,
            buffer: Vec::new(),
        })
    }

    /// Create a new TcpBufferedSubscriber bound to a given address with
    /// a specified whitelist
    pub fn new_with_whitelist(
        bind_address: SocketAddr,
        whitelist: Vec<IpAddr>,
    ) -> Result<Self, Error> {
        let listener = TcpListener::bind(bind_address)?;
        listener.set_nonblocking(true)?;

        Ok(Self {
            whitelist: Some(whitelist),
            listener,
            buffer: Vec::new(),
        })
    }

    /// Add an address to the whitelist.
    ///
    /// Note: The whitelist was intentionally made public so users can
    /// modify the whitelist themselves so this is just a convenience method.
    pub fn add_address_to_whitelist(&mut self, address: IpAddr) {
        if let Some(whitelist) = self.whitelist.as_mut() {
            whitelist.push(address);
        } else {
            self.whitelist = Some(vec![address]);
        }
    }

    /// Remove an address from the whitelist
    ///
    /// Note: The whitelist was intentionally made public so this
    /// method is more of a convenience
    pub fn remove_address_from_whitelist(&mut self, address: IpAddr) -> Option<IpAddr> {
        if let Some(whitelist) = self.whitelist.as_mut() {
            whitelist
                .iter()
                .position(|v| v.eq(&address))
                .map(|idx| whitelist.remove(idx))
        } else {
            None
        }
    }

    /// Clears the data stored in the TcpBufferedSubscriber's data buffer
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }
}

impl<Data: Packable> Subscriber for TcpBufferedSubscriber<Data> {
    type Target = Vec<Data>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = vec![0u8; Data::len()];
        while let Ok((mut stream, socket_addr)) = self.listener.accept() {
            if let Some(whitelist) = self.whitelist.as_ref() {
                if !whitelist.contains(&socket_addr.ip()) {
                    continue;
                }
            }

            if stream.read(&mut buffer).is_ok() {
                let data = Data::unpack(&buffer).unwrap();
                self.buffer.push(data);
            }
            buffer.iter_mut().for_each(|v| *v = 0);
        }

        &self.buffer
    }
}

/// A Tcp Subscriber that subscribes to a TCP stream keeping data
/// for specified time-to-live
///
/// Note: this is not the same as setting the ttl value on the TCP packet.
/// Instead, this is specifying that after the data has been received by the
/// subscriber, that piece of data is valid for a specific duration of time.
pub struct TcpTTLSubscriber<Data: Packable> {
    /// The list of whitelisted IPs to listen to
    pub whitelist: Option<Vec<IpAddr>>,
    /// The Tcp Listener for incoming data
    listener: TcpListener,
    /// The current data stored in the subscriber
    data: Option<(Data, Instant)>,
    /// The time-to-live of the packet
    ttl: Duration,
}

impl<Data: Packable> TcpTTLSubscriber<Data> {
    /// Create a new TcpTTLSubscriber bound to a specific address
    pub fn new(bind_address: SocketAddr, ttl: Duration) -> Result<Self, Error> {
        let listener = TcpListener::bind(bind_address)?;
        listener.set_nonblocking(true)?;

        Ok(Self {
            whitelist: None,
            listener,
            data: None,
            ttl,
        })
    }

    /// Create a new TcpTTLSubscriber bound to a specific address
    /// with a given whitelist
    pub fn new_with_whitelist(
        bind_address: SocketAddr,
        whitelist: Vec<IpAddr>,
        ttl: Duration,
    ) -> Result<Self, Error> {
        let listener = TcpListener::bind(bind_address)?;
        listener.set_nonblocking(true)?;

        Ok(Self {
            whitelist: Some(whitelist),
            listener,
            data: None,
            ttl,
        })
    }

    /// Add an address to the whitelist.
    ///
    /// Note: The whitelist was intentionally made public so users can
    /// modify the whitelist themselves so this is just a convenience method.
    pub fn add_address_to_whitelist(&mut self, address: IpAddr) {
        if let Some(whitelist) = self.whitelist.as_mut() {
            whitelist.push(address);
        } else {
            self.whitelist = Some(vec![address]);
        }
    }

    /// Remove an address from the whitelist
    ///
    /// Note: The whitelist was intentionally made public so this
    /// method is more of a convenience
    pub fn remove_address_from_whitelist(&mut self, address: IpAddr) -> Option<IpAddr> {
        if let Some(whitelist) = self.whitelist.as_mut() {
            whitelist
                .iter()
                .position(|v| v.eq(&address))
                .map(|idx| whitelist.remove(idx))
        } else {
            None
        }
    }
}

impl<Data: Packable> Subscriber for TcpTTLSubscriber<Data> {
    type Target = Option<(Data, Instant)>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = vec![0u8; Data::len()];
        while let Ok((mut stream, socket_addr)) = self.listener.accept() {
            if let Some(whitelist) = self.whitelist.as_ref() {
                if !whitelist.contains(&socket_addr.ip()) {
                    continue;
                }
            }

            if stream.read(&mut buffer).is_ok() {
                let data = Data::unpack(&buffer).unwrap();
                self.data = Some((data, Instant::now()));
            }
            buffer.iter_mut().for_each(|v| *v = 0);
        }

        if self.data.is_some()
            && Instant::now().duration_since(self.data.as_ref().unwrap().1) > self.ttl
        {
            self.data = None;
        }

        &self.data
    }
}

/// A Tcp Subscriber that maps incoming data to its IP address.
///
/// Note: this subscriber does not have a whitelist as all incoming valid
/// data will be stored in a hashmap by IP.
///
/// Note: All incoming data is either mapped by its peer address or its
/// local address.  Therefore, all data with a valid peer address will be
/// stored by peer address and any data without valid peer address will be
/// stored by local address.
pub struct TcpMappedSubscriber<Data: Packable> {
    /// the Tcp Listener for incoming data
    listener: TcpListener,
    /// The data currently stored in the subscriber
    data: HashMap<IpAddr, Data>,
}

impl<Data: Packable> TcpMappedSubscriber<Data> {
    /// Create a new TcpMappedSubscriber bound to a specific address
    pub fn new(bind_address: SocketAddr) -> Result<Self, Error> {
        let listener = TcpListener::bind(bind_address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            data: HashMap::new(),
        })
    }
}

impl<Data: Packable> Subscriber for TcpMappedSubscriber<Data> {
    type Target = HashMap<IpAddr, Data>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = vec![0u8; Data::len()];
        while let Ok((mut stream, socket_addr)) = self.listener.accept() {
            if stream.read(&mut buffer).is_ok() {
                let data = Data::unpack(&buffer).unwrap();
                self.data.insert(socket_addr.ip(), data);
            }
            buffer.iter_mut().for_each(|v| *v = 0);
        }

        &self.data
    }
}

/// A Tcp Subscriber that stores incoming data stored by IP Address with
/// a given time-to-live for stored data.
///
/// Note: In this case time-to-live means the amount of time after the
/// data has been received by the subscriber.
pub struct TcpMappedTTLSubscriber<Data: Packable> {
    /// The Tcp Listener for incoming data
    listener: TcpListener,
    /// The map containing the most recent piece of data per socket
    /// address
    data: HashMap<IpAddr, (Data, Instant)>,
    /// The amount of time data should live for
    ttl: Duration,
}

impl<Data: Packable> TcpMappedTTLSubscriber<Data> {
    /// Create a new TcpMappedSubscriber bound to a specific address
    pub fn new(bind_address: SocketAddr, ttl: Duration) -> Result<Self, Error> {
        let listener = TcpListener::bind(bind_address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            data: HashMap::new(),
            ttl,
        })
    }
}

impl<Data: Packable> Subscriber for TcpMappedTTLSubscriber<Data> {
    type Target = HashMap<IpAddr, (Data, Instant)>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = vec![0u8; Data::len()];
        while let Ok((mut stream, socket_addr)) = self.listener.accept() {
            if stream.read(&mut buffer).is_ok() {
                let data = Data::unpack(&buffer).unwrap();
                self.data.insert(socket_addr.ip(), (data, Instant::now()));
            }
            buffer.iter_mut().for_each(|v| *v = 0);
        }

        self.data
            .retain(|_, v| Instant::now().duration_since(v.1) <= self.ttl);

        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::random;
    use std::{
        net::{Ipv4Addr, SocketAddrV4},
        thread::sleep,
        time::Duration,
    };

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Data {
        num: u64,
    }

    impl Data {
        pub fn new() -> Self {
            Self { num: random() }
        }
    }

    impl Packable for Data {
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
    fn test_publish_tcp_subscriber() {
        let mut publisher = TcpPublisher::new(
            vec![SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6000))],
            None,
        );

        let mut subscriber: TcpSubscriber<Data> = TcpSubscriber::new_with_whitelist(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6000)),
            vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        )
        .unwrap();

        let data = Data::new();
        publisher.publish(data.clone()).unwrap();

        sleep(Duration::from_millis(50));
        assert_eq!(subscriber.get().unwrap(), data);
    }

    #[test]
    fn test_publish_buffered_subscriber() {
        let mut publisher = TcpPublisher::new(
            vec![SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6001))],
            None,
        );

        let mut subscriber: TcpBufferedSubscriber<Data> =
            TcpBufferedSubscriber::new_with_whitelist(
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6001)),
                vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
            )
            .unwrap();

        let datas = vec![Data::new(); 100];
        for data in datas.iter() {
            publisher.publish(data.clone()).unwrap();
        }

        sleep(Duration::from_millis(50));
        assert_eq!(*subscriber.get(), datas);
    }

    #[test]
    fn test_publish_ttl_subscriber() {
        let mut publisher = TcpPublisher::new(
            vec![
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6002)),
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6003)),
            ],
            None,
        );

        let mut short_subscriber: TcpTTLSubscriber<Data> = TcpTTLSubscriber::new_with_whitelist(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6002)),
            vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
            Duration::from_nanos(1),
        )
        .unwrap();

        let mut long_subscriber: TcpTTLSubscriber<Data> = TcpTTLSubscriber::new_with_whitelist(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6003)),
            vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
            Duration::from_secs(3),
        )
        .unwrap();

        let data = Data::new();
        publisher.publish(data.clone()).unwrap();

        sleep(Duration::from_millis(50));
        short_subscriber.get();
        long_subscriber.get();
        assert_eq!(*short_subscriber.get(), None);
        assert_eq!(long_subscriber.get().unwrap().0, data);
    }

    #[test]
    fn test_publish_mapped_subscriber() {
        let mut publisher = TcpPublisher::new(
            vec![SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6004))],
            None,
        );

        let mut subscriber: TcpMappedSubscriber<Data> =
            TcpMappedSubscriber::new(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6004)))
                .unwrap();

        let data = Data::new();
        publisher.publish(data.clone()).unwrap();

        sleep(Duration::from_millis(50));
        assert_eq!(
            *subscriber
                .get()
                .get(&IpAddr::V4(Ipv4Addr::LOCALHOST))
                .unwrap(),
            data
        );
    }

    #[test]
    fn test_publish_mapped_ttl_subscriber() {
        let mut publisher = TcpPublisher::new(
            vec![
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6005)),
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6006)),
            ],
            None,
        );

        let mut short_subscriber: TcpMappedTTLSubscriber<Data> = TcpMappedTTLSubscriber::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6005)),
            Duration::from_nanos(1),
        )
        .unwrap();

        let mut long_subscriber: TcpMappedTTLSubscriber<Data> = TcpMappedTTLSubscriber::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 6006)),
            Duration::from_secs(3),
        )
        .unwrap();

        let data = Data::new();
        publisher.publish(data.clone()).unwrap();

        sleep(Duration::from_millis(50));
        short_subscriber.get();
        long_subscriber.get();
        assert_eq!(
            short_subscriber.get().get(&IpAddr::V4(Ipv4Addr::LOCALHOST)),
            None
        );
        assert_eq!(
            long_subscriber
                .get()
                .get(&IpAddr::V4(Ipv4Addr::LOCALHOST))
                .unwrap()
                .0,
            data
        );
    }
}
