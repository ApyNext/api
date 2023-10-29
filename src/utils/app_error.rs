use axum::response::{IntoResponse, Response};
use hyper::StatusCode;

pub enum AppError {
    //Token decoding errors
    InvalidToken,
    ExpiredToken,
    //Email confirm route errors
    TokenMissing,
    InternalServerError,
    //Check register infos errors
    IncorrectUsernameLength,
    UsernameMustBeginByALetter,
    UsernameMustOnlyContainLettersDigitsAndUnderscores,
    InvalidEmail,
    PasswordTooShort,
//TODO    BiographyTooLong,
    //Register route errors
    InvalidBirthdate,
    EmailAddressAlreadyUsed,
    UsernameAlreadyUsed,
    //Send email errors
    EmailSendError,
    //Login route errors
    IncorrectCredentials,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = match self {
            AppError::InvalidToken => "Token invalide",
            AppError::ExpiredToken => "Token expiré",
            AppError::TokenMissing => "Token manquant",
            AppError::InternalServerError => "",
            AppError::IncorrectUsernameLength => "Le nom d'utilisateur doit contenir entre 5 et 12 caractères compris.",
            AppError::UsernameMustBeginByALetter => "Le nom d'utilisateur doit commencer par une lettre.",
            AppError::UsernameMustOnlyContainLettersDigitsAndUnderscores => "Le nom d'utilisateur ne peut contenir que des lettres, des chiffres et des underscores.",
            AppError::InvalidEmail => "L'adresse email n'est pas valide.",
            AppError::PasswordTooShort => "Le mot de passe doit contenir au moins 8 caractères.",
     //TODO       AppError::BiographyTooLong => "La biographie doit contenir au maximum 300 caractères.",
            AppError::InvalidBirthdate => "Date de naissance invalide",
            AppError::EmailAddressAlreadyUsed => "Adresse email déjà utilisée",
            AppError::UsernameAlreadyUsed => "Pseudo déjà utilisé",
            AppError::EmailSendError => "Erreur lors de l'envoi de l'email de vérification.",
            AppError::IncorrectCredentials => "Identifiants invalides",
        };

        let status_code = match self {
            AppError::InvalidToken
            | AppError::ExpiredToken
            | AppError::TokenMissing
            | AppError::IncorrectUsernameLength
            | AppError::UsernameMustBeginByALetter
            | AppError::UsernameMustOnlyContainLettersDigitsAndUnderscores
            | AppError::InvalidEmail
            | AppError::PasswordTooShort
          //TODO  | AppError::BiographyTooLong
            | AppError::InvalidBirthdate
            | AppError::EmailAddressAlreadyUsed
            | AppError::EmailSendError
            | AppError::UsernameAlreadyUsed
            | AppError::IncorrectCredentials => StatusCode::FORBIDDEN,
            AppError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        };

        if body.is_empty() {
            return status_code.into_response();
        }

        (status_code, body).into_response()
    }
}
