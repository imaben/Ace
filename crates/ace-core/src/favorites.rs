use crate::connection::ConnectionInfo;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FavoriteNode {
    Group {
        id: Uuid,
        name: String,
        children: Vec<FavoriteNode>,
        expanded: bool,
    },
    Connection {
        info: ConnectionInfo,
    },
}

impl FavoriteNode {
    pub fn group(name: impl Into<String>) -> Self {
        Self::Group {
            id: Uuid::new_v4(),
            name: name.into(),
            children: Vec::new(),
            expanded: true,
        }
    }

    pub fn connection(info: ConnectionInfo) -> Self {
        Self::Connection { info }
    }

    pub fn id(&self) -> Uuid {
        match self {
            Self::Group { id, .. } => *id,
            Self::Connection { info } => info.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Group { name, .. } => name,
            Self::Connection { info } => &info.name,
        }
    }

    pub fn walk<'a>(&'a self, out: &mut Vec<&'a FavoriteNode>) {
        out.push(self);
        if let Self::Group { children, .. } = self {
            for c in children {
                c.walk(out);
            }
        }
    }

    pub fn find_mut(&mut self, id: Uuid) -> Option<&mut FavoriteNode> {
        if self.id() == id {
            return Some(self);
        }
        if let Self::Group { children, .. } = self {
            for c in children {
                if let Some(found) = c.find_mut(id) {
                    return Some(found);
                }
            }
        }
        None
    }

    pub fn remove_child(&mut self, id: Uuid) -> Option<FavoriteNode> {
        if let Self::Group { children, .. } = self {
            if let Some(pos) = children.iter().position(|c| c.id() == id) {
                return Some(children.remove(pos));
            }
            for c in children.iter_mut() {
                if let Some(removed) = c.remove_child(id) {
                    return Some(removed);
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FavoritesTree {
    pub roots: Vec<FavoriteNode>,
}

impl FavoritesTree {
    pub fn new() -> Self {
        Self { roots: Vec::new() }
    }

    pub fn add_root(&mut self, node: FavoriteNode) {
        self.roots.push(node);
    }

    pub fn all_connections(&self) -> Vec<&ConnectionInfo> {
        let mut nodes = Vec::new();
        for r in &self.roots {
            r.walk(&mut nodes);
        }
        nodes
            .into_iter()
            .filter_map(|n| match n {
                FavoriteNode::Connection { info } => Some(info),
                _ => None,
            })
            .collect()
    }

    /// Sequel Ace style: whitespace-separated tokens AND-matched against name/host
    /// (group names are not matched; empty query returns all).
    pub fn search(&self, query: &str) -> Vec<&ConnectionInfo> {
        let tokens: Vec<String> = query
            .split_whitespace()
            .map(|t| t.to_lowercase())
            .filter(|t| !t.is_empty())
            .collect();
        self.all_connections()
            .into_iter()
            .filter(|c| {
                if tokens.is_empty() {
                    return true;
                }
                let name = c.name.to_lowercase();
                let host = c.host.to_lowercase();
                tokens
                    .iter()
                    .all(|t| name.contains(t) || host.contains(t))
            })
            .collect()
    }

    pub fn duplicate_connection(&mut self, id: Uuid) -> Option<Uuid> {
        let info = self.all_connections().into_iter().find(|c| c.id == id)?.clone();
        let mut copy = info.clone();
        copy.id = Uuid::new_v4();
        copy.name = format!("{} Copy", info.name);
        copy.password = None;
        let new_id = copy.id;
        self.add_root(FavoriteNode::connection(copy));
        Some(new_id)
    }

    pub fn find_mut(&mut self, id: Uuid) -> Option<&mut FavoriteNode> {
        for r in &mut self.roots {
            if let Some(found) = r.find_mut(id) {
                return Some(found);
            }
        }
        None
    }

    pub fn remove(&mut self, id: Uuid) -> Option<FavoriteNode> {
        if let Some(pos) = self.roots.iter().position(|r| r.id() == id) {
            return Some(self.roots.remove(pos));
        }
        for r in &mut self.roots {
            if let Some(removed) = r.remove_child(id) {
                return Some(removed);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::ConnectionInfo;

    #[test]
    fn search_favorites() {
        let mut tree = FavoritesTree::new();
        let mut info = ConnectionInfo::default();
        info.name = "Prod DB".into();
        info.host = "prod.example.com".into();
        tree.add_root(FavoriteNode::connection(info));
        assert_eq!(tree.search("prod").len(), 1);
        assert!(tree.search("staging").is_empty());
    }
}