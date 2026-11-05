use thriftp_core::browser::history;
use thriftp_core::favorites::DirectoryFavoriteService;
use thriftp_core::model::{BrowsingSession, DirectoryFavorite};
use thriftp_core::preferences::tabs::TabService;
use thriftp_core::repository::AppState;
use uuid::Uuid;

#[test]
fn tabs_restore_disconnected_and_respect_the_preference() {
    let profile = Uuid::new_v4();
    let mut state = AppState::default();
    state
        .tabs
        .push(BrowsingSession::new(Some(profile), "/home"));
    state.tabs.push(BrowsingSession::new(None, "/tmp"));

    let restored = TabService::from_state(&state, true);
    assert_eq!(restored.list().len(), 2);
    assert!(restored.list().iter().all(|tab| tab.restored));
    assert_eq!(restored.disconnected_ids(&[]), vec![profile]);

    let disabled = TabService::from_state(&state, false);
    assert!(disabled.list().is_empty());
}

#[test]
fn navigation_history_supports_back_and_forward() {
    let mut session = BrowsingSession::new(None, "/");
    history::navigate(&mut session, "/home");
    history::navigate(&mut session, "/home/docs");
    assert_eq!(history::back(&mut session).as_deref(), Some("/home"));
    assert_eq!(
        history::forward(&mut session).as_deref(),
        Some("/home/docs")
    );

    history::back(&mut session);
    history::navigate(&mut session, "/etc");
    assert!(!history::can_go_forward(&session));
    assert_eq!(session.current_path, "/etc");
}

#[test]
fn directory_favorites_crud_filter_and_reorder() {
    let profile = Uuid::new_v4();
    let other = Uuid::new_v4();
    let mut service = DirectoryFavoriteService::default();
    let first = DirectoryFavorite::new(profile, "/a", "Alpha", 0);
    let second = DirectoryFavorite::new(profile, "/b", "Beta", 1);
    let third = DirectoryFavorite::new(other, "/c", "Gamma", 0);
    let first_id = first.id;
    let second_id = second.id;

    service.create(first).unwrap();
    service.create(second).unwrap();
    service.create(third).unwrap();

    assert_eq!(service.list(Some(profile)).len(), 2);
    assert_eq!(service.list(None).len(), 3);

    service.reorder(profile, &[second_id, first_id]).unwrap();
    assert_eq!(service.list(Some(profile))[0].id, second_id);

    let mut updated = service.get(first_id).unwrap();
    updated.label = "Renamed".to_owned();
    service.update(updated).unwrap();
    assert_eq!(service.get(first_id).unwrap().label, "Renamed");

    service.delete(first_id).unwrap();
    assert!(service.get(first_id).is_err());
    assert!(service.reorder(profile, &[first_id]).is_err());
}

#[test]
fn tab_service_persists_into_metadata_state() {
    let mut service = TabService::default();
    service.save(vec![BrowsingSession::new(None, "/work")]);
    let mut state = AppState::default();
    service.apply_to(&mut state);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].current_path, "/work");
}
