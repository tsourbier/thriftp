use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::model::DirectoryFavorite;
use crate::validation::validate_directory_favorite;

#[derive(Default)]
pub struct DirectoryFavoriteService {
    favorites: Vec<DirectoryFavorite>,
}

impl DirectoryFavoriteService {
    pub fn load(&mut self, favorites: Vec<DirectoryFavorite>) {
        self.favorites = favorites;
    }

    pub fn snapshot(&self) -> Vec<DirectoryFavorite> {
        self.favorites.clone()
    }

    pub fn list(&self, profile_id: Option<Uuid>) -> Vec<DirectoryFavorite> {
        let mut favorites: Vec<_> = self
            .favorites
            .iter()
            .filter(|favorite| profile_id.is_none_or(|id| favorite.connection_profile_id == id))
            .cloned()
            .collect();
        crate::sorting::sort_directory_favorites(&mut favorites);
        favorites
    }

    pub fn get(&self, id: Uuid) -> CoreResult<DirectoryFavorite> {
        self.favorites
            .iter()
            .find(|favorite| favorite.id == id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(format!("directory favorite {id}")))
    }

    pub fn reorder(&mut self, profile_id: Uuid, ids: &[Uuid]) -> CoreResult<()> {
        let known: std::collections::HashSet<_> = self
            .favorites
            .iter()
            .filter(|favorite| favorite.connection_profile_id == profile_id)
            .map(|favorite| favorite.id)
            .collect();
        if ids.iter().any(|id| !known.contains(id)) {
            return Err(CoreError::NotFound(
                "reorder list contains an unknown directory favorite".to_owned(),
            ));
        }
        for (order, id) in ids.iter().enumerate() {
            if let Some(favorite) = self
                .favorites
                .iter_mut()
                .find(|favorite| favorite.id == *id)
            {
                favorite.sort_order = order as i64;
                favorite.updated_at = chrono::Utc::now();
            }
        }
        Ok(())
    }

    pub fn create(&mut self, favorite: DirectoryFavorite) -> CoreResult<Uuid> {
        validate_directory_favorite(&favorite)?;
        let id = favorite.id;
        self.favorites.push(favorite);
        Ok(id)
    }

    pub fn update(&mut self, favorite: DirectoryFavorite) -> CoreResult<()> {
        validate_directory_favorite(&favorite)?;
        let existing = self
            .favorites
            .iter_mut()
            .find(|candidate| candidate.id == favorite.id)
            .ok_or_else(|| CoreError::NotFound(format!("directory favorite {}", favorite.id)))?;
        *existing = favorite;
        Ok(())
    }

    pub fn delete(&mut self, id: Uuid) -> CoreResult<()> {
        let index = self
            .favorites
            .iter()
            .position(|favorite| favorite.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("directory favorite {id}")))?;
        self.favorites.remove(index);
        Ok(())
    }
}
