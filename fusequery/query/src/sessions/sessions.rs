// Copyright 2020-2021 The Datafuse Authors.
//
// SPDX-License-Identifier: Apache-2.0.

use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use common_exception::ErrorCode;
use common_exception::Result;
use common_infallible::RwLock;
use common_management::cluster::ClusterExecutor;
use common_management::cluster::ClusterManager;
use common_management::cluster::ClusterManagerRef;
use common_runtime::tokio;
use common_runtime::tokio::sync::mpsc::Receiver;
use futures::future::Either;
use metrics::counter;

use crate::configs::Config;
use crate::configs::ConfigExtractor;
use crate::datasources::DatabaseCatalog;
use crate::sessions::session::Session;
use crate::sessions::session_ref::SessionRef;

pub struct SessionManager {
    pub(in crate::sessions) conf: Config,
    pub(in crate::sessions) datasource: Arc<DatabaseCatalog>,
    pub(in crate::sessions) cluster_manager: ClusterManagerRef,

    pub(in crate::sessions) max_sessions: usize,
    pub(in crate::sessions) active_sessions: Arc<RwLock<HashMap<String, Arc<Session>>>>,
}

pub type SessionManagerRef = Arc<SessionManager>;

impl SessionManager {
    pub fn from_conf(conf: Config) -> Result<SessionManagerRef> {
        let max_active_sessions = conf.max_active_sessions as usize;
        Ok(Arc::new(SessionManager {
            conf: conf.clone(),
            max_sessions: max_active_sessions,
            datasource: Arc::new(DatabaseCatalog::try_create()?),
            cluster_manager: ClusterManager::from_conf(conf.extract_cluster()),
            active_sessions: Arc::new(RwLock::new(HashMap::with_capacity(max_active_sessions))),
        }))
    }

    pub fn get_datasource(self: &Arc<Self>) -> Arc<DatabaseCatalog> {
        self.datasource.clone()
    }

    pub fn create_session(self: &Arc<Self>, typ: impl Into<String>) -> Result<SessionRef> {
        counter!(super::metrics::METRIC_SESSION_CONNECT_NUMBERS, 1);

        let mut sessions = self.active_sessions.write();
        match sessions.len() == self.max_sessions {
            true => Err(ErrorCode::TooManyUserConnections(
                "The current accept connection has exceeded mysql_handler_thread_num config",
            )),
            false => {
                let session = Session::try_create(
                    self.conf.clone(),
                    uuid::Uuid::new_v4().to_string(),
                    self.clone(),
                )?;

                sessions.insert(session.get_id(), session.clone());
                Ok(SessionRef::create(typ.into(), session))
            }
        }
    }

    pub fn create_rpc_session(self: &Arc<Self>, id: String, aborted: bool) -> Result<SessionRef> {
        counter!(super::metrics::METRIC_SESSION_CONNECT_NUMBERS, 1);

        let mut sessions = self.active_sessions.write();

        let session = match sessions.entry(id) {
            Occupied(entry) => entry.get().clone(),
            Vacant(_) if aborted => return Err(ErrorCode::AbortedSession("Aborting server.")),
            Vacant(entry) => {
                let session =
                    Session::try_create(self.conf.clone(), entry.key().clone(), self.clone())?;

                entry.insert(session).clone()
            }
        };

        Ok(SessionRef::create(String::from("RpcSession"), session))
    }

    #[allow(clippy::ptr_arg)]
    pub fn destroy_session(self: &Arc<Self>, session_id: &String) {
        counter!(super::metrics::METRIC_SESSION_CLOSE_NUMBERS, 1);

        self.active_sessions.write().remove(session_id);
    }

    pub fn shutdown(self: &Arc<Self>, signal: Option<Receiver<()>>) -> impl Future<Output = ()> {
        let active_sessions = self.active_sessions.clone();
        async move {
            log::info!("Waiting for current connections to close.");
            if let Some(mut signal) = signal {
                let mut signal = Box::pin(signal.recv());

                for _index in 0..5 {
                    if SessionManager::destroy_idle_sessions(&active_sessions) {
                        return;
                    }

                    let interval = Duration::from_secs(1);
                    let sleep = Box::pin(tokio::time::sleep(interval));
                    match futures::future::select(sleep, signal).await {
                        Either::Right((_, _)) => break,
                        Either::Left((_, reserve_signal)) => signal = reserve_signal,
                    };
                }
            }

            log::info!("Will shutdown forcefully.");
            active_sessions
                .read()
                .values()
                .for_each(Session::force_kill);
        }
    }

    fn destroy_idle_sessions(sessions: &Arc<RwLock<HashMap<String, Arc<Session>>>>) -> bool {
        // Read lock does not support reentrant
        // https://github.com/Amanieu/parking_lot/blob/lock_api-0.4.4/lock_api/src/rwlock.rs#L422
        let active_sessions_read_guard = sessions.read();

        // First try to kill the idle session
        active_sessions_read_guard.values().for_each(Session::kill);
        let active_sessions = active_sessions_read_guard.len();

        match active_sessions {
            0 => true,
            _ => {
                log::info!("Waiting for {} connections to close.", active_sessions);
                false
            }
        }
    }

    pub fn get_conf(self: &Arc<Self>) -> Config {
        self.conf.clone()
    }

    pub fn get_cluster_manager(self: &Arc<Self>) -> ClusterManagerRef {
        self.cluster_manager.clone()
    }

    pub fn try_get_executors(self: &Arc<Self>) -> Result<Vec<Arc<ClusterExecutor>>> {
        Err(ErrorCode::UnImplement(""))
    }

    pub fn register_executor(self: &Arc<Self>) -> Result<()> {
        Err(ErrorCode::UnImplement(""))
    }

    pub fn unregister_executor(self: &Arc<Self>) -> Result<()> {
        Err(ErrorCode::UnImplement(""))
    }
}
