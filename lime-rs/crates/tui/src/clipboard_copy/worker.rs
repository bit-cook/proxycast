//! Session-lived worker for native and terminal clipboard writes.
//!
//! Clipboard providers are synchronous and can block on an X11, Wayland, or desktop broker.
//! Keep those calls out of the TUI event loop. The UI owns the short setup deadline and consumes
//! completions only at draw boundaries; a late native result is drained and never applied to a
//! newer action.

use super::{copy_to_clipboard, copy_to_primary, CopyOutcome};
use crate::tui::FrameRequester;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

const COPY_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) type CopyResult = Result<CopyOutcome, String>;

pub(crate) struct CopyCompletion {
    pub(crate) clipboard: CopyResult,
    pub(crate) primary: Option<CopyResult>,
}

fn error_completion(error: impl Into<String>) -> CopyCompletion {
    CopyCompletion {
        clipboard: Err(error.into()),
        primary: None,
    }
}

struct Request {
    id: u64,
    text: String,
    frames: FrameRequester,
    primary: bool,
}

struct Response {
    id: u64,
    completion: CopyCompletion,
}

struct PendingCopy {
    id: u64,
    deadline: Instant,
    frames: FrameRequester,
}

/// One worker is shared by the whole TUI session. There is intentionally no backlog: a second
/// copy waits until the first platform operation has returned and its response has been drained.
#[derive(Default)]
pub(crate) struct ClipboardWorker {
    requests: Option<Sender<Request>>,
    responses: Option<Receiver<Response>>,
    in_flight: Option<u64>,
    pending: Option<PendingCopy>,
    next_id: u64,
}

impl ClipboardWorker {
    /// Start an asynchronous copy. `None` means another native operation is still draining.
    pub(crate) fn request_copy(
        &mut self,
        text: String,
        frames: FrameRequester,
    ) -> Result<Option<u64>, String> {
        self.request_copy_internal(text, frames, false, copy_to_clipboard)
    }

    /// Start a copy that also publishes the X11 PRIMARY selection. PRIMARY is best effort and
    /// remains independent from the ordinary CLIPBOARD result.
    pub(crate) fn request_copy_with_primary(
        &mut self,
        text: String,
        frames: FrameRequester,
        primary: bool,
    ) -> Result<Option<u64>, String> {
        self.request_copy_internal(text, frames, primary, copy_to_clipboard)
    }

    fn request_copy_internal(
        &mut self,
        text: String,
        frames: FrameRequester,
        primary: bool,
        copy: fn(&str) -> CopyResult,
    ) -> Result<Option<u64>, String> {
        if self.in_flight.is_some() {
            return Ok(None);
        }
        if text.is_empty() {
            return Err("nothing to copy: the selected content is empty".to_string());
        }
        let sender = self.ensure_started(copy)?;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let id = self.next_id;
        sender
            .send(Request {
                id,
                text,
                frames: frames.clone(),
                primary,
            })
            .map_err(|_| "clipboard worker stopped".to_string())?;
        self.in_flight = Some(id);
        self.pending = Some(PendingCopy {
            id,
            deadline: Instant::now() + COPY_TIMEOUT,
            frames,
        });
        self.pending
            .as_ref()
            .expect("pending clipboard copy installed")
            .frames
            .schedule_frame_in(COPY_TIMEOUT);
        Ok(Some(id))
    }

    /// Poll a completion at a draw boundary. A timeout is reported immediately, while the
    /// eventual worker response is consumed later to release the worker for the next copy.
    pub(crate) fn poll(&mut self) -> Option<(u64, CopyCompletion)> {
        let now = Instant::now();
        if let Some(pending) = self.pending.as_ref() {
            if now >= pending.deadline {
                let id = pending.id;
                self.pending = None;
                return Some((
                    id,
                    error_completion("clipboard setup timed out; copy abandoned"),
                ));
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
                    .map(|_| (id, error_completion("clipboard worker stopped")));
            }
        };

        if self.in_flight == Some(response.id) {
            self.in_flight = None;
        }
        let pending = self.pending.take();
        if pending.is_some_and(|pending| pending.id == response.id) {
            Some((response.id, response.completion))
        } else {
            // A timed-out/canceled result is deliberately discarded. It must still be drained so
            // the next request cannot overlap a previous native clipboard operation.
            None
        }
    }

    fn ensure_started(&mut self, copy: fn(&str) -> CopyResult) -> Result<Sender<Request>, String> {
        if let Some(sender) = &self.requests {
            return Ok(sender.clone());
        }
        let (request_tx, request_rx) = mpsc::channel::<Request>();
        let (response_tx, response_rx) = mpsc::channel::<Response>();
        std::thread::Builder::new()
            .name("tui-clipboard-copy".to_string())
            .spawn(move || {
                while let Ok(request) = request_rx.recv() {
                    let primary = request.primary.then(|| copy_to_primary(&request.text));
                    let clipboard = copy(&request.text);
                    let _ = response_tx.send(Response {
                        id: request.id,
                        completion: CopyCompletion { clipboard, primary },
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

impl Drop for ClipboardWorker {
    fn drop(&mut self) {
        // Do not join: a platform clipboard broker may be unresponsive during shutdown. Dropping
        // the sender asks the worker to exit after its current operation returns.
        self.requests.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames() -> FrameRequester {
        FrameRequester::test_dummy()
    }

    fn requested_copy(_text: &str) -> CopyResult {
        Ok(CopyOutcome::Requested)
    }

    fn slow_requested_copy(_text: &str) -> CopyResult {
        std::thread::sleep(Duration::from_millis(20));
        Ok(CopyOutcome::Requested)
    }

    #[test]
    fn empty_copy_is_rejected_before_starting_worker() {
        let mut worker = ClipboardWorker::default();
        let error = worker
            .request_copy(String::new(), frames())
            .expect_err("empty copy must fail");
        assert!(error.contains("nothing to copy"));
        assert!(worker.requests.is_none());
    }

    #[test]
    fn worker_does_not_accept_a_second_copy_while_busy() {
        let mut worker = ClipboardWorker::default();
        let first = worker
            .request_copy_internal("first".to_string(), frames(), false, requested_copy)
            .expect("request");
        assert!(first.is_some());
        let second = worker
            .request_copy_internal("second".to_string(), frames(), false, requested_copy)
            .expect("busy request");
        assert!(second.is_none());
    }

    #[test]
    fn timed_out_copy_can_be_drained_without_reaching_ui() {
        let mut worker = ClipboardWorker::default();
        let id = worker
            .request_copy_internal("late".to_string(), frames(), false, slow_requested_copy)
            .expect("request")
            .expect("worker started");
        worker.pending.as_mut().expect("pending copy").deadline = Instant::now()
            .checked_sub(Duration::from_millis(1))
            .expect("instant subtraction");
        let timeout = worker.poll().expect("timeout");
        assert_eq!(timeout.0, id);
        assert!(timeout.1.clipboard.is_err());
        // A subsequent poll either drains the worker response or waits for it. It must not
        // surface a second UI completion for the timed-out request.
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            if worker.in_flight.is_none() {
                break;
            }
            let _ = worker.poll();
            std::thread::yield_now();
        }
        assert!(worker.in_flight.is_none());
    }
}
