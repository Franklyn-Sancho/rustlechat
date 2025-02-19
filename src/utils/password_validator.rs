// utils/password_validator.rs

use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

/// Errors that can occur during password validation
#[derive(Error, Debug)]
pub enum PasswordValidationError {
    #[error("Password must be at least {0} characters long")]
    TooShort(usize),
    #[error("Password must contain at least one uppercase letter")]
    NoUppercase,
    #[error("Password must contain at least one lowercase letter")]
    NoLowercase,
    #[error("Password must contain at least one number")]
    NoNumber,
    #[error("Password must contain at least one special character")]
    NoSpecialChar,
}

static UPPERCASE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[A-Z]").unwrap());
static LOWERCASE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[a-z]").unwrap());
static NUMBER_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\d").unwrap());
static SPECIAL_CHAR_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r##"[!@#$%^&*(),.?":{}|<>]"##).unwrap());


pub struct PasswordValidator {
    min_length: usize,
}

impl Default for PasswordValidator {
    fn default() -> Self {
        Self { min_length: 8 }
    }
}

impl PasswordValidator {
    /// Creates a new password validator with custom minimum length
    pub fn new(min_length: usize) -> Self {
        Self { min_length }
    }

    /// Validates a password and returns a Result with detailed error if validation fails
    pub fn validate_with_details(&self, password: &str) -> Result<(), PasswordValidationError> {
        if password.len() < self.min_length {
            return Err(PasswordValidationError::TooShort(self.min_length));
        }
        if !UPPERCASE_REGEX.is_match(password) {
            return Err(PasswordValidationError::NoUppercase);
        }
        if !LOWERCASE_REGEX.is_match(password) {
            return Err(PasswordValidationError::NoLowercase);
        }
        if !NUMBER_REGEX.is_match(password) {
            return Err(PasswordValidationError::NoNumber);
        }
        if !SPECIAL_CHAR_REGEX.is_match(password) {
            return Err(PasswordValidationError::NoSpecialChar);
        }

        Ok(())
    }

    /// Simple validation that returns true if password meets all requirements
    pub fn validate(password: &str) -> bool {
        Self::default().validate_with_details(password).is_ok()
    }

    /// Returns a string describing all password requirements
    pub const fn requirements() -> &'static str {
        "Password must:
         - Be at least 8 characters long
         - Contain at least one uppercase letter
         - Contain at least one lowercase letter
         - Contain at least one number
         - Contain at least one special character (!@#$%^&*(),.?\":{}|<>)"
    }
}


