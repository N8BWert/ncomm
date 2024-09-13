//!
//! A Network Udp-Based Publisher and Subscriber
//!
//! The UDP Publisher sends data as a UDP Datagram to some UDP endpoint
//!

use std::{
    collections::HashMap,
    hash::Hash,
    io::Error,
    marker::PhantomData,
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

use ncomm_core::{Publisher, Subscriber};
use ncomm_utils::packing::{Packable, PackingError};

/// A UDP Publisher that publishes data in a way defined by the Packable
/// layout to a group of addresses
pub struct UdpPublisher<Data: Packable> {
    // the UdpSocket bound for transmission
    tx: UdpSocket,
    /// The addresses to send data along.
    ///
    /// Note: addresses is public to allow users to modify the addresses
    /// to publish to in a way that is specific to the implementation of
    /// ToSocketAddrs
    pub addresses: Vec<SocketAddr>,
    // A PhantomAddress to bind the specific type of data to send to the
    // publisher
    phantom: PhantomData<Data>,
}

impl<Data: Packable> UdpPublisher<Data> {
    /// Create a new UdpPublisher
    pub fn new(bind_address: SocketAddr, send_addresses: Vec<SocketAddr>) -> Result<Self, Error> {
        let tx = UdpSocket::bind(bind_address)?;
        tx.set_nonblocking(true)?;
        Ok(Self {
            tx,
            addresses: send_addresses,
            phantom: PhantomData,
        })
    }
}

/// An Error with publishing udp packets
#[derive(Debug)]
pub enum UdpPublishError {
    /// std::io::Error occurred
    IOError(Error),
    /// An error occurred wth packing the data
    PackingError(PackingError),
}

impl<Data: Packable> Publisher for UdpPublisher<Data> {
    type Data = Data;
    type Error = UdpPublishError;

    fn publish(&mut self, data: Self::Data) -> Result<(), Self::Error> {
        let mut packed_data = vec![0u8; Data::len()];
        data.pack(&mut packed_data)
            .map_err(UdpPublishError::PackingError)?;

        for address in self.addresses.iter() {
            self.tx
                .send_to(&packed_data, address)
                .map_err(UdpPublishError::IOError)?;
        }

        Ok(())
    }
}

/// A UDP Subscriber that is set to non-blocking and updates its internal data
/// reference whenever it is dereferenced
pub struct UdpSubscriber<Data: Packable> {
    /// The receiving UdpSocket
    rx: UdpSocket,
    /// The current data stored in the subscriber
    data: Option<Data>,
}

impl<Data: Packable> UdpSubscriber<Data> {
    /// Create a new UdpSubscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr) -> Result<Self, Error> {
        let rx = UdpSocket::bind(bind_address)?;
        rx.set_nonblocking(true)?;
        Ok(Self { rx, data: None })
    }
}

impl<Data: Packable> Subscriber for UdpSubscriber<Data> {
    type Target = Option<Data>;

    fn get(&mut self) -> &Self::Target {
        let mut data = None;

        let mut buffer = vec![0u8; Data::len()];
        loop {
            let temp = match self.rx.recv_from(&mut buffer) {
                Ok(_received) => Data::unpack(&buffer[..]),
                Err(_) => break,
            };
            buffer.iter_mut().for_each(|v| *v = 0);
            if let Ok(found_data) = temp {
                data = Some(found_data);
            }
        }

        if let Some(data) = data {
            self.data = Some(data);
        }

        &self.data
    }
}

/// A Udp Subscriber that stores incoming data into a clearable buffer
pub struct UdpBufferedSubscriber<Data: Packable> {
    /// The UdpSocket to receive data through
    rx: UdpSocket,
    /// The data buffer
    buffer: Vec<Data>,
}

impl<Data: Packable> UdpBufferedSubscriber<Data> {
    /// Create a new UdpBufferedSubscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr) -> Result<Self, Error> {
        let rx = UdpSocket::bind(bind_address)?;
        rx.set_nonblocking(true)?;
        Ok(Self {
            rx,
            buffer: Vec::new(),
        })
    }

    /// Clear the buffer contained by the UdpBufferedSubscriber
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl<Data: Packable> Subscriber for UdpBufferedSubscriber<Data> {
    type Target = Vec<Data>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = vec![0u8; Data::len()];
        loop {
            let temp = match self.rx.recv_from(&mut buffer) {
                Ok(_received) => Data::unpack(&buffer[..]),
                Err(_) => break,
            };
            buffer.iter_mut().for_each(|v| *v = 0);
            if let Ok(found_data) = temp {
                self.buffer.push(found_data);
            }
        }

        &self.buffer
    }
}

/// A UDP Subscriber that updates its internal data representation with the
/// most recent piece of data that expires after a specific time-to-live
pub struct UdpTTLSubscriber<Data: Packable> {
    /// The UdpSocket to receive data through
    rx: UdpSocket,
    /// The most recent data contained by the subscriber
    data: Option<(Data, Instant)>,
    /// The total time that data is alive for
    ttl: Duration,
}

impl<Data: Packable> UdpTTLSubscriber<Data> {
    /// Create a new subscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr, ttl: Duration) -> Result<Self, Error> {
        let rx = UdpSocket::bind(bind_address)?;
        rx.set_nonblocking(true)?;
        Ok(Self {
            rx,
            data: None,
            ttl,
        })
    }
}

impl<Data: Packable> Subscriber for UdpTTLSubscriber<Data> {
    type Target = Option<(Data, Instant)>;

    fn get(&mut self) -> &Self::Target {
        let mut data = None;

        let mut buffer = vec![0u8; Data::len()];
        loop {
            let temp = match self.rx.recv_from(&mut buffer) {
                Ok(_received) => Data::unpack(&buffer[..]),
                Err(_) => break,
            };
            buffer.iter_mut().for_each(|v| *v = 0);

            if let Ok(found_data) = temp {
                data = Some(found_data);
            }
        }

        if let Some(data) = data {
            self.data = Some((data, Instant::now()));
        }

        if self.data.is_some()
            && Instant::now().duration_since(self.data.as_ref().unwrap().1) > self.ttl
        {
            self.data = None;
        }

        &self.data
    }
}

/// A UDP Subscriber that maps incoming data into slots in a HashMap by a given
/// mapping method.
pub struct UdpMappedSubscriber<Data: Packable, K: Eq + Hash, F: Fn(&Data) -> K> {
    /// The UdpSocket to receive data through
    rx: UdpSocket,
    /// A hashmap containing the most recent data for a set of keys
    data: HashMap<K, Data>,
    /// A hash method used to create keys for data obtained via the UdpSocket
    hash: F,
}

impl<Data: Packable, K: Eq + Hash, F: Fn(&Data) -> K> UdpMappedSubscriber<Data, K, F> {
    /// Create a new subscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr, map: F) -> Result<Self, Error> {
        let rx = UdpSocket::bind(bind_address)?;
        rx.set_nonblocking(true)?;
        Ok(Self {
            rx,
            data: HashMap::new(),
            hash: map,
        })
    }
}

impl<Data: Packable, K: Eq + Hash, F: Fn(&Data) -> K> Subscriber
    for UdpMappedSubscriber<Data, K, F>
{
    type Target = HashMap<K, Data>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = vec![0u8; Data::len()];
        loop {
            let temp = match self.rx.recv_from(&mut buffer) {
                Ok(_received) => Data::unpack(&buffer[..]),
                Err(_) => break,
            };
            buffer.iter_mut().for_each(|v| *v = 0);
            if let Ok(found_data) = temp {
                let label = (self.hash)(&found_data);
                self.data.insert(label, found_data);
            }
        }

        &self.data
    }
}

/// A UDP Subscriber that maps incoming data into slots in a HashMap by a given
/// mapping method while only keeping data that satisfies a given time-to-live.
pub struct UdpMappedTTLSubscriber<Data: Packable, K: Eq + Hash, F: Fn(&Data) -> K> {
    /// The UdpSocket to receive data through
    rx: UdpSocket,
    /// A hashmap containing the most recent valid data for a set of keys
    data: HashMap<K, (Data, Instant)>,
    /// A hash method used to create keys for data obtained vai the UdpSocket
    hash: F,
    // The total time that data is alive for
    ttl: Duration,
}

impl<Data: Packable, K: Eq + Hash, F: Fn(&Data) -> K> UdpMappedTTLSubscriber<Data, K, F> {
    /// Create a new subscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr, ttl: Duration, map: F) -> Result<Self, Error> {
        let rx = UdpSocket::bind(bind_address)?;
        rx.set_nonblocking(true)?;
        Ok(Self {
            rx,
            data: HashMap::new(),
            hash: map,
            ttl,
        })
    }
}

impl<Data: Packable, K: Eq + Hash, F: Fn(&Data) -> K> Subscriber
    for UdpMappedTTLSubscriber<Data, K, F>
{
    type Target = HashMap<K, (Data, Instant)>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = vec![0u8; Data::len()];
        loop {
            let temp = match self.rx.recv_from(&mut buffer) {
                Ok(_received) => Data::unpack(&buffer[..]),
                Err(_) => break,
            };
            buffer.iter_mut().for_each(|v| *v = 0);
            if let Ok(found_data) = temp {
                let label = (self.hash)(&found_data);
                self.data.insert(label, (found_data, Instant::now()));
            }
        }

        let now = Instant::now();
        self.data.retain(|_, v| now.duration_since(v.1) <= self.ttl);

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
    fn test_publish_udp_subscriber() {
        let mut publisher = UdpPublisher::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8000)),
            vec![SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8001))],
        )
        .unwrap();

        let mut subscriber: UdpSubscriber<Data> =
            UdpSubscriber::new(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8001)))
                .unwrap();

        let data = Data::new();
        publisher.publish(data.clone()).unwrap();

        sleep(Duration::from_millis(50));
        assert_eq!(subscriber.get().unwrap(), data);
    }

    #[test]
    fn test_publish_buffered_subscriber() {
        let mut publisher = UdpPublisher::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8002)),
            vec![SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8003))],
        )
        .unwrap();

        let mut subscriber: UdpBufferedSubscriber<Data> = UdpBufferedSubscriber::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8003)),
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
        let mut publisher = UdpPublisher::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8004)),
            vec![
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8005)),
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8006)),
            ],
        )
        .unwrap();

        let mut short_subscriber: UdpTTLSubscriber<Data> = UdpTTLSubscriber::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8005)),
            Duration::from_nanos(1),
        )
        .unwrap();

        let mut long_subscriber: UdpTTLSubscriber<Data> = UdpTTLSubscriber::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8006)),
            Duration::from_secs(3),
        )
        .unwrap();

        let data = Data::new();
        publisher.publish(data.clone()).unwrap();

        sleep(Duration::from_millis(50));
        assert_eq!(*short_subscriber.get(), None);
        assert_eq!(long_subscriber.get().unwrap().0, data);
    }

    #[test]
    fn tests_publish_mapped_subscriber() {
        let mut publisher = UdpPublisher::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8007)),
            vec![SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8008))],
        )
        .unwrap();

        let mut subscriber = UdpMappedSubscriber::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8008)),
            |data: &Data| data.num,
        )
        .unwrap();

        let data = Data::new();
        publisher.publish(data.clone()).unwrap();

        sleep(Duration::from_millis(50));
        assert_eq!(*subscriber.get().get(&data.num).unwrap(), data);
    }

    #[test]
    fn test_publish_mapped_ttl_subscriber() {
        let mut publisher = UdpPublisher::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8009)),
            vec![
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8010)),
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8011)),
            ],
        )
        .unwrap();

        let mut short_subscriber = UdpMappedTTLSubscriber::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8010)),
            Duration::from_nanos(1),
            |data: &Data| data.num,
        )
        .unwrap();

        let mut long_subscriber = UdpMappedTTLSubscriber::new(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8011)),
            Duration::from_secs(3),
            |data: &Data| data.num,
        )
        .unwrap();

        let data = Data::new();
        publisher.publish(data.clone()).unwrap();

        sleep(Duration::from_millis(50));
        short_subscriber.get();
        long_subscriber.get();
        assert_eq!(short_subscriber.get().get(&data.num), None);
        assert_eq!(long_subscriber.get().get(&data.num).unwrap().0, data);
    }
}
