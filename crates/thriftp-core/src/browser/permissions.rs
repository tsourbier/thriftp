use crate::error::{CoreError, CoreResult};
use crate::model::FileEntry;
use crate::protocol::RemoteFs;

/// Change remote permissions, failing clearly when the protocol cannot.
pub async fn chmod_remote(
    remote: &mut dyn RemoteFs,
    path: &str,
    mode: u32,
) -> CoreResult<FileEntry> {
    if mode > 0o777 {
        return Err(CoreError::validation(
            "permission mode must be between 000 and 777",
        ));
    }
    if !remote.capabilities().permissions {
        return Err(CoreError::UnsupportedCapability(
            "this server does not support permission changes".to_owned(),
        ));
    }
    remote.chmod(path, mode).await?;
    remote.stat(path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ProtocolCapabilities;
    use async_trait::async_trait;

    struct Locked;

    #[async_trait]
    impl RemoteFs for Locked {
        async fn connect(
            &mut self,
            _profile: &crate::model::ConnectionProfile,
            _credential: Option<crate::credentials::Secret>,
        ) -> CoreResult<()> {
            Ok(())
        }
        async fn disconnect(&mut self) -> CoreResult<()> {
            Ok(())
        }
        fn capabilities(&self) -> ProtocolCapabilities {
            ProtocolCapabilities {
                permissions: false,
                ..ProtocolCapabilities::default()
            }
        }
        async fn list_dir(&mut self, _path: &str) -> CoreResult<Vec<FileEntry>> {
            Ok(Vec::new())
        }
        async fn stat(&mut self, path: &str) -> CoreResult<FileEntry> {
            Err(CoreError::NotFound(path.to_owned()))
        }
        async fn mkdir(&mut self, _path: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn rename(&mut self, _from: &str, _to: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn delete_file(&mut self, _path: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn delete_dir(&mut self, _path: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn chmod(&mut self, _path: &str, _mode: u32) -> CoreResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn unsupported_permissions_fail_before_the_protocol_call() {
        let mut remote = Locked;
        let error = chmod_remote(&mut remote, "/file", 0o644).await.unwrap_err();
        assert!(matches!(error, CoreError::UnsupportedCapability(_)));
    }

    #[tokio::test]
    async fn invalid_modes_are_rejected() {
        let mut remote = Locked;
        assert!(chmod_remote(&mut remote, "/file", 0o1000).await.is_err());
    }
}
