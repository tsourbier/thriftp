use crate::model::BrowsingSession;

/// Navigate to a new path and truncate any forward history.
pub fn navigate(session: &mut BrowsingSession, path: impl Into<String>) {
    let path = path.into();
    if session.current_path == path {
        return;
    }
    session.history.truncate(session.history_index + 1);
    session.history.push(path.clone());
    session.history_index = session.history.len() - 1;
    session.current_path = path;
}

#[must_use]
pub fn can_go_back(session: &BrowsingSession) -> bool {
    session.history_index > 0 && !session.history.is_empty()
}

#[must_use]
pub fn can_go_forward(session: &BrowsingSession) -> bool {
    session.history_index + 1 < session.history.len()
}

/// Move back in history and return the new current path.
pub fn back(session: &mut BrowsingSession) -> Option<String> {
    if !can_go_back(session) {
        return None;
    }
    session.history_index -= 1;
    session.current_path = session.history[session.history_index].clone();
    Some(session.current_path.clone())
}

/// Move forward in history and return the new current path.
pub fn forward(session: &mut BrowsingSession) -> Option<String> {
    if !can_go_forward(session) {
        return None;
    }
    session.history_index += 1;
    session.current_path = session.history[session.history_index].clone();
    Some(session.current_path.clone())
}

/// Replace the history with a single entry (for a disconnected restored tab).
pub fn reset(session: &mut BrowsingSession, path: impl Into<String>) {
    let path = path.into();
    session.history = vec![path.clone()];
    session.history_index = 0;
    session.current_path = path;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> BrowsingSession {
        BrowsingSession::new(None, "/start")
    }

    #[test]
    fn navigates_back_and_forward() {
        let mut session = session();
        navigate(&mut session, "/a");
        navigate(&mut session, "/b");
        assert!(can_go_back(&session));
        assert_eq!(back(&mut session).as_deref(), Some("/a"));
        assert!(can_go_forward(&session));
        assert_eq!(forward(&mut session).as_deref(), Some("/b"));
        assert!(!can_go_forward(&session));
    }

    #[test]
    fn new_navigation_truncates_forward_history() {
        let mut session = session();
        navigate(&mut session, "/a");
        navigate(&mut session, "/b");
        back(&mut session);
        navigate(&mut session, "/c");
        assert!(!can_go_forward(&session));
        assert_eq!(session.history, vec!["/start", "/a", "/c"]);
    }

    #[test]
    fn reset_clears_history() {
        let mut session = session();
        navigate(&mut session, "/a");
        reset(&mut session, "/home");
        assert_eq!(session.history, vec!["/home"]);
        assert!(!can_go_back(&session));
    }
}
