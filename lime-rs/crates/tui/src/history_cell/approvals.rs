//! Approval decision transcript cells.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReviewDecision {
    Approved,
    ApprovedForSession,
    Denied,
    TimedOut,
    Abort,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ApprovalDecisionSubject {
    Command(Vec<String>),
    NetworkAccess { target: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecisionActor {
    User,
    Policy,
}

pub(crate) fn new_approval_decision_cell(
    subject: ApprovalDecisionSubject,
    decision: ReviewDecision,
    actor: ApprovalDecisionActor,
) -> Box<dyn HistoryCell> {
    let subject = match subject {
        ApprovalDecisionSubject::Command(command) => command.join(" "),
        ApprovalDecisionSubject::NetworkAccess { target } => target,
    };
    let actor = match actor {
        ApprovalDecisionActor::User => "user",
        ApprovalDecisionActor::Policy => "policy",
    };
    let decision = match decision {
        ReviewDecision::Approved => "approved",
        ReviewDecision::ApprovedForSession => "approved for session",
        ReviewDecision::Denied => "denied",
        ReviewDecision::TimedOut => "timed out",
        ReviewDecision::Abort => "aborted",
    };
    Box::new(PlainHistoryCell::new(vec![Line::from(format!(
        "{actor} {decision}: {subject}"
    ))]))
}
