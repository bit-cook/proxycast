//! Group membership is shared by task filtering, rendering and paging.

use super::{AgentsOverviewRow, AgentsOverviewView};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum AgentsOverviewGrouping {
    #[default]
    Project,
    Status,
}

impl AgentsOverviewView {
    pub(super) fn same_group(&self, left: &AgentsOverviewRow, right: &AgentsOverviewRow) -> bool {
        match self.grouping {
            AgentsOverviewGrouping::Project => left.thread.cwd == right.thread.cwd,
            AgentsOverviewGrouping::Status => left.group == right.group,
        }
    }
}
