use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Maximum number of messages that can be queued per channel
const CHANNEL_CAPACITY: usize = 1024;

/// Pub/Sub manager for handling publish/subscribe operations
#[derive(Clone)]
pub struct PubSub {
    /// Shared state containing channels and their subscribers
    shared: Arc<Mutex<PubSubState>>,
}

/// Internal state for Pub/Sub
struct PubSubState {
    /// Map of channel names to broadcast senders
    channels: HashMap<String, broadcast::Sender<Bytes>>,
}

impl PubSub {
    /// Create a new Pub/Sub manager
    pub fn new() -> Self {
        PubSub {
            shared: Arc::new(Mutex::new(PubSubState {
                channels: HashMap::new(),
            })),
        }
    }

    /// Publish a message to a channel
    ///
    /// Returns the number of subscribers that received the message
    pub fn publish(&self, channel: &str, message: Bytes) -> usize {
        let state = self.shared.lock().unwrap();

        if let Some(sender) = state.channels.get(channel) {
            // Send to all subscribers
            // receiver_count() includes the sender itself, so subtract 1
            sender
                .send(message)
                .map(|_| sender.receiver_count())
                .unwrap_or(0)
        } else {
            // No subscribers for this channel
            0
        }
    }

    /// Subscribe to a channel
    ///
    /// Returns a receiver that will get all messages published to the channel
    pub fn subscribe(&self, channel: String) -> broadcast::Receiver<Bytes> {
        let mut state = self.shared.lock().unwrap();

        // Get or create the channel
        let sender = state
            .channels
            .entry(channel)
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0);

        sender.subscribe()
    }

    /// Get the number of subscribers for a channel
    pub fn num_subscribers(&self, channel: &str) -> usize {
        let state = self.shared.lock().unwrap();

        state
            .channels
            .get(channel)
            .map(|sender| sender.receiver_count())
            .unwrap_or(0)
    }

    /// Get the number of active channels
    pub fn num_channels(&self) -> usize {
        let state = self.shared.lock().unwrap();
        state.channels.len()
    }

    /// Clean up empty channels (channels with no subscribers)
    pub fn cleanup_empty_channels(&self) {
        let mut state = self.shared.lock().unwrap();

        // Remove channels with no subscribers
        state
            .channels
            .retain(|_, sender| sender.receiver_count() > 0);
    }
}

impl Default for PubSub {
    fn default() -> Self {
        Self::new()
    }
}
