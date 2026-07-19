use std::hash::Hash;

use dashmap::{DashMap, Entry};
use itertools::Either;
use tokio::sync::{mpsc, oneshot};
use tracing::debug;

pub struct SharedQueue<K, V> {
    queues: DashMap<K, mpsc::UnboundedSender<oneshot::Sender<V>>>,
}

impl<K, V> SharedQueue<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            queues: DashMap::new(),
        }
    }

    fn try_insert_queue(
        &self,
        key: K,
    ) -> Either<mpsc::UnboundedReceiver<oneshot::Sender<V>>, oneshot::Receiver<V>> {
        let mut occupied_entry = match self.queues.entry(key) {
            Entry::Occupied(occupied) => occupied,
            Entry::Vacant(vacant) => {
                let (sender, receiver) = mpsc::unbounded_channel();
                vacant.insert(sender);
                return Either::Left(receiver);
            }
        };

        let (sender, receiver) = oneshot::channel();
        match occupied_entry.get().send(sender) {
            Ok(()) => Either::Right(receiver), // Successfully sent the sender to the existing queue
            Err(_) => {
                // The queue was dropped, so we can create a new one
                let (sender, receiver) = mpsc::unbounded_channel();
                occupied_entry.insert(sender);
                Either::Left(receiver)
            }
        }
    }

    /// Runs a task that produces a value of type `V` for a given key `K`, ensuring that only one
    /// task is running for each key at a time.
    ///
    /// If multiple requests for the same key are made while the task is running, they will be
    /// queued and receive the same result when the task completes.
    pub async fn run_shared<F, Fut, Err, FnDup>(
        &self,
        key: K,
        task: F,
        mut create_duplicates: FnDup,
    ) -> Result<V, Err>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, Err>> + Send + Sync,
        FnDup: FnMut(&V) -> V,
    {
        let mut reciever = loop {
            let one_shot_receiver = match self.try_insert_queue(key.clone()) {
                Either::Left(receiver) => break receiver, // We are the first request, we need to run the task
                Either::Right(receiver) => receiver,
            };

            debug!("Another request is already waiting, joining the queue");
            if let Ok(response) = one_shot_receiver.await {
                return Ok(response); // We received a response from the existing queue, return it
            }
            debug!("Waiting request was dropped, retrying");
        };

        debug!("We are the first request, running the task");
        let result = task().await;
        self.queues.remove(&key); // Remove the queue from the map
        let ok_result = result?;

        debug!("Task completed, sending response to waiting requests");
        while let Ok(sender) = reciever.try_recv() {
            // Send the result to the waiting request, if the sender is dropped, we can ignore it
            let _ = sender.send(create_duplicates(&ok_result));
        }

        Ok(ok_result)
    }
}
