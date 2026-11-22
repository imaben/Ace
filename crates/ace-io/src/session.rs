use ace_core::{AceError, AceResult, ConnectionInfo, FavoritesTree};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

/// Ace-native session document (JSON). Also can import Sequel Ace `.spf` plists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDocument {
    pub version: u32,
    pub connection: ConnectionInfo,
    pub selected_database: Option<String>,
    pub selected_table: Option<String>,
    pub query_tabs: Vec<QueryTabState>,
    pub content_filters: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTabState {
    pub title: String,
    pub sql: String,
}

impl SessionDocument {
    pub fn new(connection: ConnectionInfo) -> Self {
        Self {
            version: 1,
            connection,
            selected_database: None,
            selected_table: None,
            query_tabs: vec![QueryTabState {
                title: "Query 1".into(),
                sql: String::new(),
            }],
            content_filters: None,
        }
    }

    pub fn save(&self, path: &Path) -> AceResult<()> {
        let mut doc = self.clone();
        doc.connection = doc.connection.without_secrets();
        let raw = serde_json::to_string_pretty(&doc)?;
        std::fs::write(path, raw).map_err(|e| AceError::Io(e.to_string()))
    }

    pub fn load(path: &Path) -> AceResult<Self> {
        let raw = std::fs::read_to_string(path).map_err(|e| AceError::Io(e.to_string()))?;
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("spf") || e.eq_ignore_ascii_case("plist"))
            .unwrap_or(false)
        {
            return import_spf_plist(&raw);
        }
        serde_json::from_str(&raw).map_err(|e| AceError::Message(e.to_string()))
    }
}

/// Best-effort import of Sequel Ace / Sequel Pro `.spf` connection documents.
pub fn import_spf_plist(raw: &str) -> AceResult<SessionDocument> {
    let value: plist::Value =
        plist::from_bytes(raw.as_bytes()).map_err(|e| AceError::Message(e.to_string()))?;
    let dict = value
        .as_dictionary()
        .ok_or_else(|| AceError::msg("spf root is not a dictionary"))?;

    let mut info = ConnectionInfo::default();
    info.id = Uuid::new_v4();
    if let Some(name) = dict.get("name").and_then(|v| v.as_string()) {
        info.name = name.to_string();
    }
    if let Some(host) = dict.get("host").and_then(|v| v.as_string()) {
        info.host = host.to_string();
    }
    if let Some(user) = dict.get("user").and_then(|v| v.as_string()) {
        info.user = user.to_string();
    }
    if let Some(db) = dict.get("database").and_then(|v| v.as_string()) {
        info.database = Some(db.to_string());
    }
    if let Some(port) = dict.get("port") {
        if let Some(p) = port.as_unsigned_integer() {
            info.port = p as u16;
        } else if let Some(p) = port.as_signed_integer() {
            info.port = p as u16;
        } else if let Some(p) = port.as_string() {
            info.port = p.parse().unwrap_or(3306);
        }
    }

    let mut doc = SessionDocument::new(info);
    if let Some(db) = dict.get("database").and_then(|v| v.as_string()) {
        doc.selected_database = Some(db.to_string());
    }
    Ok(doc)
}

/// Import Sequel Ace favorites plist into FavoritesTree (subset).
pub fn import_favorites_plist(path: &Path) -> AceResult<FavoritesTree> {
    let raw = std::fs::read(path).map_err(|e| AceError::Io(e.to_string()))?;
    let value: plist::Value =
        plist::from_bytes(&raw).map_err(|e| AceError::Message(e.to_string()))?;
    let mut tree = FavoritesTree::new();

    fn walk(node: &plist::Value, tree: &mut FavoritesTree, parent_group: Option<uuid::Uuid>) {
        let Some(dict) = node.as_dictionary() else {
            return;
        };
        let is_group = dict
            .get("IsGroup")
            .and_then(|v| v.as_boolean())
            .unwrap_or(false)
            || dict.get("Children").is_some();

        if is_group {
            let name = dict
                .get("name")
                .or_else(|| dict.get("Name"))
                .and_then(|v| v.as_string())
                .unwrap_or("Group")
                .to_string();
            let group = ace_core::FavoriteNode::group(name);
            let gid = group.id();
            if let Some(pid) = parent_group {
                if let Some(ace_core::FavoriteNode::Group { children, .. }) = tree.find_mut(pid) {
                    children.push(group);
                }
            } else {
                tree.add_root(group);
            }
            if let Some(children) = dict.get("Children").and_then(|v| v.as_array()) {
                for child in children {
                    walk(child, tree, Some(gid));
                }
            }
        } else {
            let mut info = ConnectionInfo::default();
            info.id = Uuid::new_v4();
            if let Some(n) = dict.get("name").or_else(|| dict.get("Name")).and_then(|v| v.as_string()) {
                info.name = n.to_string();
            }
            if let Some(h) = dict.get("host").and_then(|v| v.as_string()) {
                info.host = h.to_string();
            }
            if let Some(u) = dict.get("user").and_then(|v| v.as_string()) {
                info.user = u.to_string();
            }
            if let Some(d) = dict.get("database").and_then(|v| v.as_string()) {
                info.database = Some(d.to_string());
            }
            let node = ace_core::FavoriteNode::connection(info);
            if let Some(pid) = parent_group {
                if let Some(ace_core::FavoriteNode::Group { children, .. }) = tree.find_mut(pid) {
                    children.push(node);
                }
            } else {
                tree.add_root(node);
            }
        }
    }

    if let Some(arr) = value.as_array() {
        for item in arr {
            walk(item, &mut tree, None);
        }
    } else {
        walk(&value, &mut tree, None);
    }
    Ok(tree)
}