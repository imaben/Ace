use crate::views::*;
use ace_core::{AppSettings, ConnectionInfo};
use ace_driver::meta::DatabaseObject;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppScreen {
    #[default]
    Connection,
    Workspace,
    Preferences,
}

#[derive(Debug, Clone)]
pub struct SessionTab {
    pub id: Uuid,
    pub title: String,
    pub connection: ConnectionInfo,
    pub databases: Vec<String>,
    pub current_database: Option<String>,
    pub objects: Vec<DatabaseObject>,
    pub object_filter: String,
    pub selected_table: Option<String>,
    pub main_tab: MainViewTab,
    pub content: ContentViewState,
    pub structure: StructureViewState,
    pub indexes: IndexesViewState,
    pub relations: RelationsViewState,
    pub triggers: TriggersViewState,
    pub table_info: TableInfoViewState,
    pub query: QueryWorkspaceState,
    pub console: ConsoleViewState,
    pub processes: ProcessListState,
    pub variables: ServerVariablesState,
    pub users: UserManagerState,
    pub navigator: NavigatorState,
    pub status: String,
}

impl SessionTab {
    pub fn new(connection: ConnectionInfo) -> Self {
        let title = connection.name.clone();
        Self {
            id: Uuid::new_v4(),
            title,
            connection,
            databases: Vec::new(),
            current_database: None,
            objects: Vec::new(),
            object_filter: String::new(),
            selected_table: None,
            main_tab: MainViewTab::Content,
            content: ContentViewState::new(1000),
            structure: StructureViewState::default(),
            indexes: IndexesViewState::default(),
            relations: RelationsViewState::default(),
            triggers: TriggersViewState::default(),
            table_info: TableInfoViewState::default(),
            query: QueryWorkspaceState::default(),
            console: ConsoleViewState::default(),
            processes: ProcessListState::default(),
            variables: ServerVariablesState::default(),
            users: UserManagerState::default(),
            navigator: NavigatorState::default(),
            status: "Connected".into(),
        }
    }

    pub fn filtered_objects(&self) -> Vec<&DatabaseObject> {
        let q = self.object_filter.to_lowercase();
        self.objects
            .iter()
            .filter(|o| q.is_empty() || o.name.to_lowercase().contains(&q))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub screen: AppScreen,
    pub connection_form: ConnectionFormState,
    pub favorites: FavoritesViewState,
    pub sessions: Vec<SessionTab>,
    pub active_session: usize,
    pub preferences: PreferencesState,
    pub settings: AppSettings,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            screen: AppScreen::Connection,
            connection_form: ConnectionFormState::default(),
            favorites: FavoritesViewState::default(),
            sessions: Vec::new(),
            active_session: 0,
            preferences: PreferencesState::default(),
            settings: AppSettings::default(),
        }
    }
}

impl AppState {
    pub fn active_session(&self) -> Option<&SessionTab> {
        self.sessions.get(self.active_session)
    }

    pub fn active_session_mut(&mut self) -> Option<&mut SessionTab> {
        self.sessions.get_mut(self.active_session)
    }

    pub fn open_session(&mut self, connection: ConnectionInfo) {
        self.sessions.push(SessionTab::new(connection));
        self.active_session = self.sessions.len() - 1;
        self.screen = AppScreen::Workspace;
    }

    pub fn close_session(&mut self, index: usize) {
        if index < self.sessions.len() {
            self.sessions.remove(index);
            if self.sessions.is_empty() {
                self.screen = AppScreen::Connection;
                self.active_session = 0;
            } else {
                self.active_session = self.active_session.min(self.sessions.len() - 1);
            }
        }
    }
}