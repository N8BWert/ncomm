//!
//! A Network Udp-Based Publisher and Subscriber
//! 
//! The UDP Publisher sends data as a UDP Datagram to some UDP endpoint
//! 

use std::{
    collections::HashMap,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    marker::PhantomData,
    io::Error,
    time::{Instant, Duration},
    sync::Arc,
    hash::Hash,
};

use packed_struct::{
    PackedStruct,
    PackedStructSlice,
    PackingError,
    types::bits::ByteArray,
};

use ncomm_core::{Publisher, Subscriber};

/// A UDP Publisher that publishes data in a way defined by the PackedStruct
/// layout to a group of addresses
pub struct UdpPublisher<Data: PackedStruct, A: ToSocketAddrs> {
    // the UdpSocket bound for transmission
    tx: UdpSocket,
    /// The addresses to send data along.
    /// 
    /// Note: addresses is public to allow users to modify the addresses
    /// to publish to in a way that is specific to the implementation of
    /// ToSocketAddrs
    pub addresses: A,
    // A PhantomAddress to bind the specific type of data to send to the
    // publisher
    phantom: PhantomData<Data>,
}

impl<Data: PackedStruct, A: ToSocketAddrs> UdpPublisher<Data, A> {
    /// Create a new UdpPublisher
    pub fn new(bind_address: SocketAddr, send_addresses: A) -> Result<Self, Error> {
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
pub enum UdpPublishError {
    /// std::io::Error occurred
    IOError(Error),
    /// An error occurred wth packing the data
    PackingError(PackingError),
}

impl<Data: PackedStruct, A: ToSocketAddrs> Publisher for UdpPublisher<Data, A> {
    type Data = Data;
    type Error = UdpPublishError;

    fn publish(&mut self, data: Self::Data) -> Result<(), Self::Error> {
        let packed_data = data.pack().map_err(UdpPublishError::PackingError)?;
        let packed_data = packed_data.as_bytes_slice();

        self.tx.send_to(packed_data, &self.addresses)
            .map_err(UdpPublishError::IOError)?;
        
        Ok(())
    }
}

/// A UDP Subscriber that is set to non-blocking and updates its internal data
/// reference whenever it is dereferenced
pub struct UdpSubscriber<Data: PackedStruct, const DATA_SIZE: usize> {
    /// The receiving UdpSocket
    rx: UdpSocket,
    /// The current data stored in the subscriber
    data: Option<Data>,
}

impl<Data: PackedStruct, const DATA_SIZE: usize> UdpSubscriber<Data, DATA_SIZE> {
    /// Create a new UdpSubscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr) -> Result<Self, Error>  {
        assert!(Data::ByteArray::len() == DATA_SIZE, "DATA_SIZE must be the PackedStruct size of Data");
        let rx = UdpSocket::bind(bind_address)?;
        rx.set_nonblocking(true)?;
        Ok(Self {
            rx,
            data: None,
        })
    }
}

impl<Data: PackedStruct, const DATA_SIZE: usize> Subscriber for UdpSubscriber<Data, DATA_SIZE> {
    type Target = Option<Data>;

    fn get(&mut self) -> &Self::Target {
        let mut data = None;

        let mut buffer = [0u8; DATA_SIZE];
        loop {
            let temp = match self.rx.recv_from(&mut buffer) {
                Ok(_received) => Data::unpack_from_slice(&buffer[..]),
                Err(_) => break,
            };
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

/// A UDP Subscriber that updates its internal data representation with the
/// most recent piece of data that expires after a specific time-to-live
pub struct UdpTTLSubscriber<Data: PackedStruct, const DATA_SIZE: usize> {
    /// The UdpSocket to receive data through
    rx: UdpSocket,
    /// The most recent data contained by the subscriber
    data: Option<(Data, Instant)>,
    /// The total time that data is alive for
    ttl: Duration,
}

impl<Data: PackedStruct, const DATA_SIZE: usize> UdpTTLSubscriber<Data, DATA_SIZE> {
    /// Create a new subscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr, ttl: Duration) -> Result<Self, Error> {
        assert!(Data::ByteArray::len() == DATA_SIZE, "DATA_SIZE must be the PackedStruct size of Data");
        let rx = UdpSocket::bind(bind_address)?;
        rx.set_nonblocking(true)?;
        Ok(Self {
            rx,
            data: None,
            ttl,
        })
    }
}

impl<Data: PackedStruct, const DATA_SIZE: usize> Subscriber for UdpTTLSubscriber<Data, DATA_SIZE> {
    type Target = Option<(Data, Instant)>;

    fn get(&mut self) -> &Self::Target {
        let mut data = None;

        let mut buffer = [0u8; DATA_SIZE];
        loop {
            let temp = match self.rx.recv_from(&mut buffer) {
                Ok(_received) => Data::unpack_from_slice(&buffer[..]),
                Err(_) => break,
            };

            if let Ok(found_data) = temp {
                data = Some(found_data);
            }
        }

        if let Some(data) = data {
            self.data = Some((data, Instant::now()));
        }

        if self.data.is_some() && Instant::now().duration_since(self.data.as_ref().unwrap().1) > self.ttl {
            self.data = None;
        }

        &self.data
    }
}

/// A UDP Subscriber that maps incoming data into slots in a HashMap by a given
/// mapping method.
pub struct UdpMappedSubscriber<Data: PackedStruct, K: Eq + Hash, const DATA_SIZE: usize> {
    /// The UdpSocket to receive data through
    rx: UdpSocket,
    /// A hashmap containing the most recent data for a set of keys
    data: HashMap<K, Data>,
    /// A hash method used to create keys for data obtained via the UdpSocket
    hash: Arc<dyn Fn(&Data) -> K + Send + Sync>,
}

impl<Data: PackedStruct, K: Eq + Hash, const DATA_SIZE: usize> UdpMappedSubscriber<Data, K, DATA_SIZE> {
    /// Create a new subscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr, map: Arc<dyn Fn(&Data) -> K + Send + Sync>) -> Result<Self, Error> {
        assert!(Data::ByteArray::len() == DATA_SIZE, "DATA_SIZE must be the PackedStruct size of Data");
        let rx = UdpSocket::bind(bind_address)?;
        rx.set_nonblocking(true)?;
        Ok(Self {
            rx,
            data: HashMap::new(),
            hash: map,
        })
    }
}

impl<Data: PackedStruct, K: Eq + Hash, const DATA_SIZE: usize> Subscriber for UdpMappedSubscriber<Data, K, DATA_SIZE> {
    type Target = HashMap<K, Data>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = [0u8; DATA_SIZE];
        loop {
            let temp = match self.rx.recv_from(&mut buffer) {
                Ok(_received) => Data::unpack_from_slice(&buffer[..]),
                Err(_) => break,
            };
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
pub struct UdpMappedTTLSubscriber<Data: PackedStruct, K: Eq + Hash, const DATA_SIZE: usize> {
    /// The UdpSocket to receive data through
    rx: UdpSocket,
    /// A hashmap containing the most recent valid data for a set of keys
    data: HashMap<K, (Data, Instant)>,
    /// A hash method used to create keys for data obtained vai the UdpSocket
    hash: Arc<dyn Fn(&Data) -> K + Send + Sync>,
    // The total time that data is alive for
    ttl: Duration,
}

impl<Data: PackedStruct, K: Eq + Hash, const DATA_SIZE: usize> UdpMappedTTLSubscriber<Data, K, DATA_SIZE> {
    /// Create a new subscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr, ttl: Duration, map: Arc<dyn Fn(&Data) -> K + Send + Sync>) -> Result<Self, Error> {
        assert!(Data::ByteArray::len() == DATA_SIZE, "DATA_SIZE must be the PackedStruct size of Data");
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

impl<Data: PackedStruct, K: Eq + Hash, const DATA_SIZE: usize> Subscriber for UdpMappedTTLSubscriber<Data, K, DATA_SIZE> {
    type Target = HashMap<K, (Data, Instant)>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = [0u8; DATA_SIZE];
        let now = Instant::now();
        loop {
            let temp = match self.rx.recv_from(&mut buffer) {
                Ok(_received) => Data::unpack_from_slice(&buffer[..]),
                Err(_) => break,
            };
            if let Ok(found_data) = temp {
                let label = (self.hash)(&found_data);
                self.data.insert(label, (found_data, now));
            }
        }

        self.data.retain(|_, v| now.duration_since(v.1) <= self.ttl);

        &self.data
    }
}