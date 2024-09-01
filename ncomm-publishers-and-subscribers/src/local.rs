//!
//! Local Publishers and Subscribers
//! 
//! Local Publishers and Subscribers utilize some combination of
//! primitives from the standard library to enable the sharing of
//! data between publishers and subscribers
//! 

use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
    time::{Duration, Instant}
};

use crossbeam::channel::{self, Sender, Receiver, SendError};

use ncomm_core::{Publisher, Subscriber};

/// Local Subscriber that utilizes a crossbeam multi subscriber channel
/// to receive data from a local publisher
pub struct LocalSubscriber<Data: Clone> {
    /// The receiver end of a crossbeam channel
    rx: Receiver<Data>,
    /// The current data stored in the local subscriber
    data: Option<Data>,
}

impl<Data: Clone> Subscriber for LocalSubscriber<Data> {
    type Target = Option<Data>;

    fn get(&mut self) -> &Self::Target {
        if let Some(data) = self.rx.try_iter().last() {
            self.data = Some(data);
        }

        &self.data
    }
}

/// Local subscriber where data has a specific time-to-live and will decay
/// after the lifetime has passed
pub struct LocalTTLSubscriber<Data: Clone> {
    /// The receiver end of a crossbeam channel
    rx: Receiver<Data>,
    /// The current data stored in the local subscriber
    data: Option<(Data, Instant)>,
    /// The time-to-live of a piece of data
    ttl: Duration,
}

impl<Data: Clone> Subscriber for LocalTTLSubscriber<Data> {
    type Target = Option<(Data, Instant)>;

    fn get(&mut self) -> &Self::Target {
        if let Some(data) = self.rx.try_iter().last() {
            self.data = Some((data, Instant::now()));
        }

        if self.data.is_some() && Instant::now().duration_since(self.data.as_ref().unwrap().1) > self.ttl {
            self.data = None;
        }

        &self.data
    }
}

/// Local subscriber that maps incoming data to into a location in a hashmap
/// allowing the subscriber to maintain a number of pieces of data at once.
pub struct LocalMappedSubscriber<Data: Clone, K: Eq + Hash> {
    /// The receiver end of a crossbeam channel
    rx: Receiver<Data>,
    /// The current data stored in the local hashmap
    data: HashMap<K, Data>,
    /// The hash function used to map incoming data into the hashmap
    hash: Arc<dyn Fn(&Data) -> K + Send + Sync>,
}

impl<Data: Clone, K: Eq + Hash> Subscriber for LocalMappedSubscriber<Data, K> {
    type Target = HashMap<K, Data>;

    fn get(&mut self) -> &Self::Target {
        for data in self.rx.try_iter() {
            let label = (self.hash)(&data);
            self.data.insert(label, data);
        }

        &self.data
    }
}

/// Local subscriber that maps incoming data to into a location in a hashmap
/// while specifying a time-to-live for pieces of data contained in the
/// hashmap
pub struct LocalMappedTTLSubscriber<Data: Clone, K: Eq + Hash> {
    /// The receiver end of a crossbeam channel
    rx: Receiver<Data>,
    /// The current data stored in a hashmap
    data: HashMap<K, (Data, Instant)>,
    /// The hash function used to map incoming data into the hashmap
    hash: Arc<dyn Fn(&Data) -> K + Send + Sync>,
    /// The time-to-live of pieces of data in the hashmap
    ttl: Duration,
}

impl<Data: Clone, K: Eq + Hash> Subscriber for LocalMappedTTLSubscriber<Data, K> {
    type Target = HashMap<K, (Data, Instant)>;

    fn get(&mut self) -> &Self::Target {
        for data in self.rx.try_iter() {
            let label = (self.hash)(&data);
            self.data.insert(label, (data, Instant::now()));
        }

        let now = Instant::now();
        self.data.retain(|_, v| now.duration_since(v.1) <= self.ttl);

        &self.data
    }
}

/// Local Publisher that utilizes a crossbeam multi publisher multi
/// subscriber to send data
pub struct LocalPublisher<Data: Clone> {
    /// The transmit pipe that is used to send data to the subscriber
    txs: Arc<Mutex<Vec<Sender<Data>>>>,
    /// The most recent data sent over the tx pipes so new subscribers will
    /// automatically have the most recent data
    data: Arc<Mutex<Option<(Data, Instant)>>>,
}

impl<Data: Clone> LocalPublisher<Data> {
    /// Create a new local publisher
    pub fn new() -> Self {
        Self {
            txs: Arc::new(Mutex::new(Vec::new())),
            data: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a local subscriber
    pub fn subscribe(&mut self) -> LocalSubscriber<Data> {
        let mut txs = self.txs.lock().unwrap();
        let (tx, rx) = channel::unbounded();
        txs.push(tx);

        let data = match self.data.lock().unwrap().as_ref() {
            Some(data) => Some(data.0.clone()),
            None => None,
        };

        LocalSubscriber {
            rx,
            data,
        }
    }

    /// Create a local subscriber with a specific time-to-live of pieces of data
    pub fn subscribe_ttl(&mut self, timeout: Duration) -> LocalTTLSubscriber<Data> {
        let mut txs = self.txs.lock().unwrap();
        let (tx, rx) = channel::unbounded();
        txs.push(tx);

        let data = match self.data.lock().unwrap().as_ref() {
            Some(data) => if Instant::now().duration_since(data.1) > timeout { Some(data.clone()) } else { None },
            None => None,
        };

        LocalTTLSubscriber {
            rx,
            data,
            ttl: timeout,
        }
    }

    /// Create a local subscriber that uses a map function to map data to specific slots in a hashmap.
    /// 
    /// Note: This subscriber will only have access to them most recent piece of data so
    /// do not expect that data sent a long time ago will be present in this subscriber's data
    pub fn subscribe_mapped<K: Eq + Hash>(&mut self, map: Arc<dyn Fn(&Data) -> K + Send + Sync>) -> LocalMappedSubscriber<Data, K> {
        let mut txs = self.txs.lock().unwrap();
        let (tx, rx) = channel::unbounded();
        txs.push(tx);

        let mut hashmap = HashMap::new();
        if let Some(data) = self.data.lock().unwrap().as_ref() {
            let data = data.0.clone();
            let label = (map)(&data);
            hashmap.insert(label, data);
        }

        LocalMappedSubscriber {
            rx,
            data: hashmap,
            hash: map,
        }
    }

    /// Create a local subscriber that uses a map function to map data to specific slots
    /// in a hashmap
    /// 
    /// Note: This subscriber will only have access to the most recent piece of data so
    /// do not expect that data sent a long time ago will be present in this subscriber's data
    pub fn subscribe_mapped_ttl<K: Eq + Hash>(&mut self, map: Arc<dyn Fn(&Data) -> K + Send + Sync>, ttl: Duration) -> LocalMappedTTLSubscriber<Data, K> {
        let mut txs = self.txs.lock().unwrap();
        let (tx, rx) = channel::unbounded();
        txs.push(tx);

        let mut hashmap = HashMap::new();
        if let Some(data) = self.data.lock().unwrap().as_ref() {
            if Instant::now().duration_since(data.1) <= ttl {
                let data = data.clone();
                let label = (map)(&data.0);
                hashmap.insert(label, data);
            }
        }

        LocalMappedTTLSubscriber {
            rx,
            data: hashmap,
            hash: map,
            ttl,
        }
    }
}

impl<Data: Clone> Clone for LocalPublisher<Data> {
    fn clone(&self) -> Self {
        Self {
            txs: self.txs.clone(),
            data: self.data.clone(),
        }
    }
}

impl<Data: Clone> Publisher for LocalPublisher<Data> {
    type Data = Data;
    type Error = SendError<Data>;

    fn publish(&mut self, data: Self::Data) -> Result<(), Self::Error> {
        let txs = self.txs.lock().unwrap();
        for tx in txs.iter() {
            tx.send(data.clone())?;
        }
        let mut data_ref = self.data.lock().unwrap();
        *data_ref = Some((data, Instant::now()));
        Ok(())
    }
}