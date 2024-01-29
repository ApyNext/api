use tracing::warn;

use super::app_error::AppError;

pub fn check_new_post_data(auth_user_id: i64, title: &str, content: &str) -> Result<(), AppError> {
    if title.len() < 3 || title.len() > 50 {
        warn!(
            "User {} tried to create a post with a title with a wrong length : {}/50",
            auth_user_id,
            title.len()
        );
        return Err(AppError::forbidden_error(Some(
            "Le titre d'un post doit contenir entre 3 et 50 caractères.",
        )));
    }

    if content.len() < 10 || content.len() > 1000 {
        warn!(
            "User {} tried to create a post with a content with a wrong length : {}/1000",
            auth_user_id,
            content.len()
        );
        return Err(AppError::forbidden_error(Some(
            "Le contenu d'un post doit contenir entre 10 et 1 000 caractères.",
        )));
    }

    Ok(())
}
