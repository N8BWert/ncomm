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

use ncomm_core::{Publisher, Subscriber};
use ncomm_utils::packing::{Packable, PackingError};

/// A UDP Publisher that publishes data in a way defined by the Packable
/// layout to a group of addresses
pub struct UdpPublisher<Data: Packable, A: ToSocketAddrs> {
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

impl<Data: Packable, A: ToSocketAddrs> UdpPublisher<Data, A> {
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

impl<Data: Packable, A: ToSocketAddrs> Publisher for UdpPublisher<Data, A> {
    type Data = Data;
    type Error = UdpPublishError;

    fn publish(&mut self, data: Self::Data) -> Result<(), Self::Error> {
        let mut packed_data = vec![0u8; Data::len()];
        data.pack(&mut packed_data).map_err(UdpPublishError::PackingError)?;

        self.tx.send_to(&packed_data, &self.addresses)
            .map_err(UdpPublishError::IOError)?;
        
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
    pub fn new(bind_address: SocketAddr) -> Result<Self, Error>  {
        let rx = UdpSocket::bind(bind_address)?;
        rx.set_nonblocking(true)?;
        Ok(Self {
            rx,
            data: None,
        })
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
    buffer: Vec<Data>
}

impl<Data: Packable> UdpBufferedSubscriber<Data> {
    /// Create a new UdpBufferedSubscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr) -> Result<Self, Error>  {
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
pub struct UdpMappedSubscriber<Data: Packable, K: Eq + Hash> {
    /// The UdpSocket to receive data through
    rx: UdpSocket,
    /// A hashmap containing the most recent data for a set of keys
    data: HashMap<K, Data>,
    /// A hash method used to create keys for data obtained via the UdpSocket
    hash: Arc<dyn Fn(&Data) -> K + Send + Sync>,
}

impl<Data: Packable, K: Eq + Hash> UdpMappedSubscriber<Data, K> {
    /// Create a new subscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr, map: Arc<dyn Fn(&Data) -> K + Send + Sync>) -> Result<Self, Error> {
        let rx = UdpSocket::bind(bind_address)?;
        rx.set_nonblocking(true)?;
        Ok(Self {
            rx,
            data: HashMap::new(),
            hash: map,
        })
    }
}

impl<Data: Packable, K: Eq + Hash> Subscriber for UdpMappedSubscriber<Data, K> {
    type Target = HashMap<K, Data>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = vec![0u8; Data::len()];
        loop {
            let temp = match self.rx.recv_from(&mut buffer) {
                Ok(_received) => Data::unpack(&buffer[..]),
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
pub struct UdpMappedTTLSubscriber<Data: Packable, K: Eq + Hash> {
    /// The UdpSocket to receive data through
    rx: UdpSocket,
    /// A hashmap containing the most recent valid data for a set of keys
    data: HashMap<K, (Data, Instant)>,
    /// A hash method used to create keys for data obtained vai the UdpSocket
    hash: Arc<dyn Fn(&Data) -> K + Send + Sync>,
    // The total time that data is alive for
    ttl: Duration,
}

impl<Data: Packable, K: Eq + Hash> UdpMappedTTLSubscriber<Data, K> {
    /// Create a new subscriber bound to a specific bind address
    pub fn new(bind_address: SocketAddr, ttl: Duration, map: Arc<dyn Fn(&Data) -> K + Send + Sync>) -> Result<Self, Error> {
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

impl<Data: Packable, K: Eq + Hash> Subscriber for UdpMappedTTLSubscriber<Data, K> {
    type Target = HashMap<K, (Data, Instant)>;

    fn get(&mut self) -> &Self::Target {
        let mut buffer = vec![0u8; Data::len()];
        let now = Instant::now();
        loop {
            let temp = match self.rx.recv_from(&mut buffer) {
                Ok(_received) => Data::unpack(&buffer[..]),
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