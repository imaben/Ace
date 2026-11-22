//! Root `AceApp` controller: multi-tab state, keyboard shortcuts, async DB actions,
//! Keychain password persistence, and GPUI window bootstrap.

use crate::gui::console::ConsoleLog;
use crate::gui::data_grid::{AceGridDelegate, GridCell};
use crate::gui::runtime::{
    connect_session, demo_connection, exec_single_sql, expand_home, export_data,
    fetch_database_columns, fetch_structure, fetch_table_page, import_sql_file, index_rows,
    query_result_to_cells, relation_rows, run_sql_multi, structure_column_rows, test_connection,
    tokio_rt, trigger_rows,
};
use crate::gui::sql_complete::{ColumnHint, SchemaCatalog, SqlCompletionProvider};
use crate::gui::sql_highlight::sql_highlighter_factory;
use crate::gui::state::{
    filter_op_by_label, sorted_variables, ConfirmAction, ConnectionDraft, ConnectionFormInputs,
    ModalState, QueryDrawerTab, ServerToolsTab, WorkspaceSession, WorkspaceTab, DEFAULT_PAGE_SIZE,
    FILTER_OPS,
};
use crate::gui::theme::{apply_theme, Palette};
use crate::views::MainViewTab;
use ace_core::{
    validate_connection, AceResult, ConnectionInfo, ConnectionMethod, EngineKind, FavoriteNode,
    FavoritesTree, SshOptions, SslOptions, TimeZoneMode, VaultOptions,
};
use ace_driver::meta::{ColumnMeta, TableMeta, TriggerMeta};
use ace_io::ExportFormat;
use ace_sql::{FilterNode, FilterOp};
use ace_store::{
    delete_password, export_favorites_plist, import_favorites_plist, load_favorites, load_password,
    save_favorites, save_password, HistoryEntry, HistoryStore, QueryFavorite,
};
use gpui_kit::component::input::{
    EditorState, InputEvent, InputState, ShowCompletions, TextareaState,
};
use gpui_kit::component::select::{SearchableVec, SelectEvent, SelectState};
use gpui_kit::component::table::{ColumnSort, TableEvent, TableState};
use gpui_kit::component::{v_flex, IndexPath, TitleBar};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;
use uuid::Uuid;

actions!(
    ace_app,
    [
        TabStructure,
        TabContent,
        TabRelations,
        TabTriggers,
        TabTableInfo,
        TabQuery,
        RunQueryAll,
        RunQueryCurrent,
        NewWindowTab,
        CloseWindowTab,
        ToggleConsole,
        DismissModal,
    ]
);

pub struct AceApp {
    pub(crate) tabs: Vec<WorkspaceTab>,
    pub(crate) active_tab: usize,
    pub(crate) favorites: FavoritesTree,
    pub(crate) query_history: Vec<HistoryEntry>,
    pub(crate) query_favorites: Vec<QueryFavorite>,
    pub(crate) pinned_tables: Vec<(String, String)>,
    pub(crate) modal: Option<ModalState>,
    pub(crate) dark_mode: bool,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

fn new_input(
    window: &mut Window,
    cx: &mut Context<AceApp>,
    value: impl Into<SharedString>,
    placeholder: impl Into<SharedString>,
    masked: bool,
) -> Entity<InputState> {
    let value = value.into();
    let placeholder = placeholder.into();
    cx.new(|cx| {
        let mut s = InputState::new(window, cx)
            .default_value(value)
            .placeholder(placeholder);
        if masked {
            s = s.masked(true);
        }
        s
    })
}

fn new_textarea(
    window: &mut Window,
    cx: &mut Context<AceApp>,
    value: impl Into<SharedString>,
    placeholder: impl Into<SharedString>,
) -> Entity<TextareaState> {
    let value = value.into();
    let placeholder = placeholder.into();
    cx.new(|cx| {
        TextareaState::new(window, cx)
            .default_value(value)
            .placeholder(placeholder)
    })
}

fn set_input_val(
    entity: &Entity<InputState>,
    value: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut Context<AceApp>,
) {
    let value = value.into();
    entity.update(cx, |s, cx| s.set_value(value, window, cx));
}

fn read_input_val(entity: &Entity<InputState>, cx: &App) -> String {
    entity.read(cx).value().to_string()
}

fn create_draft(
    window: &mut Window,
    cx: &mut Context<AceApp>,
    subs: &mut Vec<Subscription>,
) -> ConnectionDraft {
    let inputs = ConnectionFormInputs {
        name: new_input(window, cx, "", "Optional Name", false),
        host: new_input(window, cx, "127.0.0.1", "127.0.0.1", false),
        port: new_input(window, cx, "3306", "3306", false),
        user: new_input(window, cx, "root", "root", false),
        password: new_input(window, cx, "", "Password", true),
        database: new_input(window, cx, "", "Optional", false),
        socket: new_input(window, cx, "", "/tmp/mysql.sock", false),
        ssh_host: new_input(window, cx, "", "SSH Host", false),
        ssh_user: new_input(window, cx, "", "SSH User", false),
        ssh_password: new_input(window, cx, "", "SSH Password", true),
        ssh_port: new_input(window, cx, "22", "22", false),
        ssh_key: new_input(window, cx, "", "~/.ssh/id_ed25519", false),
        ssh_remote_socket: new_input(window, cx, "", "Optional", false),
        aws_profile: new_input(window, cx, "default", "default", false),
        aws_region: new_input(window, cx, "", "us-east-1", false),
        vault_host: new_input(window, cx, "", "vault.example.com", false),
        vault_port: new_input(window, cx, "443", "443", false),
        vault_oidc: new_input(window, cx, "oidc", "oidc", false),
        vault_mount: new_input(window, cx, "databases_credentials", "mount", false),
        vault_role: new_input(window, cx, "", "role", false),
        ssl_key: new_input(window, cx, "", "Optional client key", false),
        ssl_cert: new_input(window, cx, "", "Optional client cert", false),
        ssl_ca: new_input(window, cx, "", "Optional CA cert", false),
        timezone_fixed: new_input(window, cx, "UTC", "UTC / +08:00", false),
        favorites_search: new_input(window, cx, "", "Search Favorites", false),
    };

    // Live filter favorites & live tab title updates
    for live_in in [&inputs.favorites_search, &inputs.name] {
        subs.push(cx.subscribe_in(live_in, window, |_this, _, ev: &InputEvent, _w, cx| {
            if matches!(ev, InputEvent::Change) {
                cx.notify();
            }
        }));
    }

    // Enter in connection fields triggers Connect immediately (Sequel Ace UX)
    for submit_in in [
        &inputs.host,
        &inputs.user,
        &inputs.password,
        &inputs.database,
        &inputs.port,
        &inputs.socket,
    ] {
        subs.push(cx.subscribe_in(
            submit_in,
            window,
            |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.connect_active_tab(window, cx);
                }
            },
        ));
    }

    ConnectionDraft {
        selected_favorite: None,
        selected_group: None,
        engine: EngineKind::Mysql,
        method: ConnectionMethod::Tcp,
        color_index: -1,
        timezone: TimeZoneMode::Server,
        use_ssl: false,
        verify_server_cert: false,
        request_rsa_key: false,
        allow_data_local_infile: false,
        enable_cleartext_plugin: false,
        use_compression: true,
        form_error: None,
        test_status: None,
        connecting: false,
        testing: false,
        inputs,
        retained_password: None,
    }
}

impl AceApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dark_mode = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        apply_theme(dark_mode, Some(window), cx);

        let mut favorites = load_favorites().unwrap_or_default();
        if favorites.roots.is_empty() {
            let mut local_mysql = ConnectionInfo::mysql_local();
            local_mysql.name = "Local MySQL".into();
            local_mysql.color_index = 4;

            let mut grp = FavoriteNode::group("Favorites");
            if let FavoriteNode::Group { children, .. } = &mut grp {
                children.push(FavoriteNode::connection(local_mysql));
                children.push(FavoriteNode::connection(demo_connection()));
            }
            favorites.add_root(grp);
            let _ = save_favorites(&favorites);
        }

        let history = HistoryStore::open().ok();
        let query_history = history
            .as_ref()
            .and_then(|h| h.recent(50).ok())
            .unwrap_or_default();
        let query_favorites = history
            .as_ref()
            .and_then(|h| h.list_favorites().ok())
            .unwrap_or_default();
        let pinned_tables = history
            .as_ref()
            .and_then(|h| h.pinned_tables().ok())
            .unwrap_or_default();

        let mut subs = Vec::new();
        let first_tab = WorkspaceTab {
            id: Uuid::new_v4(),
            draft: create_draft(window, cx, &mut subs),
            session: None,
        };

        let focus_handle = cx.focus_handle();

        Self {
            tabs: vec![first_tab],
            active_tab: 0,
            favorites,
            query_history,
            query_favorites,
            pinned_tables,
            modal: None,
            dark_mode,
            focus_handle,
            _subscriptions: subs,
        }
    }

    pub(crate) fn active_tab(&self) -> &WorkspaceTab {
        &self.tabs[self.active_tab.min(self.tabs.len() - 1)]
    }

    pub(crate) fn active_tab_mut(&mut self) -> &mut WorkspaceTab {
        let idx = self.active_tab.min(self.tabs.len() - 1);
        &mut self.tabs[idx]
    }

    // ─── Theme & Window Tabs ────────────────────────────────────────────────

    pub(crate) fn toggle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dark_mode = !self.dark_mode;
        apply_theme(self.dark_mode, Some(window), cx);
        cx.notify();
    }

    pub(crate) fn new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut subs = Vec::new();
        let draft = create_draft(window, cx, &mut subs);
        self._subscriptions.extend(subs);
        let tab = WorkspaceTab {
            id: Uuid::new_v4(),
            draft,
            session: None,
        };
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        cx.notify();
    }

    pub(crate) fn switch_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.tabs.len() {
            self.active_tab = idx;
            cx.notify();
        }
    }

    pub(crate) fn close_tab(&mut self, idx: usize, _window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 || idx >= self.tabs.len() {
            return;
        }
        let removed = self.tabs.remove(idx);
        if let Some(session) = removed.session {
            let conn = session.conn;
            tokio_rt().spawn(async move {
                let _ = conn.close().await;
            });
        }
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        cx.notify();
    }

    // ─── Connection Form & Favorites ────────────────────────────────────────

    pub(crate) fn set_engine(
        &mut self,
        engine: EngineKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let port_input = self.active_tab().draft.inputs.port.clone();
        let host_input = self.active_tab().draft.inputs.host.clone();
        let user_input = self.active_tab().draft.inputs.user.clone();
        {
            let d = &mut self.active_tab_mut().draft;
            d.engine = engine;
            d.form_error = None;
            d.test_status = None;
            if engine == EngineKind::Sqlite {
                d.method = ConnectionMethod::Tcp;
            }
        }
        let default_port = ConnectionInfo::default_port_for_engine(engine);
        if default_port > 0 {
            set_input_val(&port_input, default_port.to_string(), window, cx);
            if read_input_val(&host_input, cx).trim().is_empty() {
                set_input_val(&host_input, "127.0.0.1", window, cx);
            }
            if engine == EngineKind::Postgres && read_input_val(&user_input, cx) == "root" {
                set_input_val(&user_input, "postgres", window, cx);
            } else if matches!(engine, EngineKind::Mysql | EngineKind::MariaDb)
                && read_input_val(&user_input, cx) == "postgres"
            {
                set_input_val(&user_input, "root", window, cx);
            }
        }
        cx.notify();
    }

    fn build_form_info(&self, cx: &App) -> ConnectionInfo {
        let d = &self.active_tab().draft;
        let mut info = ConnectionInfo::default();
        if let Some(fav_id) = d.selected_favorite {
            info.id = fav_id;
        }
        info.name = read_input_val(&d.inputs.name, cx);
        info.engine = d.engine;
        info.method = d.method;
        info.host = read_input_val(&d.inputs.host, cx);
        info.port = read_input_val(&d.inputs.port, cx)
            .parse()
            .unwrap_or(ConnectionInfo::default_port_for_engine(d.engine));
        info.user = read_input_val(&d.inputs.user, cx);

        let typed_pwd = read_input_val(&d.inputs.password, cx);
        info.password = if !typed_pwd.is_empty() {
            Some(typed_pwd)
        } else {
            d.retained_password.clone()
        };

        let db = read_input_val(&d.inputs.database, cx);
        info.database = if db.trim().is_empty() { None } else { Some(db) };

        let sock = read_input_val(&d.inputs.socket, cx);
        info.socket_path = if sock.trim().is_empty() {
            None
        } else {
            Some(sock)
        };

        info.color_index = d.color_index;
        info.timezone = match &d.timezone {
            TimeZoneMode::Fixed(_) => {
                TimeZoneMode::Fixed(read_input_val(&d.inputs.timezone_fixed, cx))
            }
            other => other.clone(),
        };
        info.use_compression = d.use_compression;
        info.allow_data_local_infile = d.allow_data_local_infile;
        info.enable_cleartext_plugin = d.enable_cleartext_plugin;

        let mut ssl = SslOptions {
            enabled: d.use_ssl || d.method == ConnectionMethod::AwsIam,
            request_rsa_key: d.request_rsa_key,
            verify_server_cert: d.verify_server_cert,
            ..Default::default()
        };
        let ssl_key = read_input_val(&d.inputs.ssl_key, cx);
        let ssl_cert = read_input_val(&d.inputs.ssl_cert, cx);
        let ssl_ca = read_input_val(&d.inputs.ssl_ca, cx);
        if !ssl_key.trim().is_empty() {
            ssl.client_key_path = Some(ssl_key);
        }
        if !ssl_cert.trim().is_empty() {
            ssl.client_cert_path = Some(ssl_cert);
        }
        if !ssl_ca.trim().is_empty() {
            ssl.ca_cert_path = Some(ssl_ca);
        }
        info.ssl = ssl;

        if d.method == ConnectionMethod::SshTunnel {
            let key = read_input_val(&d.inputs.ssh_key, cx);
            let remote = read_input_val(&d.inputs.ssh_remote_socket, cx);
            let ssh_pwd = read_input_val(&d.inputs.ssh_password, cx);
            info.ssh = Some(SshOptions {
                host: read_input_val(&d.inputs.ssh_host, cx),
                port: read_input_val(&d.inputs.ssh_port, cx)
                    .parse()
                    .unwrap_or(22),
                user: read_input_val(&d.inputs.ssh_user, cx),
                password: if ssh_pwd.is_empty() {
                    None
                } else {
                    Some(ssh_pwd)
                },
                private_key_path: if key.trim().is_empty() {
                    None
                } else {
                    Some(expand_home(key.trim()))
                },
                passphrase: None,
                use_ssh_config: true,
                remote_socket_path: if remote.trim().is_empty() {
                    None
                } else {
                    Some(remote)
                },
            });
        }

        if d.method == ConnectionMethod::AwsIam {
            info.aws.enabled = true;
            let profile = read_input_val(&d.inputs.aws_profile, cx);
            let region = read_input_val(&d.inputs.aws_region, cx);
            info.aws.profile = if profile.trim().is_empty() {
                Some("default".into())
            } else {
                Some(profile)
            };
            info.aws.region = if region.trim().is_empty() {
                None
            } else {
                Some(region)
            };
        }

        if d.method == ConnectionMethod::Vault {
            let vhost = read_input_val(&d.inputs.vault_host, cx);
            let vport = read_input_val(&d.inputs.vault_port, cx)
                .parse()
                .unwrap_or(443);
            let oidc = read_input_val(&d.inputs.vault_oidc, cx);
            let mount = read_input_val(&d.inputs.vault_mount, cx);
            let role = read_input_val(&d.inputs.vault_role, cx);
            info.vault = VaultOptions {
                enabled: true,
                host: if vhost.trim().is_empty() {
                    None
                } else {
                    Some(vhost)
                },
                port: vport,
                oidc_mount: if oidc.trim().is_empty() {
                    None
                } else {
                    Some(oidc)
                },
                mount_path: if mount.trim().is_empty() {
                    None
                } else {
                    Some(mount)
                },
                role: if role.trim().is_empty() {
                    None
                } else {
                    Some(role)
                },
                credentials_path: None,
            };
        }

        if d.engine == EngineKind::Sqlite {
            info.host.clear();
            info.port = 0;
            info.method = ConnectionMethod::Tcp;
        }

        info.sanitize_inputs();
        info
    }

    pub(crate) fn load_favorite(
        &mut self,
        info: &ConnectionInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Retrieve saved passwords from local credential store (falling back to in-struct password)
        let saved_pwd = load_password(&info.id.to_string()).ok().flatten();
        let resolved_pwd = saved_pwd.or_else(|| info.password.clone());
        let saved_ssh_pwd = load_password(&format!("{}:ssh", info.id)).ok().flatten();

        let (
            name_in,
            host_in,
            port_in,
            user_in,
            pwd_in,
            db_in,
            sock_in,
            ssh_host_in,
            ssh_user_in,
            ssh_port_in,
            ssh_key_in,
            ssh_remote_in,
            ssh_pwd_in,
            aws_prof_in,
            aws_reg_in,
            v_host_in,
            v_port_in,
            v_oidc_in,
            v_mount_in,
            v_role_in,
            ssl_key_in,
            ssl_cert_in,
            ssl_ca_in,
            tz_in,
        ) = {
            let d = &mut self.active_tab_mut().draft;
            d.selected_favorite = Some(info.id);
            d.selected_group = None;
            d.engine = info.engine;
            d.method = info.method;
            d.color_index = info.color_index;
            d.timezone = info.timezone.clone();
            d.use_ssl = info.ssl.enabled;
            d.verify_server_cert = info.ssl.verify_server_cert;
            d.request_rsa_key = info.ssl.request_rsa_key;
            d.allow_data_local_infile = info.allow_data_local_infile;
            d.enable_cleartext_plugin = info.enable_cleartext_plugin;
            d.use_compression = info.use_compression;
            d.retained_password = resolved_pwd.clone();
            d.form_error = None;
            d.test_status = None;
            (
                d.inputs.name.clone(),
                d.inputs.host.clone(),
                d.inputs.port.clone(),
                d.inputs.user.clone(),
                d.inputs.password.clone(),
                d.inputs.database.clone(),
                d.inputs.socket.clone(),
                d.inputs.ssh_host.clone(),
                d.inputs.ssh_user.clone(),
                d.inputs.ssh_port.clone(),
                d.inputs.ssh_key.clone(),
                d.inputs.ssh_remote_socket.clone(),
                d.inputs.ssh_password.clone(),
                d.inputs.aws_profile.clone(),
                d.inputs.aws_region.clone(),
                d.inputs.vault_host.clone(),
                d.inputs.vault_port.clone(),
                d.inputs.vault_oidc.clone(),
                d.inputs.vault_mount.clone(),
                d.inputs.vault_role.clone(),
                d.inputs.ssl_key.clone(),
                d.inputs.ssl_cert.clone(),
                d.inputs.ssl_ca.clone(),
                d.inputs.timezone_fixed.clone(),
            )
        };

        set_input_val(&name_in, info.name.clone(), window, cx);
        set_input_val(&host_in, info.host.clone(), window, cx);
        set_input_val(
            &port_in,
            if info.port == 0 {
                ConnectionInfo::default_port_for_engine(info.engine).to_string()
            } else {
                info.port.to_string()
            },
            window,
            cx,
        );
        set_input_val(&user_in, info.user.clone(), window, cx);
        set_input_val(&pwd_in, resolved_pwd.unwrap_or_default(), window, cx);
        set_input_val(&db_in, info.database.clone().unwrap_or_default(), window, cx);
        set_input_val(
            &sock_in,
            info.socket_path.clone().unwrap_or_default(),
            window,
            cx,
        );

        if let Some(ssh) = &info.ssh {
            set_input_val(&ssh_host_in, ssh.host.clone(), window, cx);
            set_input_val(&ssh_user_in, ssh.user.clone(), window, cx);
            set_input_val(&ssh_port_in, ssh.port.to_string(), window, cx);
            set_input_val(
                &ssh_key_in,
                ssh.private_key_path.clone().unwrap_or_default(),
                window,
                cx,
            );
            set_input_val(
                &ssh_remote_in,
                ssh.remote_socket_path.clone().unwrap_or_default(),
                window,
                cx,
            );
            set_input_val(
                &ssh_pwd_in,
                saved_ssh_pwd
                    .or_else(|| ssh.password.clone())
                    .unwrap_or_default(),
                window,
                cx,
            );
        }

        set_input_val(
            &aws_prof_in,
            info.aws.profile.clone().unwrap_or_else(|| "default".into()),
            window,
            cx,
        );
        set_input_val(
            &aws_reg_in,
            info.aws.region.clone().unwrap_or_default(),
            window,
            cx,
        );
        set_input_val(
            &v_host_in,
            info.vault.host.clone().unwrap_or_default(),
            window,
            cx,
        );
        set_input_val(&v_port_in, info.vault.port.to_string(), window, cx);
        set_input_val(
            &v_oidc_in,
            info.vault
                .oidc_mount
                .clone()
                .unwrap_or_else(|| "oidc".into()),
            window,
            cx,
        );
        set_input_val(
            &v_mount_in,
            info.vault
                .mount_path
                .clone()
                .unwrap_or_else(|| "databases_credentials".into()),
            window,
            cx,
        );
        set_input_val(
            &v_role_in,
            info.vault.role.clone().unwrap_or_default(),
            window,
            cx,
        );
        set_input_val(
            &ssl_key_in,
            info.ssl.client_key_path.clone().unwrap_or_default(),
            window,
            cx,
        );
        set_input_val(
            &ssl_cert_in,
            info.ssl.client_cert_path.clone().unwrap_or_default(),
            window,
            cx,
        );
        set_input_val(
            &ssl_ca_in,
            info.ssl.ca_cert_path.clone().unwrap_or_default(),
            window,
            cx,
        );

        if let TimeZoneMode::Fixed(id) = &info.timezone {
            set_input_val(&tz_in, id.clone(), window, cx);
        }

        cx.notify();
    }

    pub(crate) fn quick_connect_blank(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let blank = ConnectionInfo::mysql_local();
        self.load_favorite(&blank, window, cx);
        let d = &mut self.active_tab_mut().draft;
        d.selected_favorite = None;
        d.selected_group = None;
        d.retained_password = None;
        let name_in = d.inputs.name.clone();
        set_input_val(&name_in, "", window, cx);
        cx.notify();
    }

    pub(crate) fn toggle_favorite_group(&mut self, group_id: Uuid, cx: &mut Context<Self>) {
        if let Some(FavoriteNode::Group { expanded, .. }) = self.favorites.find_mut(group_id) {
            *expanded = !*expanded;
        }
        let d = &mut self.active_tab_mut().draft;
        d.selected_group = Some(group_id);
        d.selected_favorite = None;
        let _ = save_favorites(&self.favorites);
        cx.notify();
    }

    pub(crate) fn save_current_favorite(&mut self, cx: &mut Context<Self>) {
        let mut info = self.build_form_info(cx);
        if info.name.trim().is_empty() {
            info.name = if info.engine == EngineKind::Sqlite {
                "SQLite DB".into()
            } else {
                format!("{}@{}", info.user, info.display_host())
            };
        }
        let secret_pwd = info.password.clone();
        let secret_ssh_pwd = info.ssh.as_ref().and_then(|s| s.password.clone());
        let selected_fav = self.active_tab().draft.selected_favorite;
        let selected_group = self.active_tab().draft.selected_group;

        let target_id = if let Some(id) = selected_fav {
            if let Some(FavoriteNode::Connection { info: existing }) = self.favorites.find_mut(id) {
                info.id = existing.id;
                let mut stored = info.without_secrets();
                stored.keychain_id = Some(existing.id.to_string());
                *existing = stored;
                existing.id
            } else {
                info.id = Uuid::new_v4();
                let mut stored = info.without_secrets();
                stored.keychain_id = Some(info.id.to_string());
                self.insert_favorite_node(selected_group, FavoriteNode::connection(stored));
                info.id
            }
        } else {
            info.id = Uuid::new_v4();
            let mut stored = info.without_secrets();
            stored.keychain_id = Some(info.id.to_string());
            self.insert_favorite_node(selected_group, FavoriteNode::connection(stored));
            info.id
        };

        if let Some(pwd) = &secret_pwd {
            if !pwd.is_empty() {
                let _ = save_password(&target_id.to_string(), pwd);
            }
        }
        if let Some(spwd) = &secret_ssh_pwd {
            if !spwd.is_empty() {
                let _ = save_password(&format!("{target_id}:ssh"), spwd);
            }
        }
        let d = &mut self.active_tab_mut().draft;
        d.selected_favorite = Some(target_id);
        d.retained_password = secret_pwd;
        d.test_status = Some((true, "Favorite saved".into()));
        let _ = save_favorites(&self.favorites);
        cx.notify();
    }

    fn insert_favorite_node(&mut self, group_id: Option<Uuid>, node: FavoriteNode) {
        if let Some(gid) = group_id {
            if let Some(FavoriteNode::Group {
                children, expanded, ..
            }) = self.favorites.find_mut(gid)
            {
                *expanded = true;
                children.push(node);
                return;
            }
        }
        if let Some(FavoriteNode::Group { children, .. }) = self.favorites.roots.first_mut() {
            children.push(node);
        } else {
            self.favorites.add_root(node);
        }
    }

    pub(crate) fn duplicate_selected_favorite(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(id) = self.active_tab().draft.selected_favorite {
            if let Some(new_id) = self.favorites.duplicate_connection(id) {
                let _ = save_favorites(&self.favorites);
                if let Some(info) = self
                    .favorites
                    .all_connections()
                    .into_iter()
                    .find(|c| c.id == new_id)
                    .cloned()
                {
                    self.load_favorite(&info, window, cx);
                }
            }
        }
    }

    pub(crate) fn request_delete_selected_favorite(&mut self, cx: &mut Context<Self>) {
        let target = self
            .active_tab()
            .draft
            .selected_favorite
            .or(self.active_tab().draft.selected_group);
        if let Some(id) = target {
            self.modal = Some(ModalState::Confirm {
                title: "Delete Favorite".into(),
                message: "Remove the selected favorite (or group) from your list? This cannot be undone.".into(),
                confirm_label: "Delete".into(),
                action: ConfirmAction::DeleteFavorite(id),
            });
            cx.notify();
        }
    }

    pub(crate) fn open_new_group_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal = Some(ModalState::NewGroup {
            name: new_input(window, cx, "", "Group Name", false),
        });
        cx.notify();
    }

    pub(crate) fn commit_new_group(&mut self, cx: &mut Context<Self>) {
        if let Some(ModalState::NewGroup { name }) = self.modal.take() {
            let group_name = read_input_val(&name, cx);
            let group_name = group_name.trim();
            if !group_name.is_empty() {
                self.favorites.add_root(FavoriteNode::group(group_name));
                let _ = save_favorites(&self.favorites);
            }
            cx.notify();
        }
    }

    pub(crate) fn open_favorites_plist_modal(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.modal = Some(ModalState::FavoritesPlist {
            path: new_input(
                window,
                cx,
                expand_home("~/Documents/AceFavorites.plist"),
                "~/Documents/AceFavorites.plist",
                false,
            ),
            status: None,
        });
        cx.notify();
    }

    pub(crate) fn commit_export_plist(&mut self, cx: &mut Context<Self>) {
        let path_str = match &self.modal {
            Some(ModalState::FavoritesPlist { path, .. }) => expand_home(&read_input_val(path, cx)),
            _ => return,
        };
        let msg = match export_favorites_plist(&self.favorites, std::path::Path::new(&path_str)) {
            Ok(()) => format!("Exported favorites to {path_str}"),
            Err(e) => format!("Export failed: {e}"),
        };
        if let Some(ModalState::FavoritesPlist { status, .. }) = self.modal.as_mut() {
            *status = Some(msg);
        }
        cx.notify();
    }

    pub(crate) fn commit_import_plist(&mut self, cx: &mut Context<Self>) {
        let path_str = match &self.modal {
            Some(ModalState::FavoritesPlist { path, .. }) => expand_home(&read_input_val(path, cx)),
            _ => return,
        };
        let msg = match import_favorites_plist(std::path::Path::new(&path_str)) {
            Ok(tree) => {
                let count = tree.all_connections().len();
                for root in tree.roots {
                    self.favorites.add_root(root);
                }
                let _ = save_favorites(&self.favorites);
                format!("Imported {count} connections from {path_str}")
            }
            Err(e) => format!("Import failed: {e}"),
        };
        if let Some(ModalState::FavoritesPlist { status, .. }) = self.modal.as_mut() {
            *status = Some(msg);
        }
        cx.notify();
    }

    // ─── Test Connection & Connect ──────────────────────────────────────────

    pub(crate) fn test_active_connection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_tab().draft.testing {
            return;
        }
        let info = self.build_form_info(cx);
        if let Err(e) = validate_connection(&info) {
            self.active_tab_mut().draft.form_error = Some(e);
            cx.notify();
            return;
        }
        let tab_id = self.active_tab().id;
        {
            let d = &mut self.active_tab_mut().draft;
            d.testing = true;
            d.form_error = None;
            d.test_status = None;
        }
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move { test_connection(&info).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Connection(e.to_string())));

            this.update_in(cx, |this, _window, cx| {
                if let Some(tab) = this.tabs.iter_mut().find(|t| t.id == tab_id) {
                    tab.draft.testing = false;
                    match res {
                        Ok(msg) => tab.draft.test_status = Some((true, format!("Connected — {msg}"))),
                        Err(e) => tab.draft.test_status = Some((false, e.to_string())),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn connect_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_tab().draft.connecting {
            return;
        }
        let info = self.build_form_info(cx);
        if let Err(e) = validate_connection(&info) {
            self.active_tab_mut().draft.form_error = Some(e);
            cx.notify();
            return;
        }
        let tab_id = self.active_tab().id;
        {
            let d = &mut self.active_tab_mut().draft;
            d.connecting = true;
            d.form_error = None;
            d.test_status = None;
        }
        cx.notify();

        let console = ConsoleLog::new();
        let console_for_bg = console.clone();
        let info_for_bg = info.clone();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move { connect_session(&info_for_bg, console_for_bg).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Connection(e.to_string())));

            this.update_in(cx, |this, window, cx| {
                match res {
                    Ok(bundle) => {
                        // Persist updated password if connecting via an existing saved favorite
                        if let Some(tab) = this.tabs.iter().find(|t| t.id == tab_id) {
                            if let Some(fav_id) = tab.draft.selected_favorite {
                                if let Some(pwd) = info.password.as_deref().filter(|s| !s.is_empty()) {
                                    let _ = save_password(&fav_id.to_string(), pwd);
                                }
                                if let Some(spwd) = info
                                    .ssh
                                    .as_ref()
                                    .and_then(|s| s.password.as_deref())
                                    .filter(|s| !s.is_empty())
                                {
                                    let _ = save_password(&format!("{fav_id}:ssh"), spwd);
                                }
                            }
                        }
                        let session = this.build_workspace_session(
                            tab_id,
                            info.clone(),
                            bundle,
                            console,
                            window,
                            cx,
                        );
                        if let Some(tab) = this.tabs.iter_mut().find(|t| t.id == tab_id) {
                            tab.draft.connecting = false;
                            tab.session = Some(session);
                        }
                    }
                    Err(e) => {
                        if let Some(tab) = this.tabs.iter_mut().find(|t| t.id == tab_id) {
                            tab.draft.connecting = false;
                            tab.draft.form_error = Some(e.to_string());
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn build_workspace_session(
        &mut self,
        tab_id: Uuid,
        info: ConnectionInfo,
        bundle: crate::gui::runtime::ConnectedBundle,
        console: ConsoleLog,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> WorkspaceSession {
        let db_items: Vec<SharedString> =
            bundle.databases.iter().cloned().map(Into::into).collect();
        let selected_db_ix = bundle
            .databases
            .iter()
            .position(|d| d == &bundle.current_db)
            .map(IndexPath::new);
        let db_select = cx.new(|cx| {
            SelectState::new(SearchableVec::new(db_items), selected_db_ix, window, cx)
                .searchable(true)
        });
        let db_sub = cx.subscribe_in(
            &db_select,
            window,
            move |this, _, ev: &SelectEvent<SearchableVec<SharedString>>, window, cx| {
                if let SelectEvent::Confirm(Some(db_name)) = ev {
                    this.switch_database(db_name.to_string(), window, cx);
                }
            },
        );
        self._subscriptions.push(db_sub);

        let filter_cols: Vec<SharedString> = vec!["Custom WHERE".into()];
        let filter_col_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(filter_cols),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
            .searchable(true)
        });
        let filter_ops: Vec<SharedString> = FILTER_OPS.iter().map(|(l, _)| (*l).into()).collect();
        let filter_op_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(filter_ops),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });

        let content_filter_value = new_input(
            window,
            cx,
            "",
            "Value (or raw WHERE clause when Custom WHERE selected)…",
            false,
        );
        let filter_enter_sub = cx.subscribe_in(
            &content_filter_value,
            window,
            |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.apply_content_filter(window, cx);
                }
            },
        );
        self._subscriptions.push(filter_enter_sub);

        let cell_edit_input = new_input(window, cx, "", "Edit selected cell…", false);
        let cell_enter_sub = cx.subscribe_in(
            &cell_edit_input,
            window,
            |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.save_inline_cell_edit(false, window, cx);
                }
            },
        );
        self._subscriptions.push(cell_enter_sub);

        let weak_self = cx.entity().downgrade();
        let on_sort: crate::gui::data_grid::SortHandler = Rc::new(move |col, sort, window, cx| {
            let _ = weak_self.update(cx, |this, cx| {
                this.apply_content_sort(col, sort, window, cx);
            });
        });

        let content_table = cx.new(|cx| {
            TableState::new(
                AceGridDelegate::new("No rows", true).with_on_sort(on_sort),
                window,
                cx,
            )
            .cell_selectable(true)
            .row_selectable(true)
            .col_resizable(true)
            .sortable(true)
            .row_header(false)
        });

        let content_sub = cx.subscribe_in(
            &content_table,
            window,
            move |this, _, ev: &TableEvent, window, cx| match ev {
                TableEvent::SelectCell(r, c) => {
                    this.on_select_content_cell(*r, *c, window, cx);
                }
                TableEvent::DoubleClickedCell(r, c) => {
                    this.on_select_content_cell(*r, *c, window, cx);
                    this.open_selected_cell_editor(window, cx);
                }
                TableEvent::SelectRow(r) => {
                    this.on_select_content_cell(*r, 0, window, cx);
                }
                _ => {}
            },
        );
        self._subscriptions.push(content_sub);

        let structure_table = cx.new(|cx| {
            TableState::new(AceGridDelegate::new("No columns", false), window, cx)
                .row_selectable(true)
                .col_resizable(true)
        });
        let struct_sub = cx.subscribe_in(
            &structure_table,
            window,
            move |this, _, ev: &TableEvent, window, cx| match ev {
                TableEvent::SelectRow(r) => {
                    if let Some(s) = this.session_by_tab_mut(tab_id) {
                        s.selected_structure_row = Some(*r);
                    }
                    cx.notify();
                }
                TableEvent::DoubleClickedRow(r) => {
                    if let Some(s) = this.session_by_tab_mut(tab_id) {
                        s.selected_structure_row = Some(*r);
                    }
                    this.open_edit_selected_column_modal(window, cx);
                }
                _ => {}
            },
        );
        self._subscriptions.push(struct_sub);

        let index_table = cx.new(|cx| {
            TableState::new(AceGridDelegate::new("No indexes", false), window, cx)
                .row_selectable(true)
                .col_resizable(true)
        });
        let idx_sub = cx.subscribe_in(
            &index_table,
            window,
            move |this, _, ev: &TableEvent, _window, cx| {
                if let TableEvent::SelectRow(r) = ev {
                    if let Some(s) = this.session_by_tab_mut(tab_id) {
                        s.selected_index_row = Some(*r);
                    }
                    cx.notify();
                }
            },
        );
        self._subscriptions.push(idx_sub);

        let relation_table = cx.new(|cx| {
            TableState::new(AceGridDelegate::new("No foreign keys", false), window, cx)
                .row_selectable(true)
                .col_resizable(true)
        });
        let rel_sub = cx.subscribe_in(
            &relation_table,
            window,
            move |this, _, ev: &TableEvent, _window, cx| {
                if let TableEvent::SelectRow(r) = ev {
                    if let Some(s) = this.session_by_tab_mut(tab_id) {
                        s.selected_relation_row = Some(*r);
                    }
                    cx.notify();
                }
            },
        );
        self._subscriptions.push(rel_sub);

        let trigger_table = cx.new(|cx| {
            TableState::new(AceGridDelegate::new("No triggers", false), window, cx)
                .row_selectable(true)
                .col_resizable(true)
        });
        let trg_sub = cx.subscribe_in(
            &trigger_table,
            window,
            move |this, _, ev: &TableEvent, _window, cx| {
                if let TableEvent::SelectRow(r) = ev {
                    if let Some(s) = this.session_by_tab_mut(tab_id) {
                        s.selected_trigger_row = Some(*r);
                    }
                    cx.notify();
                }
            },
        );
        self._subscriptions.push(trg_sub);

        let query_result_table = cx.new(|cx| {
            TableState::new(
                AceGridDelegate::new("Run a query to see results", false),
                window,
                cx,
            )
            .cell_selectable(true)
            .row_selectable(true)
            .col_resizable(true)
            .row_header(false)
        });
        let qres_sub = cx.subscribe_in(
            &query_result_table,
            window,
            move |this, table_ent, ev: &TableEvent, _window, cx| {
                if let TableEvent::DoubleClickedCell(r, c) = ev {
                    if let Some(cell) = table_ent.read(cx).delegate().cell(*r, *c) {
                        cx.write_to_clipboard(ClipboardItem::new_string(cell.display.clone()));
                        if let Some(s) = this.session_by_tab_mut(tab_id) {
                            s.status = "Copied result cell to clipboard".into();
                        }
                        cx.notify();
                    }
                }
            },
        );
        self._subscriptions.push(qres_sub);

        let object_filter = new_input(window, cx, "", "Filter tables…", false);
        let console_filter = new_input(window, cx, "", "Filter console…", false);
        for live_filter in [&object_filter, &console_filter] {
            self._subscriptions.push(cx.subscribe_in(
                live_filter,
                window,
                |_this, _, ev: &InputEvent, _w, cx| {
                    if matches!(ev, InputEvent::Change) {
                        cx.notify();
                    }
                },
            ));
        }

        let schema_catalog = SchemaCatalog::new();
        schema_catalog.set_databases(bundle.databases.clone());
        schema_catalog.set_objects(bundle.current_db.clone(), &bundle.objects);

        // Silently load all column definitions for SQL autocomplete in the background
        {
            let raw_conn = bundle.raw_conn.clone();
            let engine = info.engine;
            let db_for_cols = bundle.current_db.clone();
            let tbl_names: Vec<String> = bundle.objects.iter().map(|o| o.name.clone()).collect();
            let catalog_bg = schema_catalog.clone();
            tokio_rt().spawn(async move {
                let cols = fetch_database_columns(&raw_conn, engine, &db_for_cols, &tbl_names).await;
                catalog_bg.set_all_columns(cols);
            });
        }

        let provider_catalog = schema_catalog.clone();
        let query_editor = cx.new(|cx| {
            let mut ed = EditorState::new(window, cx)
                .language("sql")
                .placeholder("Write SQL queries here… (Cmd+R to run current, Cmd+Shift+R to run all)");
            ed.set_highlighter_factory(sql_highlighter_factory(), cx);
            ed.lsp_mut().completion_provider = Some(Rc::new(SqlCompletionProvider::new(provider_catalog)));
            ed
        });
        let qed_sub = cx.subscribe_in(
            &query_editor,
            window,
            |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { secondary: true, .. }) {
                    this.run_custom_query(true, window, cx);
                }
            },
        );
        self._subscriptions.push(qed_sub);

        WorkspaceSession {
            info,
            conn: bundle.conn,
            raw_conn: bundle.raw_conn,
            schema_catalog,
            server_info: bundle.server_info,
            console,
            console_open: false,
            console_filter,
            databases: bundle.databases,
            current_db: bundle.current_db,
            db_select,
            objects: bundle.objects,
            object_filter,
            selected_table: None,
            main_tab: MainViewTab::Content,
            table_meta: None,
            triggers: Vec::new(),
            structure_table,
            index_table,
            relation_table,
            trigger_table,
            selected_structure_row: None,
            selected_index_row: None,
            selected_relation_row: None,
            selected_trigger_row: None,
            content_table,
            content_columns: Vec::new(),
            content_raw: None,
            row_count: None,
            page_offset: 0,
            page_size: DEFAULT_PAGE_SIZE,
            sort_column: None,
            sort_desc: false,
            where_clause: None,
            filter_col_select,
            filter_op_select,
            content_filter_value,
            selected_cell: None,
            cell_edit_input,
            table_comment_input: new_input(window, cx, "", "Table comment", false),
            query_editor,
            query_results: Vec::new(),
            active_result_idx: 0,
            query_result_table,
            query_message: String::new(),
            query_error: None,
            query_drawer: QueryDrawerTab::History,
            query_drawer_open: false,
            status: bundle.status,
            busy: false,
        }
    }

    fn session_by_tab_mut(&mut self, tab_id: Uuid) -> Option<&mut WorkspaceSession> {
        self.tabs
            .iter_mut()
            .find(|t| t.id == tab_id)
            .and_then(|t| t.session.as_mut())
    }

    pub(crate) fn disconnect_active_tab(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = self.active_tab_mut().session.take() {
            let conn = session.conn;
            tokio_rt().spawn(async move {
                let _ = conn.close().await;
            });
        }
        self.modal = None;
        cx.notify();
    }

    pub(crate) fn set_main_tab(&mut self, tab: MainViewTab, cx: &mut Context<Self>) {
        if let Some(s) = self.active_tab_mut().session.as_mut() {
            s.main_tab = tab;
        }
        cx.notify();
    }

    pub(crate) fn toggle_console(&mut self, cx: &mut Context<Self>) {
        if let Some(s) = self.active_tab_mut().session.as_mut() {
            s.console_open = !s.console_open;
        }
        cx.notify();
    }

    pub(crate) fn set_session_status(&mut self, msg: impl Into<String>, cx: &mut Context<Self>) {
        if let Some(s) = self.active_tab_mut().session.as_mut() {
            s.status = msg.into();
        }
        cx.notify();
    }

    // ─── Database & Table Navigation ────────────────────────────────────────

    pub(crate) fn switch_database(
        &mut self,
        database: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.active_tab().id;
        let Some(session) = self.active_tab_mut().session.as_mut() else {
            return;
        };
        let conn = session.conn.clone();
        session.busy = true;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move {
                    conn.use_database(&database).await?;
                    let objects = conn.list_objects(&database).await?;
                    AceResult::Ok((database, objects))
                })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));

            this.update_in(cx, |this, _window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    match res {
                        Ok((db, objects)) => {
                            let raw_conn = s.raw_conn.clone();
                            let engine = s.info.engine;
                            let catalog = s.schema_catalog.clone();
                            let tbl_names: Vec<String> =
                                objects.iter().map(|o| o.name.clone()).collect();
                            catalog.set_objects(db.clone(), &objects);
                            let db_bg = db.clone();
                            tokio_rt().spawn(async move {
                                let cols =
                                    fetch_database_columns(&raw_conn, engine, &db_bg, &tbl_names)
                                        .await;
                                catalog.set_all_columns(cols);
                            });

                            s.current_db = db.clone();
                            s.objects = objects;
                            s.selected_table = None;
                            s.table_meta = None;
                            s.triggers.clear();
                            s.selected_cell = None;
                            s.content_columns.clear();
                            s.content_raw = None;
                            s.content_table.update(cx, |t, cx| {
                                t.delegate_mut().clear();
                                t.refresh(cx);
                            });
                            s.structure_table.update(cx, |t, cx| {
                                t.delegate_mut().clear();
                                t.refresh(cx);
                            });
                            s.status = format!("Database: {db}");
                        }
                        Err(e) => s.status = format!("Switch DB error: {e}"),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn refresh_databases_and_tables(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.active_tab().id;
        let Some(session) = self.active_tab_mut().session.as_mut() else {
            return;
        };
        let conn = session.conn.clone();
        let current_db = session.current_db.clone();
        session.busy = true;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move {
                    let dbs = conn.list_databases().await?;
                    let objs = if current_db.is_empty() {
                        Vec::new()
                    } else {
                        conn.list_objects(&current_db).await.unwrap_or_default()
                    };
                    AceResult::Ok((dbs, objs))
                })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));

            this.update_in(cx, |this, window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    match res {
                        Ok((dbs, objs)) => {
                            let cur = s.current_db.clone();
                            s.schema_catalog.set_databases(dbs.clone());
                            s.schema_catalog.set_objects(cur.clone(), &objs);
                            let raw_conn = s.raw_conn.clone();
                            let engine = s.info.engine;
                            let catalog = s.schema_catalog.clone();
                            let tbl_names: Vec<String> =
                                objs.iter().map(|o| o.name.clone()).collect();
                            let db_bg = cur.clone();
                            tokio_rt().spawn(async move {
                                let cols =
                                    fetch_database_columns(&raw_conn, engine, &db_bg, &tbl_names)
                                        .await;
                                catalog.set_all_columns(cols);
                            });

                            s.databases = dbs.clone();
                            s.objects = objs;
                            let items: Vec<SharedString> = dbs.into_iter().map(Into::into).collect();
                            let sel = s
                                .databases
                                .iter()
                                .position(|d| d == &cur)
                                .map(IndexPath::new);
                            s.db_select.update(cx, |sel_state, cx| {
                                sel_state.set_items(SearchableVec::new(items), window, cx);
                                sel_state.set_selected_index(sel, window, cx);
                            });
                            s.status = format!("Refreshed ({} objects)", s.objects.len());
                        }
                        Err(e) => s.status = format!("Refresh failed: {e}"),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn refresh_current_database_objects(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.active_tab().id;
        let Some(session) = self.active_tab_mut().session.as_mut() else {
            return;
        };
        if session.current_db.is_empty() {
            return;
        }
        let conn = session.conn.clone();
        let db = session.current_db.clone();
        session.busy = true;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move { conn.list_objects(&db).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));
            this.update_in(cx, |this, _window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    match res {
                        Ok(objs) => {
                            let cur = s.current_db.clone();
                            s.schema_catalog.set_objects(cur.clone(), &objs);
                            let raw_conn = s.raw_conn.clone();
                            let engine = s.info.engine;
                            let catalog = s.schema_catalog.clone();
                            let tbl_names: Vec<String> =
                                objs.iter().map(|o| o.name.clone()).collect();
                            tokio_rt().spawn(async move {
                                let cols =
                                    fetch_database_columns(&raw_conn, engine, &cur, &tbl_names)
                                        .await;
                                catalog.set_all_columns(cols);
                            });

                            s.objects = objs;
                            s.status = format!("{} objects", s.objects.len());
                        }
                        Err(e) => s.status = format!("Error: {e}"),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn select_table(
        &mut self,
        table: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.active_tab().id;
        let Some(session) = self.active_tab_mut().session.as_mut() else {
            return;
        };
        let conn = session.conn.clone();
        let db = session.current_db.clone();
        let page_size = session.page_size;
        session.selected_table = Some(table.clone());
        session.page_offset = 0;
        session.sort_column = None;
        session.sort_desc = false;
        session.where_clause = None;
        session.selected_cell = None;
        session.selected_structure_row = None;
        session.selected_index_row = None;
        session.selected_relation_row = None;
        session.selected_trigger_row = None;
        session.busy = true;
        let filter_inp = session.content_filter_value.clone();
        set_input_val(&filter_inp, "", window, cx);
        cx.notify();

        let tbl_a = table.clone();
        let tbl_b = table.clone();
        let conn_a = conn.clone();
        let db_a = db.clone();

        cx.spawn_in(window, async move |this, cx| {
            let struct_res = tokio_rt()
                .spawn(async move { fetch_structure(&conn_a, &db_a, &tbl_a).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));

            let content_res = tokio_rt()
                .spawn(async move {
                    fetch_table_page(&conn, &db, &tbl_b, page_size, 0, None, None).await
                })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));

            this.update_in(cx, |this, window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    if let Ok((meta, triggers)) = struct_res {
                        apply_structure_to_session(s, meta, triggers, window, cx);
                    }
                    match content_res {
                        Ok(page) => {
                            s.content_columns = page.columns.clone();
                            s.row_count = page.total_count;
                            s.content_raw = Some(page.raw_result);
                            let mut filter_items: Vec<SharedString> =
                                vec!["Custom WHERE".into()];
                            filter_items.extend(page.columns.iter().cloned().map(Into::into));
                            s.filter_col_select.update(cx, |sel, cx| {
                                sel.set_items(SearchableVec::new(filter_items), window, cx);
                                sel.set_selected_index(Some(IndexPath::new(0)), window, cx);
                            });
                            s.content_table.update(cx, |t, cx| {
                                t.clear_selection(cx);
                                t.delegate_mut().set_data(page.columns, page.cells, None, false);
                                t.refresh(cx);
                            });
                            s.status = format!(
                                "{}.{table} — {} rows",
                                s.current_db,
                                s.row_count.unwrap_or(0)
                            );
                        }
                        Err(e) => {
                            s.status = format!("Content error: {e}");
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn reload_selected_table(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tbl = self
            .active_tab()
            .session
            .as_ref()
            .and_then(|s| s.selected_table.clone());
        if let Some(table) = tbl {
            self.select_table(table, window, cx);
        }
    }

    pub(crate) fn toggle_pin_selected_table(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let db = s.current_db.clone();
        let key = (db.clone(), table.clone());
        if let Ok(store) = HistoryStore::open() {
            if self.pinned_tables.contains(&key) {
                let _ = store.unpin_table(&db, &table);
                self.pinned_tables.retain(|p| p != &key);
            } else {
                let _ = store.pin_table(&db, &table);
                self.pinned_tables.push(key);
            }
        }
        cx.notify();
    }

    // ─── Content Pagination, Sort, Filter, Cell Edit & Row Ops ──────────────

    fn order_clause(session: &WorkspaceSession) -> Option<String> {
        let col = session.sort_column.as_ref()?;
        let dir = if session.sort_desc { "DESC" } else { "ASC" };
        Some(format!("{} {dir}", session.conn.dialect().quote_ident(col)))
    }

    pub(crate) fn load_content_page(
        &mut self,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.active_tab().id;
        let Some(s) = self.active_tab_mut().session.as_mut() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let conn = s.conn.clone();
        let db = s.current_db.clone();
        let limit = s.page_size;
        let order = Self::order_clause(s);
        let where_clause = s.where_clause.clone();
        let sort_col = s.sort_column.clone();
        let sort_desc = s.sort_desc;
        s.busy = true;
        s.selected_cell = None;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move {
                    fetch_table_page(
                        &conn,
                        &db,
                        &table,
                        limit,
                        offset,
                        order.as_deref(),
                        where_clause.as_deref(),
                    )
                    .await
                })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));

            this.update_in(cx, |this, _window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    s.page_offset = offset;
                    match res {
                        Ok(page) => {
                            let loaded = page.cells.len();
                            s.content_columns = page.columns.clone();
                            s.row_count = page.total_count;
                            s.content_raw = Some(page.raw_result);
                            s.content_table.update(cx, |t, cx| {
                                t.clear_selection(cx);
                                t.delegate_mut().set_data(
                                    page.columns,
                                    page.cells,
                                    sort_col.as_deref(),
                                    sort_desc,
                                );
                                t.refresh(cx);
                            });
                            s.status = format!(
                                "{}.{} — {}–{} of {}",
                                s.current_db,
                                s.selected_table.as_deref().unwrap_or("?"),
                                if loaded == 0 { 0 } else { offset + 1 },
                                offset + loaded,
                                s.row_count.unwrap_or(0)
                            );
                        }
                        Err(e) => s.status = format!("Error: {e}"),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn apply_content_sort(
        &mut self,
        col: String,
        sort: ColumnSort,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(s) = self.active_tab_mut().session.as_mut() {
            match sort {
                ColumnSort::Ascending => {
                    s.sort_column = Some(col);
                    s.sort_desc = false;
                }
                ColumnSort::Descending => {
                    s.sort_column = Some(col);
                    s.sort_desc = true;
                }
                ColumnSort::Default => {
                    s.sort_column = None;
                    s.sort_desc = false;
                }
            }
            s.page_offset = 0;
        }
        self.load_content_page(0, window, cx);
    }

    pub(crate) fn apply_content_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let col_choice = s
            .filter_col_select
            .read(cx)
            .selected_value()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "Custom WHERE".into());
        let op_label = s
            .filter_op_select
            .read(cx)
            .selected_value()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "=".into());
        let raw_val = read_input_val(&s.content_filter_value, cx);
        let dialect = s.conn.dialect();

        let where_sql = if col_choice == "Custom WHERE" {
            let trimmed = raw_val
                .trim()
                .trim_start_matches("WHERE")
                .trim_start_matches("where")
                .trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        } else {
            let op = filter_op_by_label(&op_label);
            if op.needs_value() && raw_val.trim().is_empty() {
                None
            } else {
                let node = FilterNode::rule(
                    col_choice,
                    op,
                    if matches!(op, FilterOp::IsNull | FilterOp::IsNotNull) {
                        None
                    } else {
                        Some(raw_val)
                    },
                );
                let clause = node.to_where_clause(&|c| dialect.quote_ident(c));
                if clause.is_empty() {
                    None
                } else {
                    Some(clause)
                }
            }
        };

        if let Some(s) = self.active_tab_mut().session.as_mut() {
            s.where_clause = where_sql;
            s.page_offset = 0;
        }
        self.load_content_page(0, window, cx);
    }

    pub(crate) fn clear_content_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let inp = self
            .active_tab()
            .session
            .as_ref()
            .map(|s| s.content_filter_value.clone());
        if let Some(inp) = inp {
            set_input_val(&inp, "", window, cx);
        }
        if let Some(s) = self.active_tab_mut().session.as_mut() {
            s.where_clause = None;
            s.page_offset = 0;
        }
        self.load_content_page(0, window, cx);
    }

    pub(crate) fn cycle_page_size(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(s) = self.active_tab_mut().session.as_mut() {
            s.page_size = match s.page_size {
                100 => 500,
                500 => 1000,
                _ => 100,
            };
            s.page_offset = 0;
        }
        self.load_content_page(0, window, cx);
    }

    fn on_select_content_cell(
        &mut self,
        row: usize,
        col: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cell_val = {
            let Some(s) = self.active_tab_mut().session.as_mut() else {
                return;
            };
            s.selected_cell = Some((row, col));
            let val = s
                .content_table
                .read(cx)
                .delegate()
                .cell(row, col)
                .map(|c| if c.is_null { String::new() } else { c.display.clone() })
                .unwrap_or_default();
            (s.cell_edit_input.clone(), val)
        };
        set_input_val(&cell_val.0, cell_val.1, window, cx);
        cx.notify();
    }

    fn row_keys(
        session: &WorkspaceSession,
        row_ix: usize,
        cx: &App,
    ) -> (Vec<(String, String, bool)>, Vec<(String, String, bool)>) {
        let table = session.content_table.read(cx);
        let Some(row) = table.delegate().row(row_ix) else {
            return (Vec::new(), Vec::new());
        };
        let all: Vec<(String, String, bool)> = session
            .content_columns
            .iter()
            .enumerate()
            .map(|(i, col)| match row.get(i) {
                Some(cell) => (col.clone(), cell.display.clone(), cell.is_null),
                None => (col.clone(), String::new(), true),
            })
            .collect();

        let pks: Vec<(String, String, bool)> = session
            .table_meta
            .as_ref()
            .map(|meta| {
                meta.columns
                    .iter()
                    .filter(|c| c.primary_key)
                    .filter_map(|c| {
                        let idx = session.content_columns.iter().position(|n| n == &c.name)?;
                        let cell = row.get(idx)?;
                        Some((c.name.clone(), cell.display.clone(), cell.is_null))
                    })
                    .collect()
            })
            .unwrap_or_default();

        (pks, all)
    }

    pub(crate) fn save_inline_cell_edit(
        &mut self,
        set_null: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some((row, col)) = s.selected_cell else {
            return;
        };
        let new_val = if set_null {
            String::new()
        } else {
            read_input_val(&s.cell_edit_input, cx)
        };
        self.update_cell_value(row, col, new_val, set_null, window, cx);
    }

    fn update_cell_value(
        &mut self,
        row: usize,
        col: usize,
        new_val: String,
        set_null: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.active_tab().id;
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let Some(col_name) = s.content_columns.get(col).cloned() else {
            return;
        };
        let (pks, all) = Self::row_keys(s, row, cx);
        let dialect = s.conn.dialect();
        let db = s.current_db.clone();
        let sql = match ace_sql::build_update_row(
            &dialect, &db, &table, &col_name, &new_val, set_null, &pks, &all,
        ) {
            Ok(sql) => sql,
            Err(e) => {
                self.set_session_status(e, cx);
                return;
            }
        };
        let conn = s.conn.clone();
        let offset = s.page_offset;
        if let Some(s) = self.active_tab_mut().session.as_mut() {
            s.busy = true;
        }
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move { exec_single_sql(&conn, &sql).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));

            this.update_in(cx, |this, window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    match res {
                        Ok(_) => {
                            s.status = format!("Updated `{col_name}`");
                            this.load_content_page(offset, window, cx);
                        }
                        Err(e) => s.status = format!("Update failed: {e}"),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn open_selected_cell_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some((row, col)) = s.selected_cell else {
            return;
        };
        let col_name = s
            .content_columns
            .get(col)
            .cloned()
            .unwrap_or_else(|| format!("col_{col}"));
        let (val, is_null) = s
            .content_table
            .read(cx)
            .delegate()
            .cell(row, col)
            .map(|c| {
                if c.is_null {
                    (String::new(), true)
                } else {
                    (c.display.clone(), false)
                }
            })
            .unwrap_or((String::new(), false));

        self.modal = Some(ModalState::FieldEditor {
            row,
            col,
            column_name: col_name,
            is_null,
            textarea: new_textarea(window, cx, val, "Field value…"),
        });
        cx.notify();
    }

    pub(crate) fn pretty_format_field_editor_json(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ta = match &self.modal {
            Some(ModalState::FieldEditor { textarea, .. }) => textarea.clone(),
            _ => return,
        };
        let raw = ta.read(cx).value().to_string();
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                ta.update(cx, |s, cx| s.set_value(pretty, window, cx));
            }
        }
    }

    pub(crate) fn commit_field_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ModalState::FieldEditor {
            row,
            col,
            is_null,
            textarea,
            ..
        }) = self.modal.take()
        {
            let val = textarea.read(cx).value().to_string();
            self.update_cell_value(row, col, val, is_null, window, cx);
        }
    }

    pub(crate) fn copy_selected_cell(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some((row, col)) = s.selected_cell else {
            return;
        };
        if let Some(cell) = s.content_table.read(cx).delegate().cell(row, col) {
            cx.write_to_clipboard(ClipboardItem::new_string(cell.display.clone()));
            self.set_session_status("Copied cell value", cx);
        }
    }

    pub(crate) fn copy_selected_row_as_insert(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let Some((row, _)) = s.selected_cell else {
            self.set_session_status("Select a row first", cx);
            return;
        };
        let (_, all) = Self::row_keys(s, row, cx);
        if let Ok(sql) =
            ace_sql::build_insert_typed(&s.conn.dialect(), &s.current_db, &table, &all)
        {
            cx.write_to_clipboard(ClipboardItem::new_string(format!("{sql};")));
            self.set_session_status("Copied INSERT statement", cx);
        }
    }

    pub(crate) fn open_insert_row_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let cols: Vec<(String, String, bool)> = if let Some(meta) = &s.table_meta {
            meta.columns
                .iter()
                .map(|c| {
                    let placeholder = if c.auto_increment {
                        "AUTO_INCREMENT (leave NULL)".into()
                    } else if let Some(d) = &c.default {
                        format!("default: {d}")
                    } else {
                        c.full_type.clone()
                    };
                    let default_null = c.auto_increment || c.nullable;
                    (c.name.clone(), placeholder, default_null)
                })
                .collect()
        } else {
            s.content_columns
                .iter()
                .map(|c| (c.clone(), String::new(), false))
                .collect()
        };
        if cols.is_empty() {
            return;
        }
        let mut columns = Vec::new();
        let mut inputs = Vec::new();
        let mut null_flags = Vec::new();
        for (name, ph, def_null) in cols {
            columns.push(name);
            inputs.push(new_input(window, cx, "", ph, false));
            null_flags.push(def_null);
        }
        self.modal = Some(ModalState::InsertRow {
            table,
            columns,
            inputs,
            null_flags,
        });
        cx.notify();
    }

    pub(crate) fn commit_insert_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ModalState::InsertRow {
            table,
            columns,
            inputs,
            null_flags,
        }) = self.modal.take()
        else {
            return;
        };
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let entries: Vec<(String, String, bool)> = columns
            .into_iter()
            .enumerate()
            .map(|(i, col)| {
                let is_null = null_flags.get(i).copied().unwrap_or(false);
                let val = if is_null {
                    String::new()
                } else {
                    read_input_val(&inputs[i], cx)
                };
                (col, val, is_null)
            })
            .collect();

        let sql = match ace_sql::build_insert_typed(
            &s.conn.dialect(),
            &s.current_db,
            &table,
            &entries,
        ) {
            Ok(sql) => sql,
            Err(e) => {
                self.set_session_status(e, cx);
                return;
            }
        };
        self.exec_and_reload_content(sql, "Row inserted", window, cx);
    }

    pub(crate) fn duplicate_selected_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let Some((row, _)) = s.selected_cell else {
            self.set_session_status("Select a row to duplicate", cx);
            return;
        };
        let (_, all) = Self::row_keys(s, row, cx);
        let auto_inc_cols: Vec<String> = s
            .table_meta
            .as_ref()
            .map(|m| {
                m.columns
                    .iter()
                    .filter(|c| c.auto_increment)
                    .map(|c| c.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        let entries: Vec<(String, String, bool)> = all
            .into_iter()
            .filter(|(c, _, _)| !auto_inc_cols.contains(c))
            .collect();
        match ace_sql::build_insert_typed(&s.conn.dialect(), &s.current_db, &table, &entries) {
            Ok(sql) => self.exec_and_reload_content(sql, "Row duplicated", window, cx),
            Err(e) => self.set_session_status(e, cx),
        }
    }

    pub(crate) fn request_delete_selected_row(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let Some((row_ix, _)) = s.selected_cell else {
            self.set_session_status("Select a row first", cx);
            return;
        };
        self.modal = Some(ModalState::Confirm {
            title: "Delete Row".into(),
            message: format!(
                "Delete row {} from `{table}`? This operation cannot be undone.",
                s.page_offset + row_ix + 1
            ),
            confirm_label: "Delete Row".into(),
            action: ConfirmAction::DeleteRow { table, row_ix },
        });
        cx.notify();
    }

    fn exec_and_reload_content(
        &mut self,
        sql: String,
        ok_msg: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.active_tab().id;
        let Some(s) = self.active_tab_mut().session.as_mut() else {
            return;
        };
        let conn = s.conn.clone();
        let offset = s.page_offset;
        s.busy = true;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move { exec_single_sql(&conn, &sql).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));
            this.update_in(cx, |this, window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    match res {
                        Ok(_) => {
                            s.status = ok_msg.into();
                            this.load_content_page(offset, window, cx);
                        }
                        Err(e) => s.status = format!("Error: {e}"),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ─── DDL Actions (DB / Table / Column / Index / FK / Trigger) ──────────

    pub(crate) fn open_add_database_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal = Some(ModalState::AddDatabase {
            name: new_input(window, cx, "", "new_database", false),
            charset: new_input(window, cx, "utf8mb4", "utf8mb4", false),
            collation: new_input(window, cx, "utf8mb4_unicode_ci", "utf8mb4_unicode_ci", false),
        });
        cx.notify();
    }

    pub(crate) fn request_drop_current_database(&mut self, cx: &mut Context<Self>) {
        let Some(db) = self
            .active_tab()
            .session
            .as_ref()
            .map(|s| s.current_db.clone())
            .filter(|d| !d.is_empty())
        else {
            return;
        };
        self.modal = Some(ModalState::Confirm {
            title: format!("Drop Database `{db}`"),
            message: format!("Permanently drop database `{db}` and EVERY table inside it?"),
            confirm_label: "Drop Database".into(),
            action: ConfirmAction::DropDatabase(db),
        });
        cx.notify();
    }

    pub(crate) fn commit_add_database(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ModalState::AddDatabase {
            name,
            charset,
            collation,
        }) = self.modal.take()
        else {
            return;
        };
        let db_name = read_input_val(&name, cx).trim().to_string();
        if db_name.is_empty() {
            return;
        }
        let cs = read_input_val(&charset, cx);
        let col = read_input_val(&collation, cx);
        let sql = ace_sql::create_database_sql(&db_name, Some(&cs), Some(&col));
        let tab_id = self.active_tab().id;
        let Some(s) = self.active_tab_mut().session.as_mut() else {
            return;
        };
        let conn = s.conn.clone();
        s.busy = true;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move { exec_single_sql(&conn, &sql).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));
            this.update_in(cx, |this, window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    match res {
                        Ok(_) => {
                            s.status = format!("Created database `{db_name}`");
                            this.refresh_databases_and_tables(window, cx);
                        }
                        Err(e) => s.status = format!("Create DB failed: {e}"),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn open_add_table_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal = Some(ModalState::AddTable {
            name: new_input(window, cx, "", "new_table", false),
            engine: new_input(window, cx, "InnoDB", "InnoDB", false),
            charset: new_input(window, cx, "utf8mb4", "utf8mb4", false),
        });
        cx.notify();
    }

    pub(crate) fn commit_add_table(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ModalState::AddTable {
            name,
            engine,
            charset,
        }) = self.modal.take()
        else {
            return;
        };
        let tbl = read_input_val(&name, cx).trim().to_string();
        if tbl.is_empty() {
            return;
        }
        let eng = read_input_val(&engine, cx);
        let cs = read_input_val(&charset, cx);
        let tab_id = self.active_tab().id;
        let Some(s) = self.active_tab_mut().session.as_mut() else {
            return;
        };
        let is_sqlite = s.info.engine == EngineKind::Sqlite;
        let sql = ace_sql::create_table_sql(&s.current_db, &tbl, Some(&eng), Some(&cs), is_sqlite);
        let conn = s.conn.clone();
        s.busy = true;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move { exec_single_sql(&conn, &sql).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));
            this.update_in(cx, |this, window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    match res {
                        Ok(_) => {
                            s.status = format!("Created table `{tbl}`");
                            this.refresh_current_database_objects(window, cx);
                            this.select_table(tbl, window, cx);
                        }
                        Err(e) => s.status = format!("Create table failed: {e}"),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn open_rename_table_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tbl) = self
            .active_tab()
            .session
            .as_ref()
            .and_then(|s| s.selected_table.clone())
        else {
            return;
        };
        self.modal = Some(ModalState::RenameTable {
            old_name: tbl.clone(),
            new_name: new_input(window, cx, tbl, "New table name", false),
        });
        cx.notify();
    }

    pub(crate) fn commit_rename_table(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ModalState::RenameTable { old_name, new_name }) = self.modal.take() else {
            return;
        };
        let new_tbl = read_input_val(&new_name, cx).trim().to_string();
        if new_tbl.is_empty() || new_tbl == old_name {
            return;
        }
        let tab_id = self.active_tab().id;
        let Some(s) = self.active_tab_mut().session.as_mut() else {
            return;
        };
        let sql = if s.info.engine == EngineKind::Sqlite {
            format!(
                "ALTER TABLE \"{}\" RENAME TO \"{}\"",
                old_name.replace('"', "\"\""),
                new_tbl.replace('"', "\"\"")
            )
        } else {
            ace_sql::rename_table_sql(&s.current_db, &old_name, &new_tbl)
        };
        let conn = s.conn.clone();
        s.busy = true;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move { exec_single_sql(&conn, &sql).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));
            this.update_in(cx, |this, window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    match res {
                        Ok(_) => {
                            s.status = format!("Renamed to `{new_tbl}`");
                            this.refresh_current_database_objects(window, cx);
                            this.select_table(new_tbl, window, cx);
                        }
                        Err(e) => s.status = format!("Rename failed: {e}"),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn request_drop_selected_table(&mut self, cx: &mut Context<Self>) {
        let Some(tbl) = self
            .active_tab()
            .session
            .as_ref()
            .and_then(|s| s.selected_table.clone())
        else {
            return;
        };
        self.modal = Some(ModalState::Confirm {
            title: format!("Drop Table `{tbl}`"),
            message: format!("Permanently drop table `{tbl}` and ALL of its data?"),
            confirm_label: "Drop Table".into(),
            action: ConfirmAction::DropTable(tbl),
        });
        cx.notify();
    }

    pub(crate) fn request_truncate_selected_table(&mut self, cx: &mut Context<Self>) {
        let Some(tbl) = self
            .active_tab()
            .session
            .as_ref()
            .and_then(|s| s.selected_table.clone())
        else {
            return;
        };
        self.modal = Some(ModalState::Confirm {
            title: format!("Truncate Table `{tbl}`"),
            message: format!("Delete EVERY row from `{tbl}`? This cannot be undone."),
            confirm_label: "Truncate".into(),
            action: ConfirmAction::TruncateTable(tbl),
        });
        cx.notify();
    }

    pub(crate) fn open_add_column_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(table) = self
            .active_tab()
            .session
            .as_ref()
            .and_then(|s| s.selected_table.clone())
        else {
            return;
        };
        self.modal = Some(ModalState::EditColumn {
            table,
            editing_old_name: None,
            name: new_input(window, cx, "", "column_name", false),
            data_type: new_input(window, cx, "VARCHAR(255)", "VARCHAR(255) / INT / TEXT", false),
            nullable: true,
            auto_increment: false,
            default_val: new_input(window, cx, "", "Optional default", false),
            comment: new_input(window, cx, "", "Optional comment", false),
        });
        cx.notify();
    }

    pub(crate) fn open_edit_selected_column_modal(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let Some(row_ix) = s.selected_structure_row else {
            self.set_session_status("Select a column row first", cx);
            return;
        };
        let Some(col) = s
            .table_meta
            .as_ref()
            .and_then(|m| m.columns.get(row_ix))
            .cloned()
        else {
            return;
        };
        self.modal = Some(ModalState::EditColumn {
            table,
            editing_old_name: Some(col.name.clone()),
            name: new_input(window, cx, col.name, "column_name", false),
            data_type: new_input(window, cx, col.full_type, "VARCHAR(255)", false),
            nullable: col.nullable,
            auto_increment: col.auto_increment,
            default_val: new_input(window, cx, col.default.unwrap_or_default(), "Optional", false),
            comment: new_input(window, cx, col.comment.unwrap_or_default(), "Optional", false),
        });
        cx.notify();
    }

    pub(crate) fn commit_edit_column(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ModalState::EditColumn {
            table,
            editing_old_name,
            name,
            data_type,
            nullable,
            auto_increment,
            default_val,
            comment,
        }) = self.modal.take()
        else {
            return;
        };
        let col_name = read_input_val(&name, cx).trim().to_string();
        let dtype = read_input_val(&data_type, cx).trim().to_string();
        if col_name.is_empty() || dtype.is_empty() {
            return;
        }
        let def = read_input_val(&default_val, cx);
        let cmt = read_input_val(&comment, cx);
        let meta = ColumnMeta {
            name: col_name,
            data_type: dtype.clone(),
            full_type: dtype,
            nullable,
            primary_key: false,
            auto_increment,
            default: if def.trim().is_empty() {
                None
            } else {
                Some(def)
            },
            extra: None,
            comment: if cmt.trim().is_empty() {
                None
            } else {
                Some(cmt)
            },
            charset: None,
            collation: None,
            unsigned: false,
            zerofill: false,
            character_maximum_length: None,
            numeric_precision: None,
            numeric_scale: None,
            ordinal_position: 0,
        };
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let sql = match editing_old_name {
            Some(old) => ace_sql::change_column_sql(&s.current_db, &table, &old, &meta),
            None => ace_sql::add_column_sql(&s.current_db, &table, &meta),
        };
        self.exec_ddl_and_reload_table(sql, "Column saved", window, cx);
    }

    pub(crate) fn request_drop_selected_column(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let Some(row_ix) = s.selected_structure_row else {
            self.set_session_status("Select a column first", cx);
            return;
        };
        let Some(col) = s
            .table_meta
            .as_ref()
            .and_then(|m| m.columns.get(row_ix))
            .map(|c| c.name.clone())
        else {
            return;
        };
        self.modal = Some(ModalState::Confirm {
            title: format!("Drop Column `{col}`"),
            message: format!("Drop column `{col}` from table `{table}`?"),
            confirm_label: "Drop Column".into(),
            action: ConfirmAction::DropColumn { table, column: col },
        });
        cx.notify();
    }

    pub(crate) fn open_add_index_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(table) = self
            .active_tab()
            .session
            .as_ref()
            .and_then(|s| s.selected_table.clone())
        else {
            return;
        };
        self.modal = Some(ModalState::AddIndex {
            table,
            name: new_input(window, cx, "", "idx_column", false),
            columns: new_input(window, cx, "", "col1, col2", false),
            unique: false,
        });
        cx.notify();
    }

    pub(crate) fn commit_add_index(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ModalState::AddIndex {
            table,
            name,
            columns,
            unique,
        }) = self.modal.take()
        else {
            return;
        };
        let idx_name = read_input_val(&name, cx).trim().to_string();
        let cols_raw = read_input_val(&columns, cx);
        let cols: Vec<&str> = cols_raw
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if idx_name.is_empty() || cols.is_empty() {
            return;
        }
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let sql = ace_sql::add_index_sql(&s.current_db, &table, &idx_name, &cols, unique);
        self.exec_ddl_and_reload_table(sql, "Index added", window, cx);
    }

    pub(crate) fn request_drop_selected_index(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let Some(row_ix) = s.selected_index_row else {
            self.set_session_status("Select an index first", cx);
            return;
        };
        let Some(idx_name) = s
            .table_meta
            .as_ref()
            .and_then(|m| m.indexes.get(row_ix))
            .map(|i| i.name.clone())
        else {
            return;
        };
        self.modal = Some(ModalState::Confirm {
            title: format!("Drop Index `{idx_name}`"),
            message: format!("Drop index `{idx_name}` on `{table}`?"),
            confirm_label: "Drop Index".into(),
            action: ConfirmAction::DropIndex {
                table,
                index: idx_name,
            },
        });
        cx.notify();
    }

    pub(crate) fn open_add_foreign_key_modal(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(table) = self
            .active_tab()
            .session
            .as_ref()
            .and_then(|s| s.selected_table.clone())
        else {
            return;
        };
        self.modal = Some(ModalState::AddForeignKey {
            table: table.clone(),
            name: new_input(window, cx, format!("fk_{table}_"), "fk_name", false),
            column: new_input(window, cx, "", "user_id", false),
            ref_table: new_input(window, cx, "", "users", false),
            ref_column: new_input(window, cx, "id", "id", false),
            on_update: new_input(window, cx, "RESTRICT", "RESTRICT / CASCADE / SET NULL", false),
            on_delete: new_input(window, cx, "RESTRICT", "RESTRICT / CASCADE / SET NULL", false),
        });
        cx.notify();
    }

    pub(crate) fn commit_add_foreign_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ModalState::AddForeignKey {
            table,
            name,
            column,
            ref_table,
            ref_column,
            on_update,
            on_delete,
        }) = self.modal.take()
        else {
            return;
        };
        let cname = read_input_val(&name, cx).trim().to_string();
        let col_raw = read_input_val(&column, cx);
        let rtbl = read_input_val(&ref_table, cx).trim().to_string();
        let rcol_raw = read_input_val(&ref_column, cx);
        let upd = read_input_val(&on_update, cx);
        let del = read_input_val(&on_delete, cx);
        let cols: Vec<&str> = col_raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let rcols: Vec<&str> = rcol_raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if cname.is_empty() || cols.is_empty() || rtbl.is_empty() || rcols.is_empty() {
            return;
        }
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let sql = ace_sql::add_foreign_key_sql(
            &s.current_db,
            &table,
            &cname,
            &cols,
            &rtbl,
            &rcols,
            if upd.trim().is_empty() { "RESTRICT" } else { upd.trim() },
            if del.trim().is_empty() { "RESTRICT" } else { del.trim() },
        );
        self.exec_ddl_and_reload_table(sql, "Foreign key added", window, cx);
    }

    pub(crate) fn request_drop_selected_relation(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let Some(row_ix) = s.selected_relation_row else {
            self.set_session_status("Select a foreign key first", cx);
            return;
        };
        let Some(fk_name) = s
            .table_meta
            .as_ref()
            .and_then(|m| m.foreign_keys.get(row_ix))
            .map(|f| f.name.clone())
        else {
            return;
        };
        self.modal = Some(ModalState::Confirm {
            title: format!("Drop Foreign Key `{fk_name}`"),
            message: format!("Drop foreign key `{fk_name}` from `{table}`?"),
            confirm_label: "Drop Foreign Key".into(),
            action: ConfirmAction::DropForeignKey {
                table,
                constraint: fk_name,
            },
        });
        cx.notify();
    }

    pub(crate) fn open_add_trigger_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(table) = self
            .active_tab()
            .session
            .as_ref()
            .and_then(|s| s.selected_table.clone())
        else {
            return;
        };
        self.modal = Some(ModalState::AddTrigger {
            table: table.clone(),
            name: new_input(window, cx, format!("trg_{table}_"), "trigger_name", false),
            timing: "BEFORE".into(),
            event: "INSERT".into(),
            body: new_textarea(window, cx, "BEGIN\n  -- trigger body\nEND", "BEGIN ... END"),
        });
        cx.notify();
    }

    pub(crate) fn commit_add_trigger(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ModalState::AddTrigger {
            table,
            name,
            timing,
            event,
            body,
        }) = self.modal.take()
        else {
            return;
        };
        let tname = read_input_val(&name, cx).trim().to_string();
        let stmt = body.read(cx).value().to_string();
        if tname.is_empty() || stmt.trim().is_empty() {
            return;
        }
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let sql =
            ace_sql::create_trigger_sql(&s.current_db, &tname, &timing, &event, &table, &stmt);
        self.exec_ddl_and_reload_table(sql, "Trigger created", window, cx);
    }

    pub(crate) fn request_drop_selected_trigger(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(row_ix) = s.selected_trigger_row else {
            self.set_session_status("Select a trigger first", cx);
            return;
        };
        let Some(trigger) = s.triggers.get(row_ix).map(|t| t.name.clone()) else {
            return;
        };
        self.modal = Some(ModalState::Confirm {
            title: format!("Drop Trigger `{trigger}`"),
            message: format!("Permanently drop trigger `{trigger}`?"),
            confirm_label: "Drop Trigger".into(),
            action: ConfirmAction::DropTrigger { trigger },
        });
        cx.notify();
    }

    pub(crate) fn save_table_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let comment = read_input_val(&s.table_comment_input, cx);
        let sql = ace_sql::alter_table_comment_sql(&s.current_db, &table, &comment);
        self.exec_ddl_and_reload_table(sql, "Updated table comment", window, cx);
    }

    pub(crate) fn run_table_maintenance(
        &mut self,
        op: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let Some(table) = s.selected_table.clone() else {
            return;
        };
        let sql = format!(
            "{op} {}",
            s.conn.dialect().qualified_name(Some(&s.current_db), &table)
        );
        self.exec_ddl_and_reload_table(sql, op, window, cx);
    }

    fn exec_ddl_and_reload_table(
        &mut self,
        sql: String,
        ok_msg: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.active_tab().id;
        let Some(s) = self.active_tab_mut().session.as_mut() else {
            return;
        };
        let conn = s.conn.clone();
        s.busy = true;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move { exec_single_sql(&conn, &sql).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));
            this.update_in(cx, |this, window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    match res {
                        Ok(_) => {
                            s.status = ok_msg.into();
                            this.reload_selected_table(window, cx);
                        }
                        Err(e) => s.status = format!("SQL error: {e}"),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ─── Confirm Modal Executor ─────────────────────────────────────────────

    pub(crate) fn execute_confirm_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ModalState::Confirm { action, .. }) = self.modal.take() else {
            return;
        };
        match action {
            ConfirmAction::DeleteFavorite(id) => {
                self.favorites.remove(id);
                let _ = delete_password(&id.to_string());
                let _ = delete_password(&format!("{id}:ssh"));
                let d = &mut self.active_tab_mut().draft;
                if d.selected_favorite == Some(id) {
                    d.selected_favorite = None;
                    d.retained_password = None;
                }
                if d.selected_group == Some(id) {
                    d.selected_group = None;
                }
                let _ = save_favorites(&self.favorites);
                cx.notify();
            }
            ConfirmAction::DropDatabase(db) => {
                let sql = ace_sql::drop_database_sql(&db);
                let tab_id = self.active_tab().id;
                let Some(s) = self.active_tab_mut().session.as_mut() else {
                    return;
                };
                let conn = s.conn.clone();
                s.busy = true;
                cx.spawn_in(window, async move |this, cx| {
                    let res = tokio_rt()
                        .spawn(async move { exec_single_sql(&conn, &sql).await })
                        .await
                        .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));
                    this.update_in(cx, |this, window, cx| {
                        if let Some(s) = this.session_by_tab_mut(tab_id) {
                            s.busy = false;
                            match res {
                                Ok(_) => this.refresh_databases_and_tables(window, cx),
                                Err(e) => s.status = format!("Drop DB failed: {e}"),
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }
            ConfirmAction::DropTable(table) => {
                let tab_id = self.active_tab().id;
                let Some(s) = self.active_tab_mut().session.as_mut() else {
                    return;
                };
                let is_sqlite = s.info.engine == EngineKind::Sqlite;
                let sql = ace_sql::drop_table_sql(&s.current_db, &table, is_sqlite);
                let conn = s.conn.clone();
                s.busy = true;
                cx.notify();

                cx.spawn_in(window, async move |this, cx| {
                    let res = tokio_rt()
                        .spawn(async move { exec_single_sql(&conn, &sql).await })
                        .await
                        .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));
                    this.update_in(cx, |this, window, cx| {
                        if let Some(s) = this.session_by_tab_mut(tab_id) {
                            s.busy = false;
                            match res {
                                Ok(_) => {
                                    s.selected_table = None;
                                    s.table_meta = None;
                                    s.status = format!("Dropped `{table}`");
                                    this.refresh_current_database_objects(window, cx);
                                }
                                Err(e) => s.status = format!("Drop table failed: {e}"),
                            }
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }
            ConfirmAction::TruncateTable(table) => {
                let Some(s) = self.active_tab().session.as_ref() else {
                    return;
                };
                let sql = ace_sql::truncate_table_sql(
                    &s.current_db,
                    &table,
                    s.info.engine == EngineKind::Sqlite,
                );
                self.exec_and_reload_content(sql, "Table truncated", window, cx);
            }
            ConfirmAction::DropColumn { table, column } => {
                let Some(s) = self.active_tab().session.as_ref() else {
                    return;
                };
                let sql = ace_sql::drop_column_sql(&s.current_db, &table, &column);
                self.exec_ddl_and_reload_table(sql, "Column dropped", window, cx);
            }
            ConfirmAction::DropIndex { table, index } => {
                let Some(s) = self.active_tab().session.as_ref() else {
                    return;
                };
                let sql = ace_sql::drop_index_sql(&s.current_db, &table, &index);
                self.exec_ddl_and_reload_table(sql, "Index dropped", window, cx);
            }
            ConfirmAction::DropForeignKey { table, constraint } => {
                let Some(s) = self.active_tab().session.as_ref() else {
                    return;
                };
                let sql = ace_sql::drop_foreign_key_sql(&s.current_db, &table, &constraint);
                self.exec_ddl_and_reload_table(sql, "Foreign key dropped", window, cx);
            }
            ConfirmAction::DropTrigger { trigger } => {
                let Some(s) = self.active_tab().session.as_ref() else {
                    return;
                };
                let sql = ace_sql::drop_trigger_sql(
                    &s.current_db,
                    &trigger,
                    s.info.engine == EngineKind::Sqlite,
                );
                self.exec_ddl_and_reload_table(sql, "Trigger dropped", window, cx);
            }
            ConfirmAction::DeleteRow { table, row_ix } => {
                let Some(s) = self.active_tab().session.as_ref() else {
                    return;
                };
                let (pks, all) = Self::row_keys(s, row_ix, cx);
                match ace_sql::build_delete_row(&s.conn.dialect(), &s.current_db, &table, &pks, &all)
                {
                    Ok(sql) => self.exec_and_reload_content(sql, "Row deleted", window, cx),
                    Err(e) => self.set_session_status(e, cx),
                }
            }
        }
    }

    pub(crate) fn close_modal(&mut self, cx: &mut Context<Self>) {
        self.modal = None;
        cx.notify();
    }

    // ─── Custom Query Execution, Formatting, History & Favorites ────────────

    pub(crate) fn run_custom_query(
        &mut self,
        current_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.active_tab().id;
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let editor = s.query_editor.read(cx);
        let full_sql = editor.value().to_string();
        let selected = editor.selected_text().to_string();
        let cursor_offset = editor.cursor();
        let sql_to_run = if current_only {
            if !selected.trim().is_empty() {
                selected
            } else {
                ace_sql::statement_at_cursor(&full_sql, cursor_offset)
                    .unwrap_or_else(|| full_sql.clone())
            }
        } else {
            full_sql
        };

        if sql_to_run.trim().is_empty() {
            return;
        }
        let conn = s.conn.clone();
        let session_id = s.info.id.to_string();
        let sql_history = sql_to_run.clone();
        if let Some(s) = self.active_tab_mut().session.as_mut() {
            s.busy = true;
            s.main_tab = MainViewTab::CustomQuery;
            s.query_error = None;
            s.query_message = "Running…".into();
        }
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move { run_sql_multi(&conn, &sql_to_run).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));

            this.update_in(cx, |this, _window, cx| {
                if let Some(s) = this.session_by_tab_mut(tab_id) {
                    s.busy = false;
                    match res {
                        Ok(out) => {
                            let ms = out.total_duration_ms;
                            let affected = out.total_affected;
                            let count = out.statement_count;
                            let non_empty_sets: Vec<_> = out
                                .results
                                .into_iter()
                                .filter(|r| !r.columns.is_empty())
                                .collect();
                            s.query_error = None;
                            s.active_result_idx = 0;
                            if let Some(first) = non_empty_sets.first() {
                                let (cols, cells) = query_result_to_cells(first);
                                let rows_n = cells.len();
                                s.query_result_table.update(cx, |t, cx| {
                                    t.clear_selection(cx);
                                    t.delegate_mut().set_data(cols, cells, None, false);
                                    t.refresh(cx);
                                });
                                s.query_message = if non_empty_sets.len() > 1 {
                                    format!(
                                        "{} result sets — {rows_n} rows ({ms} ms)",
                                        non_empty_sets.len()
                                    )
                                } else {
                                    format!("{rows_n} rows ({ms} ms)")
                                };
                            } else {
                                s.query_result_table.update(cx, |t, cx| {
                                    t.delegate_mut().clear();
                                    t.refresh(cx);
                                });
                                s.query_message = format!(
                                    "OK — {count} statement(s), {affected} affected ({ms} ms)"
                                );
                            }
                            s.query_results = non_empty_sets;

                            if let Ok(store) = HistoryStore::open() {
                                let _ = store.add_history(
                                    Some(&session_id),
                                    &sql_history,
                                    true,
                                    ms,
                                    None,
                                );
                                this.query_history = store.recent(50).unwrap_or_default();
                            }
                        }
                        Err(e) => {
                            let err_s = e.to_string();
                            s.query_error = Some(err_s.clone());
                            s.query_message = "Query failed".into();
                            s.query_results.clear();
                            if let Ok(store) = HistoryStore::open() {
                                let _ = store.add_history(
                                    Some(&session_id),
                                    &sql_history,
                                    false,
                                    0,
                                    Some(&err_s),
                                );
                                this.query_history = store.recent(50).unwrap_or_default();
                            }
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn select_query_result_set(&mut self, idx: usize, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab_mut().session.as_mut() else {
            return;
        };
        if let Some(res) = s.query_results.get(idx) {
            s.active_result_idx = idx;
            let (cols, cells) = query_result_to_cells(res);
            s.query_result_table.update(cx, |t, cx| {
                t.clear_selection(cx);
                t.delegate_mut().set_data(cols, cells, None, false);
                t.refresh(cx);
            });
        }
        cx.notify();
    }

    pub(crate) fn format_query_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self
            .active_tab()
            .session
            .as_ref()
            .map(|s| s.query_editor.clone())
        else {
            return;
        };
        let raw = editor.read(cx).value().to_string();
        let formatted = ace_sql::format_sql(&raw);
        editor.update(cx, |e, cx| e.set_value(formatted, window, cx));
    }

    pub(crate) fn explain_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self
            .active_tab()
            .session
            .as_ref()
            .map(|s| s.query_editor.clone())
        else {
            return;
        };
        let raw = editor.read(cx).value().to_string();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return;
        }
        let explained = if trimmed.to_uppercase().starts_with("EXPLAIN") {
            trimmed.to_string()
        } else {
            format!("EXPLAIN {trimmed}")
        };
        editor.update(cx, |e, cx| e.set_value(explained, window, cx));
        self.run_custom_query(false, window, cx);
    }

    pub(crate) fn load_sql_into_editor(
        &mut self,
        sql: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self
            .active_tab()
            .session
            .as_ref()
            .map(|s| s.query_editor.clone())
        {
            editor.update(cx, |e, cx| e.set_value(sql, window, cx));
        }
    }

    pub(crate) fn open_save_query_favorite_modal(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let sql = s.query_editor.read(cx).value().to_string();
        if sql.trim().is_empty() {
            return;
        }
        self.modal = Some(ModalState::SaveQueryFavorite {
            name: new_input(window, cx, "", "Favorite query name", false),
            sql,
        });
        cx.notify();
    }

    pub(crate) fn commit_save_query_favorite(&mut self, cx: &mut Context<Self>) {
        let Some(ModalState::SaveQueryFavorite { name, sql }) = self.modal.take() else {
            return;
        };
        let n = read_input_val(&name, cx).trim().to_string();
        if n.is_empty() {
            return;
        }
        if let Ok(store) = HistoryStore::open() {
            let _ = store.add_favorite(&n, &sql);
            self.query_favorites = store.list_favorites().unwrap_or_default();
        }
        cx.notify();
    }

    pub(crate) fn delete_query_favorite(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if let Ok(store) = HistoryStore::open() {
            let _ = store.delete_favorite(id);
            self.query_favorites = store.list_favorites().unwrap_or_default();
        }
        cx.notify();
    }

    pub(crate) fn delete_query_history_entry(&mut self, id: i64, cx: &mut Context<Self>) {
        if let Ok(store) = HistoryStore::open() {
            let _ = store.delete_history(id);
            self.query_history = store.recent(50).unwrap_or_default();
        }
        cx.notify();
    }

    pub(crate) fn clear_query_history(&mut self, cx: &mut Context<Self>) {
        if let Ok(store) = HistoryStore::open() {
            let _ = store.clear_history();
            self.query_history.clear();
        }
        cx.notify();
    }

    pub(crate) fn clear_query_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = self
            .active_tab()
            .session
            .as_ref()
            .map(|s| s.query_editor.clone())
        {
            editor.update(cx, |e, cx| e.set_value("", window, cx));
        }
    }

    // ─── Export / Import / Server Tools ─────────────────────────────────────

    pub(crate) fn open_export_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(s) = self.active_tab().session.as_ref() else {
            return;
        };
        let (source_label, table_name, result) = if s.main_tab == MainViewTab::CustomQuery {
            if let Some(res) = s.query_results.get(s.active_result_idx).cloned() {
                ("Query Result".into(), None, res)
            } else {
                self.set_session_status("No query result to export", cx);
                return;
            }
        } else if let Some(res) = s.content_raw.clone() {
            let tbl = s.selected_table.clone();
            (
                tbl.clone().unwrap_or_else(|| "Table".into()),
                tbl,
                res,
            )
        } else {
            self.set_session_status("Select a table or run a query first", cx);
            return;
        };
        let default_name = table_name.as_deref().unwrap_or("export");
        let default_path = expand_home(&format!("~/Desktop/{default_name}.sql"));
        self.modal = Some(ModalState::Export {
            source_label,
            table_name,
            result,
            path: new_input(window, cx, default_path, "~/Desktop/export.sql", false),
            format: ExportFormat::Sql,
            gzip: false,
            status: None,
        });
        cx.notify();
    }

    pub(crate) fn set_export_format(
        &mut self,
        fmt: ExportFormat,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ext = match fmt {
            ExportFormat::Sql => "sql",
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Xml => "xml",
            ExportFormat::Html => "html",
            ExportFormat::Dot => "dot",
        };
        let path_ent = match self.modal.as_mut() {
            Some(ModalState::Export { format, path, .. }) => {
                *format = fmt;
                path.clone()
            }
            _ => return,
        };
        let cur = read_input_val(&path_ent, cx);
        let updated = match cur.rfind('.') {
            Some(pos) => format!("{}.{ext}", &cur[..pos]),
            None => format!("{cur}.{ext}"),
        };
        set_input_val(&path_ent, updated, window, cx);
        cx.notify();
    }

    pub(crate) fn commit_export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (path_str, result, table_name, format, gzip) = match &self.modal {
            Some(ModalState::Export {
                path,
                result,
                table_name,
                format,
                gzip,
                ..
            }) => (
                read_input_val(path, cx),
                result.clone(),
                table_name.clone(),
                *format,
                *gzip,
            ),
            _ => return,
        };
        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move {
                    export_data(&path_str, &result, table_name.as_deref(), format, gzip).await
                })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Io(e.to_string())));

            this.update_in(cx, |this, _window, cx| {
                if let Some(ModalState::Export { status, .. }) = this.modal.as_mut() {
                    *status = Some(match res {
                        Ok(n) => format!("Exported {n} rows successfully."),
                        Err(e) => format!("Export failed: {e}"),
                    });
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn open_import_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.modal = Some(ModalState::ImportSql {
            path: new_input(window, cx, "", "~/Desktop/dump.sql", false),
            running: false,
            status: None,
        });
        cx.notify();
    }

    pub(crate) fn commit_import_sql(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let path_str = match self.modal.as_mut() {
            Some(ModalState::ImportSql {
                path,
                running,
                status,
            }) => {
                if *running {
                    return;
                }
                *running = true;
                *status = Some("Importing…".into());
                read_input_val(path, cx)
            }
            _ => return,
        };
        let Some(conn) = self
            .active_tab()
            .session
            .as_ref()
            .map(|s| s.conn.clone())
        else {
            return;
        };
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move { import_sql_file(&conn, &path_str).await })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Io(e.to_string())));

            this.update_in(cx, |this, window, cx| {
                if let Some(ModalState::ImportSql {
                    running, status, ..
                }) = this.modal.as_mut()
                {
                    *running = false;
                    match res {
                        Ok((n, ms)) => {
                            *status = Some(format!("Imported {n} statement(s) in {ms} ms."));
                            this.refresh_current_database_objects(window, cx);
                        }
                        Err(e) => *status = Some(format!("Import failed: {e}")),
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn open_server_tools_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let proc_table = cx.new(|cx| {
            TableState::new(AceGridDelegate::new("No active processes", false), window, cx)
                .row_selectable(true)
                .col_resizable(true)
        });
        let proc_sub = cx.subscribe_in(
            &proc_table,
            window,
            |this, _, ev: &TableEvent, _window, cx| {
                if let TableEvent::SelectRow(r) = ev {
                    if let Some(ModalState::ServerTools {
                        selected_proc_row, ..
                    }) = this.modal.as_mut()
                    {
                        *selected_proc_row = Some(*r);
                    }
                    cx.notify();
                }
            },
        );
        self._subscriptions.push(proc_sub);

        let var_table = cx.new(|cx| {
            TableState::new(AceGridDelegate::new("No server variables", false), window, cx)
                .row_selectable(true)
                .col_resizable(true)
        });
        let user_table = cx.new(|cx| {
            TableState::new(AceGridDelegate::new("No users visible", false), window, cx)
                .row_selectable(true)
                .col_resizable(true)
        });

        let var_filter = new_input(window, cx, "", "Filter variables…", false);
        let vf_sub = cx.subscribe_in(
            &var_filter,
            window,
            |this, _, ev: &InputEvent, _window, cx| {
                if matches!(ev, InputEvent::Change | InputEvent::PressEnter { .. }) {
                    this.filter_server_variables(cx);
                }
            },
        );
        self._subscriptions.push(vf_sub);

        self.modal = Some(ModalState::ServerTools {
            tab: ServerToolsTab::Processes,
            loading: true,
            processes: Vec::new(),
            variables: Vec::new(),
            var_filter,
            users: Vec::new(),
            proc_table,
            var_table,
            user_table,
            selected_proc_row: None,
            error: None,
        });
        cx.notify();
        self.refresh_server_tools(window, cx);
    }

    pub(crate) fn refresh_server_tools(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(conn) = self
            .active_tab()
            .session
            .as_ref()
            .map(|s| s.conn.clone())
        else {
            return;
        };
        if let Some(ModalState::ServerTools { loading, error, .. }) = self.modal.as_mut() {
            *loading = true;
            *error = None;
        }
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let (procs, vars, users) = tokio_rt()
                .spawn(async move {
                    let p = conn.process_list().await.unwrap_or_default();
                    let v = conn.server_variables().await.unwrap_or_default();
                    let u = conn.list_users().await.unwrap_or_default();
                    (p, sorted_variables(v), u)
                })
                .await
                .unwrap_or_default();

            this.update_in(cx, |this, _window, cx| {
                if let Some(ModalState::ServerTools {
                    loading,
                    processes,
                    variables,
                    users: ulist,
                    proc_table,
                    user_table,
                    ..
                }) = this.modal.as_mut()
                {
                    *loading = false;
                    *processes = procs.clone();
                    *variables = vars;
                    *ulist = users.clone();

                    let p_headers = vec![
                        "ID".into(),
                        "User".into(),
                        "Host".into(),
                        "DB".into(),
                        "Command".into(),
                        "Time (s)".into(),
                        "State".into(),
                        "Info".into(),
                    ];
                    let p_rows: Vec<Vec<GridCell>> = procs
                        .into_iter()
                        .map(|p| {
                            vec![
                                GridCell::text(p.id.to_string()),
                                GridCell::text(p.user),
                                GridCell::text(p.host),
                                GridCell::text(p.database.unwrap_or_default()),
                                GridCell::text(p.command),
                                GridCell::text(p.time_secs.to_string()),
                                GridCell::text(p.state.unwrap_or_default()),
                                GridCell::text(p.info.unwrap_or_default()),
                            ]
                        })
                        .collect();
                    proc_table.update(cx, |t, cx| {
                        t.delegate_mut().set_data(p_headers, p_rows, None, false);
                        t.refresh(cx);
                    });

                    let u_headers = vec!["User".into(), "Host".into()];
                    let u_rows: Vec<Vec<GridCell>> = users
                        .into_iter()
                        .map(|u| vec![GridCell::text(u.user), GridCell::text(u.host)])
                        .collect();
                    user_table.update(cx, |t, cx| {
                        t.delegate_mut().set_data(u_headers, u_rows, None, false);
                        t.refresh(cx);
                    });
                }
                this.filter_server_variables(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn filter_server_variables(&mut self, cx: &mut Context<Self>) {
        let Some(ModalState::ServerTools {
            variables,
            var_filter,
            var_table,
            ..
        }) = self.modal.as_ref()
        else {
            return;
        };
        let q = read_input_val(var_filter, cx).to_lowercase();
        let headers = vec!["Variable".into(), "Value".into()];
        let rows: Vec<Vec<GridCell>> = variables
            .iter()
            .filter(|(k, v)| q.is_empty() || k.to_lowercase().contains(&q) || v.to_lowercase().contains(&q))
            .map(|(k, v)| vec![GridCell::text(k.clone()), GridCell::text(v.clone())])
            .collect();
        var_table.update(cx, |t, cx| {
            t.delegate_mut().set_data(headers, rows, None, false);
            t.refresh(cx);
        });
    }

    pub(crate) fn kill_selected_process(
        &mut self,
        kill_conn: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pid = match &self.modal {
            Some(ModalState::ServerTools {
                processes,
                selected_proc_row: Some(r),
                ..
            }) => processes.get(*r).map(|p| p.id),
            _ => None,
        };
        let Some(pid) = pid else {
            return;
        };
        let Some(conn) = self
            .active_tab()
            .session
            .as_ref()
            .map(|s| s.conn.clone())
        else {
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let res = tokio_rt()
                .spawn(async move {
                    if kill_conn {
                        conn.kill_connection(pid).await
                    } else {
                        conn.kill_query(pid).await
                    }
                })
                .await
                .unwrap_or_else(|e| Err(ace_core::AceError::Query(e.to_string())));
            this.update_in(cx, |this, window, cx| {
                if let Err(e) = res {
                    if let Some(ModalState::ServerTools { error, .. }) = this.modal.as_mut() {
                        *error = Some(e.to_string());
                    }
                } else {
                    this.refresh_server_tools(window, cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

fn apply_structure_to_session(
    session: &mut WorkspaceSession,
    meta: TableMeta,
    triggers: Vec<TriggerMeta>,
    window: &mut Window,
    cx: &mut Context<AceApp>,
) {
    let (sc, sr) = structure_column_rows(&meta.columns);
    session.structure_table.update(cx, |t, cx| {
        t.clear_selection(cx);
        t.delegate_mut().set_data(sc, sr, None, false);
        t.refresh(cx);
    });
    let (ic, ir) = index_rows(&meta.indexes);
    session.index_table.update(cx, |t, cx| {
        t.clear_selection(cx);
        t.delegate_mut().set_data(ic, ir, None, false);
        t.refresh(cx);
    });
    let (rc, rr) = relation_rows(&meta.foreign_keys);
    session.relation_table.update(cx, |t, cx| {
        t.clear_selection(cx);
        t.delegate_mut().set_data(rc, rr, None, false);
        t.refresh(cx);
    });
    let (tc, tr) = trigger_rows(&triggers);
    session.trigger_table.update(cx, |t, cx| {
        t.clear_selection(cx);
        t.delegate_mut().set_data(tc, tr, None, false);
        t.refresh(cx);
    });
    let comment = meta.comment.clone().unwrap_or_default();
    let comment_in = session.table_comment_input.clone();
    set_input_val(&comment_in, comment, window, cx);
    let col_hints: Vec<ColumnHint> = meta.columns.iter().map(ColumnHint::from).collect();
    session.schema_catalog.set_table_columns(&meta.name, col_hints);
    session.table_meta = Some(meta);
    session.triggers = triggers;
}

impl Render for AceApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::for_app(cx);
        let is_connected = self.active_tab().session.is_some();

        let tab_bar = self.render_tab_bar(cx);
        let body: AnyElement = if is_connected {
            self.render_workspace_screen(cx)
        } else {
            v_flex()
                .flex_1()
                .min_h_0()
                .w_full()
                .child(self.render_connection_toolbar(cx))
                .child(self.render_connection_screen(cx))
                .into_any_element()
        };
        let modal = self.render_modal_overlay(cx);

        v_flex()
            .id("ace-root")
            .key_context("AceApp")
            .track_focus(&self.focus_handle)
            .relative()
            .size_full()
            .bg(p.window_bg)
            .text_color(p.text)
            .on_action(cx.listener(|this, _: &TabStructure, _, cx| {
                this.set_main_tab(MainViewTab::Structure, cx);
            }))
            .on_action(cx.listener(|this, _: &TabContent, _, cx| {
                this.set_main_tab(MainViewTab::Content, cx);
            }))
            .on_action(cx.listener(|this, _: &TabRelations, _, cx| {
                this.set_main_tab(MainViewTab::Relations, cx);
            }))
            .on_action(cx.listener(|this, _: &TabTriggers, _, cx| {
                this.set_main_tab(MainViewTab::Triggers, cx);
            }))
            .on_action(cx.listener(|this, _: &TabTableInfo, _, cx| {
                this.set_main_tab(MainViewTab::TableInfo, cx);
            }))
            .on_action(cx.listener(|this, _: &TabQuery, _, cx| {
                this.set_main_tab(MainViewTab::CustomQuery, cx);
            }))
            .on_action(cx.listener(|this, _: &RunQueryCurrent, window, cx| {
                if let Some(s) = this.active_tab().session.as_ref() {
                    if s.main_tab == MainViewTab::CustomQuery {
                        this.run_custom_query(true, window, cx);
                    } else {
                        this.reload_selected_table(window, cx);
                    }
                }
            }))
            .on_action(cx.listener(|this, _: &RunQueryAll, window, cx| {
                if let Some(s) = this.active_tab().session.as_ref() {
                    if s.main_tab == MainViewTab::CustomQuery {
                        this.run_custom_query(false, window, cx);
                    } else {
                        this.reload_selected_table(window, cx);
                    }
                }
            }))
            .on_action(cx.listener(|this, _: &NewWindowTab, window, cx| {
                this.new_tab(window, cx);
            }))
            .on_action(cx.listener(|this, _: &CloseWindowTab, window, cx| {
                let idx = this.active_tab;
                this.close_tab(idx, window, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleConsole, _, cx| {
                this.toggle_console(cx);
            }))
            .on_action(cx.listener(|this, _: &DismissModal, _, cx| {
                if this.modal.is_some() {
                    this.close_modal(cx);
                }
            }))
            .child(tab_bar)
            .child(body)
            .when_some(modal, |el, m| el.child(m))
    }
}

pub fn run_gui() {
    application().with_assets(assets::AllAssets).run(|cx| {
        init(cx);
        cx.bind_keys([
            KeyBinding::new("cmd-1", TabStructure, Some("AceApp")),
            KeyBinding::new("cmd-2", TabContent, Some("AceApp")),
            KeyBinding::new("cmd-3", TabRelations, Some("AceApp")),
            KeyBinding::new("cmd-4", TabTriggers, Some("AceApp")),
            KeyBinding::new("cmd-5", TabTableInfo, Some("AceApp")),
            KeyBinding::new("cmd-6", TabQuery, Some("AceApp")),
            KeyBinding::new("cmd-r", RunQueryCurrent, Some("AceApp")),
            KeyBinding::new("cmd-shift-r", RunQueryAll, Some("AceApp")),
            KeyBinding::new("cmd-enter", RunQueryCurrent, Some("AceApp")),
            KeyBinding::new("ctrl-space", ShowCompletions, Some("Input")),
            KeyBinding::new("cmd-t", NewWindowTab, Some("AceApp")),
            KeyBinding::new("cmd-w", CloseWindowTab, Some("AceApp")),
            KeyBinding::new("cmd-shift-c", ToggleConsole, Some("AceApp")),
            KeyBinding::new("escape", DismissModal, Some("AceApp")),
        ]);

        let mut opts = TitleBar::window_options();
        opts.window_bounds = Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(1260.), px(800.)),
            cx,
        )));
        opts.window_min_size = Some(size(px(860.), px(540.)));

        open_window(opts, cx, |window, cx| cx.new(|cx| AceApp::new(window, cx)))
            .expect("Failed to open Ace window");
    });
}
