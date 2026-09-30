//! Session-lived clipboard text worker for mouse paste.
//!
//! Native clipboard APIs are synchronous and may block on an X11/Wayland/Windows broker. Codex
//! keeps those calls off the TUI event loop and treats the five-second deadline as a UI contract:
//! a late native result is drained and discarded, never inserted into a newer draft.

use super::{read_clipboard_text, ClipboardTextSource};
use crate::tui::FrameRequester;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

const READ_TIMEOUT: Duration = Duration::from_secs(5);

type ReadResult = Result<String, String>;
type Reader = fn(ClipboardTextSource) -> ReadResult;

struct Request {
    id: u64,
    source: ClipboardTextSource,
    frames: FrameRequester,
}

struct Response {
    id: u64,
    result: ReadResult,
}

/// One worker is shared by the whole TUI session. It deliberately accepts only one in-flight
/// native read; a canceled/expired read must finish draining before a later mouse gesture can
/// start another one.
#[derive(Default)]
pub(crate) struct ClipboardWorker {
    requests: Option<Sender<Request>>,
    responses: Option<Receiver<Response>>,
    in_flight: Option<u64>,
    pending: Option<PendingRead>,
    next_id: u64,
}

struct PendingRead {
    id: u64,
    deadline: Instant,
    frames: FrameRequester,
}

impl ClipboardWorker {
    /// Begin a clipboard read. `None` means the previous native operation is still draining.
    pub(crate) fn request_read(
        &mut self,
        source: ClipboardTextSource,
        frames: FrameRequester,
    ) -> Result<Option<u64>, String> {
        self.request_read_with(source, frames, default_reader)
    }

    fn request_read_with(
        &mut self,
        source: ClipboardTextSource,
        frames: FrameRequester,
        reader: Reader,
    ) -> Result<Option<u64>, String> {
        if self.in_flight.is_some() {
            return Ok(None);
        }
        let sender = self.ensure_started(reader)?;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let id = self.next_id;
        sender
            .send(Request {
                id,
                source,
                frames: frames.clone(),
            })
            .map_err(|_| "clipboard worker stopped".to_string())?;
        self.in_flight = Some(id);
        self.pending = Some(PendingRead {
            id,
            deadline: Instant::now() + READ_TIMEOUT,
            frames,
        });
        self.pending
            .as_ref()
            .expect("pending clipboard read installed")
            .frames
            .schedule_frame_in(READ_TIMEOUT);
        Ok(Some(id))
    }

    /// Cancel the current UI owner. The worker still drains the native call, but its response is
    /// no longer eligible for insertion into the composer.
    pub(crate) fn cancel(&mut self, id: Option<u64>) {
        if id.is_none()
            || self
                .pending
                .as_ref()
                .is_some_and(|pending| Some(pending.id) == id)
        {
            self.pending = None;
        }
    }

    /// Poll a completion at a draw boundary. A response for a canceled or timed-out request is
    /// consumed only to release the worker for the next request.
    pub(crate) fn poll(&mut self) -> Option<(u64, ReadResult)> {
        self.poll_at(Instant::now())
    }

    fn poll_at(&mut self, now: Instant) -> Option<(u64, ReadResult)> {
        if let Some(pending) = self.pending.as_ref() {
            if now >= pending.deadline {
                let id = pending.id;
                self.pending = None;
                return Some((id, Err("clipboard read timed out".to_string())));
            }
            pending.frames.schedule_frame_in(pending.deadline - now);
        }

        let response = match self.responses.as_ref()?.try_recv() {
            Ok(response) => response,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                let id = self.in_flight.take()?;
                let pending = self.pending.take();
                self.requests = None;
                self.responses = None;
                return pending
                    .filter(|pending| pending.id == id)
                    .map(|_| (id, Err("clipboard worker stopped".to_string())));
            }
        };
        if self.in_flight == Some(response.id) {
            self.in_flight = None;
        }
        let pending = self.pending.take();
        if pending.is_some_and(|pending| pending.id == response.id) {
            Some((response.id, response.result))
        } else {
            // Stale response: keep draining without stealing a newer request's result.
            None
        }
    }

    fn ensure_started(&mut self, reader: Reader) -> Result<Sender<Request>, String> {
        if let Some(sender) = &self.requests {
            return Ok(sender.clone());
        }
        let (request_tx, request_rx) = mpsc::channel::<Request>();
        let (response_tx, response_rx) = mpsc::channel::<Response>();
        std::thread::Builder::new()
            .name("tui-clipboard".to_string())
            .spawn(move || {
                while let Ok(request) = request_rx.recv() {
                    let result = reader(request.source);
                    let _ = response_tx.send(Response {
                        id: request.id,
                        result,
                    });
                    request.frames.schedule_frame();
                }
            })
            .map_err(|error| format!("could not start clipboard worker: {error}"))?;
        self.requests = Some(request_tx.clone());
        self.responses = Some(response_rx);
        Ok(request_tx)
    }
}

fn default_reader(source: ClipboardTextSource) -> ReadResult {
    read_clipboard_text(source)
}

impl Drop for ClipboardWorker {
    fn drop(&mut self) {
        // Dropping the sender asks the worker to exit after the current native call. We do not
        // join here because platform clipboard brokers are outside the TUI shutdown deadline.
        self.requests.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames() -> FrameRequester {
        FrameRequester::test_dummy()
    }

    fn reader(_source: ClipboardTextSource) -> ReadResult {
        Ok("clipboard text".to_string())
    }

    #[test]
    fn worker_reads_off_thread_and_returns_result() {
        let mut worker = ClipboardWorker::default();
        let id = worker
            .request_read_with(ClipboardTextSource::Clipboard, frames(), reader)
            .expect("request")
            .expect("worker available");
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if let Some((result_id, result)) = worker.poll() {
                assert_eq!(result_id, id);
                assert_eq!(result.expect("text"), "clipboard text");
                break;
            }
            assert!(Instant::now() < deadline, "worker did not respond");
            std::thread::yield_now();
        }
    }

    #[test]
    fn canceled_result_is_drained_and_does_not_reach_ui() {
        fn slow_reader(_source: ClipboardTextSource) -> ReadResult {
            std::thread::sleep(Duration::from_millis(20));
            Ok("stale".to_string())
        }
        let mut worker = ClipboardWorker::default();
        let id = worker
            .request_read_with(ClipboardTextSource::Clipboard, frames(), slow_reader)
            .expect("request")
            .expect("worker available");
        worker.cancel(Some(id));
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            if worker.poll().is_none() {
                std::thread::yield_now();
                continue;
            }
            break;
        }
        assert!(worker.poll().is_none());
        let next = worker
            .request_read_with(ClipboardTextSource::Clipboard, frames(), reader)
            .expect("next request")
            .expect("worker released");
        assert_ne!(id, next);
    }

    #[test]
    fn timeout_is_reported_without_waiting_for_native_reader() {
        let mut worker = ClipboardWorker::default();
        let id = worker
            .request_read_with(ClipboardTextSource::Clipboard, frames(), reader)
            .expect("request")
            .expect("worker available");
        let now = Instant::now();
        worker.pending.as_mut().expect("pending read").deadline = now - Duration::from_millis(1);
        let result = worker.poll_at(now).expect("timeout result");
        assert_eq!(result.0, id);
        assert_eq!(
            result.1.expect_err("must time out"),
            "clipboard read timed out"
        );
    }

    #[test]
    fn disconnected_worker_releases_busy_state() {
        let mut worker = ClipboardWorker::default();
        let id = worker
            .request_read_with(ClipboardTextSource::Clipboard, frames(), reader)
            .expect("request")
            .expect("worker available");
        let (response_tx, response_rx) = mpsc::channel();
        drop(response_tx);
        worker.responses = Some(response_rx);
        let _ = worker.poll();
        assert!(worker.in_flight.is_none());
        assert!(worker.pending.is_none());
        assert!(worker
            .request_read_with(ClipboardTextSource::Clipboard, frames(), reader)
            .expect("restart")
            .is_some());
        assert_ne!(id, worker.in_flight.expect("new request"));
    }
}
