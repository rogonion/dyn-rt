//! Domain types for walking a parsed JSONPath: the cursor tracking which
//! recursive-descent group and which segment within the group is current
//! (port of go-json's `internal.PathSegmentsIndexes`).

/// Position while walking a [`dyn_rt_json_path::RecursiveDescentSegments`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathCursor {
    /// Index of the current recursive-descent group (`..`-separated part).
    pub recursive: usize,
    /// Index of the current segment within the current group.
    pub collection: usize,
    /// Index of the last recursive-descent group.
    pub last_recursive: usize,
}

impl PathCursor {
    /// A cursor starting at the first group / first segment.
    pub fn start(segment_count: usize) -> Self {
        PathCursor {
            recursive: 0,
            collection: 0,
            last_recursive: segment_count.saturating_sub(1),
        }
    }

    /// Whether the cursor sits on the very last segment of the whole path.
    pub fn is_terminal(&self, groups: &dyn_rt_json_path::RecursiveDescentSegments) -> bool {
        self.collection == groups.0[self.recursive].0.len().saturating_sub(1)
            && self.recursive == self.last_recursive
    }

    /// Whether the cursor sits on the last segment of its group (so the next
    /// group begins at a `..` boundary).
    pub fn is_group_end(&self, groups: &dyn_rt_json_path::RecursiveDescentSegments) -> bool {
        self.collection == groups.0[self.recursive].0.len().saturating_sub(1)
    }

    /// Cursor advanced by one segment within the current group.
    pub fn next_in_group(&self) -> Self {
        PathCursor {
            collection: self.collection + 1,
            ..*self
        }
    }

    /// Cursor moved to the first segment of the next recursive-descent group.
    pub fn next_group(&self) -> Self {
        PathCursor {
            recursive: self.recursive + 1,
            collection: 0,
            ..*self
        }
    }
}
