use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tracing::Instrument;
use uuid::Uuid;

use crate::credentials::Secret;
use crate::error::{CoreError, CoreResult};
use crate::model::{ActiveConnection, ConnectionProfile};
use crate::protocol::{RemoteFs, RemoteFsFactory};
use crate::validation::validate_profile;

/// A live protocol session shared by browsing and transfer workers.
pub type SharedSession = Arc<tokio::sync::Mutex<Box<dyn RemoteFs>>>;

pub struct ConnectionService {
    factory: Box<dyn RemoteFsFactory>,
    profiles: HashMap<Uuid, ConnectionProfile>,
    active: HashMap<Uuid, ActiveConnection>,
    sessions: HashMap<Uuid, SharedSession>,
}

impl ConnectionService {
    pub fn new(factory: Box<dyn RemoteFsFactory>) -> Self {
        Self {
            factory,
            profiles: HashMap::new(),
            active: HashMap::new(),
            sessions: HashMap::new(),
        }
    }

    pub fn load_profiles(&mut self, profiles: Vec<ConnectionProfile>) {
        for profile in profiles {
            if validate_profile(&profile).is_ok() {
                self.profiles.insert(profile.id, profile);
            }
        }
    }

    pub fn list_profiles(&self) -> Vec<ConnectionProfile> {
        let mut profiles: Vec<_> = self.profiles.values().cloned().collect();
        crate::sorting::sort_profiles(&mut profiles);
        profiles
    }

    pub fn get_profile(&self, id: Uuid) -> CoreResult<ConnectionProfile> {
        self.profiles
            .get(&id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(format!("connection profile {id}")))
    }

    pub fn create_profile(&mut self, profile: ConnectionProfile) -> CoreResult<Uuid> {
        validate_profile(&profile)?;
        if self.profiles.contains_key(&profile.id) {
            return Err(CoreError::Conflict(
                "connection profile already exists".to_owned(),
            ));
        }
        let id = profile.id;
        self.profiles.insert(id, profile);
        Ok(id)
    }

    pub fn update_profile(&mut self, profile: ConnectionProfile) -> CoreResult<()> {
        validate_profile(&profile)?;
        if !self.profiles.contains_key(&profile.id) {
            return Err(CoreError::NotFound(format!(
                "connection profile {}",
                profile.id
            )));
        }
        self.profiles.insert(profile.id, profile);
        Ok(())
    }

    pub fn delete_profile(&mut self, id: Uuid) -> CoreResult<()> {
        self.profiles
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| CoreError::NotFound(format!("connection profile {id}")))
    }

    #[tracing::instrument(skip(self, credential), fields(profile_id = %profile_id))]
    pub async fn connect(
        &mut self,
        profile_id: Uuid,
        credential: Option<Secret>,
    ) -> CoreResult<ActiveConnection> {
        let profile = self.get_profile(profile_id)?;
        let mut session = self.factory.create(profile.protocol);
        session
            .connect(&profile, credential)
            .instrument(tracing::debug_span!("connection.open_session"))
            .await?;
        let active = ActiveConnection::new(profile.id, profile.protocol);
        self.sessions
            .insert(active.id, Arc::new(tokio::sync::Mutex::new(session)));
        self.active.insert(active.id, active.clone());
        if let Some(stored) = self.profiles.get_mut(&profile_id) {
            stored.last_used_at = Some(Utc::now());
        }
        tracing::info!(
            connection_id = %active.id,
            profile_id = %profile_id,
            protocol = ?profile.protocol,
            "connection established"
        );
        Ok(active)
    }

    #[tracing::instrument(skip(self, credential), fields(profile_id = %profile_id))]
    pub async fn test(&mut self, profile_id: Uuid, credential: Option<Secret>) -> CoreResult<()> {
        let profile = self.get_profile(profile_id)?;
        let mut session = self.factory.create(profile.protocol);
        session.connect(&profile, credential).await?;
        session.disconnect().await?;
        Ok(())
    }

    #[tracing::instrument(skip(self), fields(connection_id = %connection_id))]
    pub async fn disconnect(&mut self, connection_id: Uuid) -> CoreResult<()> {
        let session = self
            .sessions
            .remove(&connection_id)
            .ok_or_else(|| CoreError::NotFound(format!("active connection {connection_id}")))?;
        session.lock().await.disconnect().await?;
        self.active.remove(&connection_id);
        tracing::info!(connection_id = %connection_id, "connection closed");
        Ok(())
    }

    #[tracing::instrument(skip(self, credential), fields(connection_id = %connection_id))]
    pub async fn reconnect(
        &mut self,
        connection_id: Uuid,
        credential: Option<Secret>,
    ) -> CoreResult<ActiveConnection> {
        let profile_id = self
            .active
            .get(&connection_id)
            .map(|connection| connection.connection_profile_id)
            .ok_or_else(|| CoreError::NotFound(format!("active connection {connection_id}")))?;
        let session = self
            .sessions
            .remove(&connection_id)
            .ok_or_else(|| CoreError::NotFound(format!("active connection {connection_id}")))?;
        let _ = session.lock().await.disconnect().await;
        self.active.remove(&connection_id);
        self.connect(profile_id, credential).await
    }

    pub fn active_connections(&self) -> Vec<ActiveConnection> {
        let mut active: Vec<_> = self.active.values().cloned().collect();
        active.sort_by_key(|connection| connection.started_at);
        active
    }

    pub fn session(&self, connection_id: Uuid) -> CoreResult<SharedSession> {
        self.sessions
            .get(&connection_id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(format!("active connection {connection_id}")))
    }

    pub fn active_for_profile(&self, profile_id: Uuid) -> Option<Uuid> {
        self.active
            .values()
            .filter(|connection| connection.connection_profile_id == profile_id)
            .map(|connection| connection.id)
            .next()
    }
}
