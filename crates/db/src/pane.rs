use gpui::Axis;
use settings::DockAnchor;

use crate::{items::ItemId, workspace::WorkspaceId};

use super::Db;

// We have an many-branched, unbalanced tree with three types:
// Pane Groups
// Panes
// Items

// The root is always a Pane Group
// Pane Groups can have 0 (or more) Panes and/or Pane Groups as children
// Panes can have 0 or more items as children
// Panes can be their own root
// Items cannot have children
// References pointing down is hard (SQL doesn't like arrays)
// References pointing up is easy (1-1 item / parent relationship) but is harder to query
//

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct PaneId {
    workspace_id: WorkspaceId,
    pane_id: usize,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct PaneGroupId {
    workspace_id: WorkspaceId,
    group_id: usize,
}

impl PaneGroupId {
    pub(crate) fn root(workspace_id: WorkspaceId) -> Self {
        Self {
            workspace_id,
            group_id: 0,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SerializedPaneGroup {
    group_id: PaneGroupId,
    axis: Axis,
    children: Vec<PaneGroupChild>,
}

impl SerializedPaneGroup {
    pub(crate) fn empty_root(workspace_id: WorkspaceId) -> Self {
        Self {
            group_id: PaneGroupId::root(workspace_id),
            axis: Default::default(),
            children: Default::default(),
        }
    }
}

struct PaneGroupChildRow {
    child_pane_id: Option<usize>,
    child_group_id: Option<usize>,
    index: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PaneGroupChild {
    Pane(SerializedPane),
    Group(SerializedPaneGroup),
}

#[derive(Debug, PartialEq, Eq)]
pub struct SerializedPane {
    pane_id: PaneId,
    children: Vec<ItemId>,
}

pub(crate) const PANE_M_1: &str = "
BEGIN TRANSACTION;

CREATE TABLE dock_panes(
    dock_pane_id INTEGER PRIMARY KEY,
    workspace_id INTEGER NOT NULL,
    anchor_position TEXT NOT NULL, -- Enum: 'Bottom' / 'Right' / 'Expanded'
    shown INTEGER NOT NULL, -- Boolean
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE pane_groups(
    group_id INTEGER PRIMARY KEY,
    workspace_id INTEGER NOT NULL,
    parent_group INTEGER, -- NULL indicates that this is a root node
    axis TEXT NOT NULL, -- Enum:  'Vertical' / 'Horizontal'
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    FOREIGN KEY(parent_group) REFERENCES pane_groups(group_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE grouped_panes(
    pane_id INTEGER PRIMARY KEY,
    workspace_id INTEGER NOT NULL,
    group_id INTEGER NOT NULL,
    idx INTEGER NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    FOREIGN KEY(group_id) REFERENCES pane_groups(group_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE items(
    item_id INTEGER PRIMARY KEY,
    workspace_id INTEGER NOT NULL,
    kind TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE group_items(
    workspace_id INTEGER NOT NULL,
    pane_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    idx INTEGER NOT NULL,
    PRIMARY KEY (workspace_id, pane_id, item_id)
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    FOREIGN KEY(pane_id) REFERENCES grouped_panes(pane_id) ON DELETE CASCADE,
    FOREIGN KEY(item_id) REFERENCES items(item_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE dock_items(
    workspace_id INTEGER NOT NULL,
    dock_pane_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    idx INTEGER NOT NULL,
    PRIMARY KEY (workspace_id, dock_pane_id, item_id)
    FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    FOREIGN KEY(dock_pane_id) REFERENCES dock_panes(dock_pane_id) ON DELETE CASCADE,
    FOREIGN KEY(item_id) REFERENCES items(item_id)ON DELETE CASCADE
) STRICT;

COMMIT;
";

#[derive(Default, Debug)]
pub struct SerializedDockPane {
    pub workspace: WorkspaceId,
    pub anchor_position: DockAnchor,
    pub shown: bool,
}

impl Db {
    pub(crate) fn get_pane_group(&self, pane_group_id: PaneGroupId) -> SerializedPaneGroup {
        let axis = self.get_pane_group_axis(pane_group_id);
        let mut children: Vec<(usize, PaneGroupChild)> = Vec::new();
        for child_row in self.get_pane_group_children(pane_group_id) {
            if let Some(child_pane_id) = child_row.child_pane_id {
                children.push((
                    child_row.index,
                    PaneGroupChild::Pane(self.get_pane(PaneId {
                        workspace_id: pane_group_id.workspace_id,
                        pane_id: child_pane_id,
                    })),
                ));
            } else if let Some(child_group_id) = child_row.child_group_id {
                children.push((
                    child_row.index,
                    PaneGroupChild::Group(self.get_pane_group(PaneGroupId {
                        workspace_id: pane_group_id.workspace_id,
                        group_id: child_group_id,
                    })),
                ));
            }
        }
        children.sort_by_key(|(index, _)| *index);

        SerializedPaneGroup {
            group_id: pane_group_id,
            axis,
            children: children.into_iter().map(|(_, child)| child).collect(),
        }
    }

    fn get_pane_group_children(
        &self,
        pane_group_id: PaneGroupId,
    ) -> impl Iterator<Item = PaneGroupChildRow> {
        Vec::new().into_iter()
    }

    fn get_pane_group_axis(&self, pane_group_id: PaneGroupId) -> Axis {
        unimplemented!();
    }

    pub fn save_pane_splits(&self, center_pane_group: SerializedPaneGroup) {
        // Delete the center pane group for this workspace and any of its children
        // Generate new pane group IDs as we go through
        // insert them
        // Items garbage collect themselves when dropped
    }

    pub(crate) fn get_pane(&self, pane_id: PaneId) -> SerializedPane {
        unimplemented!();
    }

    pub fn get_dock_pane(&self, workspace: WorkspaceId) -> Option<SerializedDockPane> {
        unimplemented!()
    }

    pub fn save_dock_pane(&self, dock_pane: SerializedDockPane) {}
}

#[cfg(test)]
mod tests {
    use settings::DockAnchor;

    use crate::Db;

    use super::SerializedDockPane;

    #[test]
    fn test_basic_dock_pane() {
        let db = Db::open_in_memory();

        let workspace = db.make_new_workspace::<String>(&[]);

        db.update_worktrees(&workspace.workspace_id, &["/tmp"]);

        db.save_dock_pane(SerializedDockPane {
            workspace: workspace.workspace_id,
            anchor_position: DockAnchor::Expanded,
            shown: true,
        });

        let new_workspace = db.workspace_for_roots(&["/tmp"]);
    }
}
