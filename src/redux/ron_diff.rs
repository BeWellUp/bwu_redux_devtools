//! Structural diff between two `ron::Value` trees.
//!
//! Used by the Tree view to color-code what changed between an action's
//! state and the one immediately before it. History entries are full
//! snapshots, not deltas, so the diff is computed on demand from the two
//! `ron::Value` trees rather than stored.

/// The diff status of a single node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffStatus {
    Unchanged,
    Added,
    Removed,
    Changed,
}

/// A tree mirroring a `ron::Value`'s shape (`Map`/`Seq` recurse, everything
/// else is a leaf) annotated with what changed. The Tree renderer walks the
/// original `ron::Value` and this tree in lockstep, looking up each child's
/// diff by map key or seq index.
#[derive(Clone, Debug, PartialEq)]
pub enum DiffNode {
    Leaf(DiffStatus),
    Map(DiffStatus, Vec<(ron::Value, DiffNode)>),
    Seq(DiffStatus, Vec<DiffNode>),
}

impl DiffNode {
    pub const fn status(&self) -> DiffStatus {
        match self {
            Self::Leaf(status) | Self::Map(status, _) | Self::Seq(status, _) => *status,
        }
    }

    /// Whether this node or any descendant changed. Used both to filter out
    /// fully-unchanged subtrees (the "hide unchanged" toggle) and to mark an
    /// ancestor `SubMenu` that something inside it changed even while
    /// collapsed.
    pub fn has_changes(&self) -> bool {
        match self {
            Self::Leaf(status) => *status != DiffStatus::Unchanged,
            Self::Map(status, entries) => {
                *status != DiffStatus::Unchanged
                    || entries.iter().any(|(_, node)| node.has_changes())
            }
            Self::Seq(status, items) => {
                *status != DiffStatus::Unchanged || items.iter().any(Self::has_changes)
            }
        }
    }

    /// The child diff for a map entry with this key, if this node is a `Map`
    /// and it has one (entries are matched by key equality, not position).
    pub fn map_child(&self, key: &ron::Value) -> Option<&Self> {
        match self {
            Self::Map(_, entries) => entries.iter().find(|(k, _)| k == key).map(|(_, node)| node),
            Self::Leaf(_) | Self::Seq(_, _) => None,
        }
    }

    /// The child diff for a seq entry at this position, if this node is a
    /// `Seq` and has one.
    pub fn seq_child(&self, index: usize) -> Option<&Self> {
        match self {
            Self::Seq(_, items) => items.get(index),
            Self::Leaf(_) | Self::Map(_, _) => None,
        }
    }
}

/// Diff `new` against `old`, the previous entry's state. `old = None` means
/// there is no previous entry (the first action for this app) — everything
/// is reported `Unchanged` rather than `Added`, since there's nothing to
/// meaningfully compare against.
pub fn diff(old: Option<&ron::Value>, new: &ron::Value) -> DiffNode {
    old.map_or_else(
        || tag_tree(new, DiffStatus::Unchanged),
        |old| diff_values(old, new),
    )
}

fn diff_values(old: &ron::Value, new: &ron::Value) -> DiffNode {
    match (old, new) {
        (ron::Value::Option(old_inner), ron::Value::Option(new_inner)) => {
            match (old_inner.as_deref(), new_inner.as_deref()) {
                (Some(old_value), Some(new_value)) => diff_values(old_value, new_value),
                (None, None) => DiffNode::Leaf(DiffStatus::Unchanged),
                (None, Some(new_value)) => tag_tree(new_value, DiffStatus::Added),
                (Some(old_value), None) => tag_tree(old_value, DiffStatus::Removed),
            }
        }
        (ron::Value::Map(old_map), ron::Value::Map(new_map)) => {
            let mut entries = Vec::new();
            let mut any_change = false;

            for (key, new_value) in new_map.iter() {
                let child = old_map.get(key).map_or_else(
                    || tag_tree(new_value, DiffStatus::Added),
                    |old_value| diff_values(old_value, new_value),
                );
                any_change |= child.has_changes();
                entries.push((key.clone(), child));
            }
            for (key, old_value) in old_map.iter() {
                if new_map.get(key).is_none() {
                    entries.push((key.clone(), tag_tree(old_value, DiffStatus::Removed)));
                    any_change = true;
                }
            }

            let status = if any_change {
                DiffStatus::Changed
            } else {
                DiffStatus::Unchanged
            };
            DiffNode::Map(status, entries)
        }
        (ron::Value::Seq(old_items), ron::Value::Seq(new_items)) => {
            let mut children = Vec::new();
            let mut any_change = false;

            for (index, new_value) in new_items.iter().enumerate() {
                let child = old_items.get(index).map_or_else(
                    || tag_tree(new_value, DiffStatus::Added),
                    |old_value| diff_values(old_value, new_value),
                );
                any_change |= child.has_changes();
                children.push(child);
            }
            for old_value in old_items.iter().skip(new_items.len()) {
                children.push(tag_tree(old_value, DiffStatus::Removed));
                any_change = true;
            }

            let status = if any_change {
                DiffStatus::Changed
            } else {
                DiffStatus::Unchanged
            };
            DiffNode::Seq(status, children)
        }
        _ if old == new => tag_tree(new, DiffStatus::Unchanged),
        _ => tag_tree(new, DiffStatus::Changed),
    }
}

/// Tag `value` and its entire subtree with the same status — used when a
/// value was wholly added/removed/changed and there's no matching previous
/// value to recurse into.
fn tag_tree(value: &ron::Value, status: DiffStatus) -> DiffNode {
    match value {
        ron::Value::Map(map) => DiffNode::Map(
            status,
            map.iter()
                .map(|(key, value)| (key.clone(), tag_tree(value, status)))
                .collect(),
        ),
        ron::Value::Seq(items) => {
            DiffNode::Seq(status, items.iter().map(|v| tag_tree(v, status)).collect())
        }
        ron::Value::Bool(_)
        | ron::Value::Char(_)
        | ron::Value::Number(_)
        | ron::Value::Option(_)
        | ron::Value::String(_)
        | ron::Value::Bytes(_)
        | ron::Value::Unit => DiffNode::Leaf(status),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use ron::Value;

    use super::*;

    fn map(entries: &[(&str, Value)]) -> Value {
        Value::Map(
            entries
                .iter()
                .map(|(k, v)| (Value::String((*k).to_owned()), v.clone()))
                .collect(),
        )
    }

    #[test]
    fn no_previous_state_is_unchanged() {
        let new = map(&[("a", Value::Bool(true))]);
        let result = diff(None, &new);
        assert_eq!(result.status(), DiffStatus::Unchanged);
        assert!(!result.has_changes());
    }

    #[test]
    fn scalar_change_is_detected() {
        let old = map(&[("a", Value::Bool(true))]);
        let new = map(&[("a", Value::Bool(false))]);
        let result = diff(Some(&old), &new);
        assert_eq!(result.status(), DiffStatus::Changed);
        let child = result.map_child(&Value::String("a".to_owned())).unwrap();
        assert_eq!(child.status(), DiffStatus::Changed);
    }

    #[test]
    fn identical_state_is_unchanged() {
        let old = map(&[("a", Value::Bool(true))]);
        let new = map(&[("a", Value::Bool(true))]);
        let result = diff(Some(&old), &new);
        assert_eq!(result.status(), DiffStatus::Unchanged);
        assert!(!result.has_changes());
    }

    #[test]
    fn added_map_key_is_detected() {
        let old = map(&[]);
        let new = map(&[("a", Value::Bool(true))]);
        let result = diff(Some(&old), &new);
        assert_eq!(result.status(), DiffStatus::Changed);
        let child = result.map_child(&Value::String("a".to_owned())).unwrap();
        assert_eq!(child.status(), DiffStatus::Added);
    }

    #[test]
    fn removed_map_key_is_detected() {
        let old = map(&[("a", Value::Bool(true))]);
        let new = map(&[]);
        let result = diff(Some(&old), &new);
        assert_eq!(result.status(), DiffStatus::Changed);
        let child = result.map_child(&Value::String("a".to_owned())).unwrap();
        assert_eq!(child.status(), DiffStatus::Removed);
    }

    #[test]
    fn nested_map_change_bubbles_up_has_changes() {
        let old = map(&[("outer", map(&[("inner", Value::Bool(true))]))]);
        let new = map(&[("outer", map(&[("inner", Value::Bool(false))]))]);
        let result = diff(Some(&old), &new);
        assert!(result.has_changes());
        let outer = result
            .map_child(&Value::String("outer".to_owned()))
            .unwrap();
        assert!(outer.has_changes());
        let inner = outer.map_child(&Value::String("inner".to_owned())).unwrap();
        assert_eq!(inner.status(), DiffStatus::Changed);
    }

    #[test]
    fn seq_positional_change_is_detected() {
        let old = Value::Seq(vec![Value::Bool(true), Value::Bool(true)]);
        let new = Value::Seq(vec![Value::Bool(true), Value::Bool(false)]);
        let result = diff(Some(&old), &new);
        assert_eq!(result.status(), DiffStatus::Changed);
        assert_eq!(result.seq_child(0).unwrap().status(), DiffStatus::Unchanged);
        assert_eq!(result.seq_child(1).unwrap().status(), DiffStatus::Changed);
    }

    #[test]
    fn seq_growth_marks_new_items_added() {
        let old = Value::Seq(vec![Value::Bool(true)]);
        let new = Value::Seq(vec![Value::Bool(true), Value::Bool(false)]);
        let result = diff(Some(&old), &new);
        assert_eq!(result.seq_child(0).unwrap().status(), DiffStatus::Unchanged);
        assert_eq!(result.seq_child(1).unwrap().status(), DiffStatus::Added);
    }

    #[test]
    fn type_change_marks_whole_value_changed() {
        let old = Value::Bool(true);
        let new = Value::Seq(vec![Value::Bool(true)]);
        let result = diff(Some(&old), &new);
        assert_eq!(result.status(), DiffStatus::Changed);
    }

    #[test]
    fn option_some_to_some_change_recurses_into_inner() {
        let old = map(&[("a", Value::Option(Some(Box::new(Value::Bool(true)))))]);
        let new = map(&[("a", Value::Option(Some(Box::new(Value::Bool(false)))))]);
        let result = diff(Some(&old), &new);
        assert_eq!(result.status(), DiffStatus::Changed);
        let child = result.map_child(&Value::String("a".to_owned())).unwrap();
        assert_eq!(child.status(), DiffStatus::Changed);
    }

    #[test]
    fn option_none_to_some_is_added() {
        let old = Value::Option(None);
        let new = Value::Option(Some(Box::new(Value::Bool(true))));
        let result = diff(Some(&old), &new);
        assert_eq!(result.status(), DiffStatus::Added);
    }

    #[test]
    fn option_some_to_none_is_removed() {
        let old = Value::Option(Some(Box::new(Value::Bool(true))));
        let new = Value::Option(None);
        let result = diff(Some(&old), &new);
        assert_eq!(result.status(), DiffStatus::Removed);
    }

    #[test]
    fn option_none_to_none_is_unchanged() {
        let old = Value::Option(None);
        let new = Value::Option(None);
        let result = diff(Some(&old), &new);
        assert_eq!(result.status(), DiffStatus::Unchanged);
    }

    #[test]
    fn option_some_nested_map_unchanged_stays_unchanged() {
        let old = Value::Option(Some(Box::new(map(&[("a", Value::Bool(true))]))));
        let new = Value::Option(Some(Box::new(map(&[("a", Value::Bool(true))]))));
        let result = diff(Some(&old), &new);
        assert_eq!(result.status(), DiffStatus::Unchanged);
    }
}
