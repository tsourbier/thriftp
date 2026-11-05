use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::State;
use thriftp_core::model::TransferState;

use crate::AppState;
use crate::error::DesktopError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuitPreparationView {
    pub active_transfers: usize,
    pub cancelled_transfers: usize,
}

/// Pause the queue, give active transfers a short grace period to stop at a
/// chunk boundary, then cancel anything still running.
pub async fn prepare_quit(state: &AppState) -> QuitPreparationView {
    {
        let guard = state.transfer_host.lock().await;
        if let Some(runtime) = guard.as_ref()
            && !runtime.is_host()
        {
            // The desktop is an observer; never cancel transfers owned by
            // another host.
            return QuitPreparationView {
                active_transfers: 0,
                cancelled_transfers: 0,
            };
        }
    }
    let mut manager = state.transfers.lock().await;
    manager.queue.pause();
    manager.pause_all();

    let started = Instant::now();
    let grace = Duration::from_secs(2);
    loop {
        let active = manager
            .queue
            .list()
            .into_iter()
            .filter(|transfer| transfer.state == TransferState::Active)
            .count();
        if active == 0 || started.elapsed() >= grace {
            break;
        }
        drop(manager);
        tokio::time::sleep(Duration::from_millis(50)).await;
        manager = state.transfers.lock().await;
    }

    // Preserve interrupted, resumable state instead of silently cancelling.
    let active_ids: Vec<_> = manager
        .queue
        .list()
        .into_iter()
        .filter(|transfer| transfer.state == TransferState::Active)
        .map(|transfer| transfer.id)
        .collect();
    let interrupted = active_ids.len();
    for id in active_ids {
        let _ = manager.queue.mark_state(id, TransferState::Interrupted);
    }
    let _ = manager.persist();

    QuitPreparationView {
        active_transfers: interrupted,
        cancelled_transfers: 0,
    }
}

#[tauri::command]
pub async fn lifecycle_prepare_quit(
    state: State<'_, AppState>,
) -> Result<QuitPreparationView, DesktopError> {
    Ok(prepare_quit(&state).await)
}
