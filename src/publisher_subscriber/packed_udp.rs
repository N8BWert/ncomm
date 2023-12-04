//!
//! A Network Udp-Based Publisher + Subscriber that utilizes PackedStruct for Byte Packing
//! 
//! The PackedUdpPublisher sends data as a UDP Datagram to some other PackedUdpSubscriber
//! 

use std::{net::UdpSocket, collections::HashMap, hash::Hash, sync::Arc};
use std::marker::PhantomData;

use packed_struct::{PackedStruct, PackedStructSlice};
use packed_struct::types::bits::ByteArray;

use crate::publisher_subscriber::{Publish, SubscribeRemote, Receive};

/// Packed Struct UDP Publisher Type
/// 
/// The Packed Udp Publisher has a group of subscribers that it will send a
/// specific data type packed into a packed udp datagram
pub struct PackedUdpPublisher<'a, Data: PackedStruct + Send + Clone> {
    tx: UdpSocket,
    addresses: Vec<&'a str>,
    phantom: PhantomData<Data>,
}

/// Packed Struct Udp Subscriber Type
/// 
/// The Packed Udp Subscriber subscribes to a publisher of some sort of packed data and
/// decodes the data into the packed struct type given by data.
pub struct PackedUdpSubscriber<Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> {
    rx: UdpSocket,
    pub data: Option<Data>,
}

/// Buffered Packed Struct Udp Subscriber Type
/// 
/// The Buffered Packed Struct UDP Subscriber stores incoming data into a buffer that can be cleared or
/// read
pub struct BufferedPackedUdpSubscriber<Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> {
    rx: UdpSocket,
    pub data: Vec<Data>,
}

/// Mapped Packed Struct Udp Subscriber Type
/// 
/// Sort incoming data based on its hash and stores into a data hashmap
pub struct MappedPackedUdpSubscriber<Data: PackedStruct + Send + Clone, K: Eq + Hash, const DATA_SIZE: usize> {
    rx: UdpSocket,
    pub data: HashMap<K, Data>,
    hash: Arc<dyn Fn(&Data) -> K>,
}

impl<'a, Data: PackedStruct + Send + Clone> PackedUdpPublisher<'a, Data> {
    /// Create a new PackedUdpPublisher with a socket bound to the bind address that
    /// will send to the addresses listed in the addresses
    pub fn new(bind_address: &'a str, addresses: Vec<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given bind address");
        socket.set_nonblocking(true).unwrap();

        Self {
            tx: socket,
            addresses,
            phantom: PhantomData,
        }
    }
}

impl<'a, Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> PackedUdpSubscriber<Data, DATA_SIZE> {
    /// Create a new PackedUdpSubscriber bound to the bind address.
    /// 
    /// The from_address field is optional and will make it so that the subscriber only listens
    /// to the given address.
    pub fn new(bind_address: &'a str, from_address: Option<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        if let Some(from_address) = from_address {
            socket.connect(from_address).expect("couldn't connect to the given address");
        }
        socket.set_nonblocking(true).unwrap();

        assert_eq!(<Data as PackedStruct>::ByteArray::len(), DATA_SIZE);

        Self {
            rx: socket,
            data: None,
        }
    }
}

impl<'a, Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> BufferedPackedUdpSubscriber<Data, DATA_SIZE> {
    /// Create a new BufferedPackedSubscriber bound to the bind address
    /// 
    /// The from_address field is optional, but if given it will make this subscriber ignore all communcations
    /// except the one from the given address
    pub fn new(bind_address: &'a str, from_address: Option<&'a str>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        if let Some(from_address) = from_address {
            socket.connect(from_address).expect("couldn't connect to the given address");
        }
        socket.set_nonblocking(true).unwrap();

        assert_eq!(<Data as PackedStruct>::ByteArray::len(), DATA_SIZE);

        Self {
            rx: socket,
            data: Vec::new(),
        }
    }
}

impl<'a, Data: PackedStruct + Send + Clone, K: Eq + Hash, const DATA_SIZE: usize> MappedPackedUdpSubscriber<Data, K, DATA_SIZE> {
    ///Create a new MappedPackedSubscriber bound to the bind address
    /// 
    /// The from_address field is optional, but if given it will make this subscriber ignore all communcations
    /// except the one from the given address
    pub fn new(bind_address: &'a str, from_address: Option<&'a str>, hash_function: Arc<dyn Fn(&Data) -> K + Send + Sync>) -> Self {
        let socket = UdpSocket::bind(bind_address).expect("couldn't bind to the given address");
        if let Some(from_address) = from_address {
            socket.connect(from_address).expect("couldn't connect to the given address");
        }
        socket.set_nonblocking(true).unwrap();

        assert_eq!(<Data as PackedStruct>::ByteArray::len(), DATA_SIZE);

        Self {
            rx: socket,
            data: HashMap::new(),
            hash: hash_function,
        }
    }
}

impl<'a, Data: PackedStruct + Send + Clone> Publish<Data> for PackedUdpPublisher<'a, Data> {
    fn send(&mut self, data: Data) {
        let packed_data = match data.pack() {
            Ok(bytes) => bytes,
            Err(err) => panic!("Unable to Pack Data to Send: {:?}", err),
        };

        let packed_data = packed_data.as_bytes_slice();

        for address in self.addresses.iter() {
            if let Err(err) = self.tx.send_to(&packed_data, address) {
                println!("Error on Sending: {}", err);
            }
        }
    }
}

impl<'a, Data: PackedStruct + Send + Clone> SubscribeRemote<'a> for PackedUdpPublisher<'a, Data> {
    fn add_subscriber(&mut self, address: &'a str) {
        self.addresses.push(address);
    }
}

impl<Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> Receive for PackedUdpSubscriber<Data, DATA_SIZE> {
    fn update_data(&mut self) {
        let mut data: Option<Data> = None;

        loop {
            let mut buf = [0u8; DATA_SIZE];
            let temp = match self.rx.recv(&mut buf) {
                Ok(_received) => Data::unpack_from_slice(&buf[..]),
                Err(_) => break,
            };
            if let Ok(found_data) = temp {
                data = Some(found_data);
            }
        }

        if let Some(data) = data {
            self.data = Some(data);
        }
    }
}

impl<Data: PackedStruct + Send + Clone, const DATA_SIZE: usize> Receive for BufferedPackedUdpSubscriber<Data, DATA_SIZE> {
    fn update_data(&mut self) {
        loop {
            let mut buf = [0u8; DATA_SIZE];
            let temp = match self.rx.recv(&mut buf) {
                Ok(_received) => Data::unpack_from_slice(&buf[..]),
                Err(_) => break,
            };

            if let Ok(found_data) = temp {
                self.data.push(found_data);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    use packed_struct::prelude::*;

    #[derive(PackedStruct, Clone, Copy, Debug, PartialEq)]
    #[packed_struct(bit_numbering="msb0")]
    pub struct TestData {
        #[packed_field(bits="0..=2")]
        tiny_int: Integer<u8, packed_bits::Bits::<3>>,
        #[packed_field(bits="3..=4", ty = "enum")]
        mode: SelfTestMode,
        #[packed_field(bits="7")]
        enabled: bool
    }

    #[derive(PrimitiveEnum_u8, Clone, Copy, Debug, PartialEq)]
    pub enum SelfTestMode {
        NormalMode = 0,
        PositiveSignSelfTest = 1,
        NegativeSignSelfTest = 2,
        DebugMode = 3,
    }

    #[test]
    // Test that a packed udp publisher and subscriber can be created
    fn test_create_packed_udp_publisher_and_subscriber() {
        let subscriber: PackedUdpSubscriber<TestData, 1> = PackedUdpSubscriber::new("127.0.0.1:10001", Some("127.0.0.1:10000"));
        let publisher: PackedUdpPublisher<TestData> = PackedUdpPublisher::new("127.0.0.1:10000", vec!["127.0.0.1:10001"]);

        assert_eq!(subscriber.rx.local_addr().unwrap(), SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 10001)));
        assert_eq!(subscriber.data, None);

        assert_eq!(publisher.tx.local_addr().unwrap(), SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 10000)));
        assert_eq!(String::from(publisher.addresses[0]), String::from("127.0.0.1:10001"));
    }

    #[test]
    // Test that a packed udp publisher and subscriber can send data
    fn test_send_data_packed_udp_publisher_and_subscriber() {
        let mut subscriber: PackedUdpSubscriber<TestData, 1> = PackedUdpSubscriber::new("127.0.0.1:10003", Some("127.0.0.1:10002"));
        let mut publisher: PackedUdpPublisher<TestData> = PackedUdpPublisher::new("127.0.0.1:10002", vec!["127.0.0.1:10003"]);

        publisher.send(TestData { tiny_int: 5.into(), mode: SelfTestMode::DebugMode, enabled: true });

        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.unwrap(), TestData { tiny_int: 5.into(), mode: SelfTestMode::DebugMode, enabled: true });
    }

    #[test]
    // Test that a packed udp publisher and subscriber can send multiple pieces of data
    fn test_send_many_packed_data_udp_publisher_and_subscriber() {
        let mut subscriber: PackedUdpSubscriber<TestData, 1> = PackedUdpSubscriber::new("127.0.0.1:10005", Some("127.0.0.1:10004"));
        let mut publisher: PackedUdpPublisher<TestData> = PackedUdpPublisher::new("127.0.0.1:10004", vec!["127.0.0.1:10005"]);

        for i in 0..=5u8 {
            publisher.send(TestData { tiny_int: i.into(), mode: SelfTestMode::DebugMode, enabled: true });
        }

        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.unwrap(), TestData { tiny_int: 5.into(), mode: SelfTestMode::DebugMode, enabled: true });
    }

    #[test]
    // Test that a buffered udp subscriber can be created
    fn test_create_buffered_udp_subscriber() {
        let subscriber: BufferedPackedUdpSubscriber<TestData, 1> = BufferedPackedUdpSubscriber::new("127.0.0.1:10007", Some("127.0.0.1:10006"));
        let publisher: PackedUdpPublisher<TestData> = PackedUdpPublisher::new("127.0.0.1:10006", vec!["127.0.0.1:10007"]);

        assert_eq!(subscriber.rx.local_addr().unwrap(), SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 10007)));
        assert_eq!(subscriber.data.len(), 0);

        assert_eq!(publisher.tx.local_addr().unwrap(), SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 10006)));
        assert_eq!(String::from(publisher.addresses[0]), String::from("127.0.0.1:10007"));
    }

    #[test]
    // Test that a udp publisher can send data to the buffered udp subscriber
    fn test_send_packed_data_buffered_udp_subscriber() {
        let mut subscriber: BufferedPackedUdpSubscriber<TestData, 1> = BufferedPackedUdpSubscriber::new("127.0.0.1:10009", Some("127.0.0.1:10008"));
        let mut publisher: PackedUdpPublisher<TestData> = PackedUdpPublisher::new("127.0.0.1:10008", vec!["127.0.0.1:10009"]);

        publisher.send(TestData { tiny_int: 5.into(), mode: SelfTestMode::DebugMode, enabled: true });

        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.len(), 1);
        assert_eq!(subscriber.data[0], TestData { tiny_int: 5.into(), mode: SelfTestMode::DebugMode, enabled: true });
    }

    #[test]
    // Test that a udp publisher can send many data to the buffered udp subscriber
    fn test_send_many_packed_data_buffered_udp_subscriber() {
        let mut subscriber: BufferedPackedUdpSubscriber<TestData, 1> = BufferedPackedUdpSubscriber::new("127.0.0.1:10011", Some("127.0.0.1:10010"));
        let mut publisher: PackedUdpPublisher<TestData> = PackedUdpPublisher::new("127.0.0.1:10010", vec!["127.0.0.1:10011"]);

        for i in 0..=5u8 {
            publisher.send(TestData { tiny_int: i.into(), mode: SelfTestMode::DebugMode, enabled: true });
        }

        thread::sleep(time::Duration::from_millis(10));

        subscriber.update_data();

        assert_eq!(subscriber.data.len(), 6);
        for i in 0..=5u8 {
            assert_eq!(subscriber.data[i as usize], TestData { tiny_int: i.into(), mode: SelfTestMode::DebugMode, enabled: true });
        }
    }

    #[test]
    fn test_mapped_packed_udp_subscriber() {
        let mut subscriber: MappedPackedUdpSubscriber<TestData, u8, 1> = MappedPackedUdpSubscriber::new("127.0.0.1:10012", Some("127.0.0.1:10013"), Arc::new(|data: &TestData| { *data.tiny_int }));
        let mut publisher: PackedUdpPublisher<TestData> = PackedUdpPublisher::new("127.0.0.1:10013", vec!["127.0.0.1:10012"]);

        for i in 0..=5u8 {
            publisher.send(TestData { tiny_int: i.into(), mode: SelfTestMode::DebugMode, enabled: true });
        }

        thread::sleep(time::Duration::from_millis(10));
        
        subscriber.update_data();

        for i in 0..=5u8 {
            assert_eq!(*subscriber.data.get(&i).unwrap(), TestData { tiny_int: i.into(), mode: SelfTestMode::DebugMode, enabled: true });
        }
    }
}