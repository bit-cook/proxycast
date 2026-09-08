//! Bounded replay buffer for notifications and interactive requests by thread.
//!
//! The App Server stream is shared by all threads visible to the TUI. When an agent overview or
//! a background child thread is active, events for another thread must remain ordered until that
//! thread is resumed. This owner keeps the buffer finite and never stores requests it cannot
//! replay safely.

use std::borrow::Cow;

use app_server_protocol::protocol::v2::ServerNotification;

use super::thread_events::{ThreadBufferedEvent, ThreadEventStore};

/// Maximum number of events retained for one thread.
pub(crate) const THREAD_EVENT_CHANNEL_CAPACITY: usize = 256;
const MAX_COALESCED_AGENT_MESSAGE_DELTA_BYTES: usize = 4 * 1024;
const MAX_BUFFERED_AGENT_MESSAGE_DELTA_BYTES: usize = 256 * 1024;

impl ThreadEventStore {
    pub(super) fn push_replay_notification(&mut self, notification: Cow<'_, ServerNotification>) {
        if let ServerNotification::AgentMessageDelta(delta) = notification.as_ref() {
            if delta.delta.len() > MAX_BUFFERED_AGENT_MESSAGE_DELTA_BYTES {
                return;
            }
            let can_coalesce = match self.buffer.back() {
                Some(ThreadBufferedEvent::Notification(previous)) => match previous.as_ref() {
                    ServerNotification::AgentMessageDelta(previous) => {
                        previous.thread_id == delta.thread_id
                            && previous.turn_id == delta.turn_id
                            && previous.item_id == delta.item_id
                            && previous.delta.len().saturating_add(delta.delta.len())
                                <= MAX_COALESCED_AGENT_MESSAGE_DELTA_BYTES
                    }
                    _ => false,
                },
                _ => false,
            };
            if can_coalesce {
                if let Some(ThreadBufferedEvent::Notification(previous)) = self.buffer.back_mut() {
                    if let ServerNotification::AgentMessageDelta(previous) = previous.as_mut() {
                        previous.delta.push_str(&delta.delta);
                        self.buffered_agent_message_delta_bytes = self
                            .buffered_agent_message_delta_bytes
                            .saturating_add(delta.delta.len());
                        self.evict_overflowing_events();
                        return;
                    }
                }
            }
        }

        self.push_buffered_event(ThreadBufferedEvent::Notification(Box::new(
            notification.into_owned(),
        )));
    }

    pub(super) fn push_buffered_event(&mut self, event: ThreadBufferedEvent) -> bool {
        if matches!(event, ThreadBufferedEvent::Request(_)) && self.buffer.len() >= self.capacity {
            return false;
        }
        if let ThreadBufferedEvent::Notification(notification) = &event {
            if let ServerNotification::AgentMessageDelta(delta) = notification.as_ref() {
                self.buffered_agent_message_delta_bytes = self
                    .buffered_agent_message_delta_bytes
                    .saturating_add(delta.delta.len());
            }
        }
        self.buffer.push_back(event);
        self.evict_overflowing_events();
        true
    }

    fn evict_overflowing_events(&mut self) {
        while self.buffer.len() > self.capacity
            || self.buffered_agent_message_delta_bytes > MAX_BUFFERED_AGENT_MESSAGE_DELTA_BYTES
        {
            let Some(index) = self
                .buffer
                .iter()
                .position(|event| matches!(event, ThreadBufferedEvent::Notification(_)))
            else {
                break;
            };
            let Some(ThreadBufferedEvent::Notification(notification)) = self.buffer.remove(index)
            else {
                continue;
            };
            if let ServerNotification::AgentMessageDelta(delta) = notification.as_ref() {
                self.buffered_agent_message_delta_bytes = self
                    .buffered_agent_message_delta_bytes
                    .saturating_sub(delta.delta.len());
            }
        }
    }
}

#[cfg(test)]
#[path = "thread_event_buffer_tests.rs"]
mod tests;
