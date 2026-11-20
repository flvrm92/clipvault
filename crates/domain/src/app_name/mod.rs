//! Executable names identifying source applications (CAP-009, CAP-011).

/// Errors parsing an [`AppName`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppNameError {
    /// No file name could be extracted.
    #[error("application name is blank")]
    Blank,
}

/// A lowercase executable file name such as `keepass.exe`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AppName(String);

impl AppName {
    /// Accepts a bare executable name or a full path; keeps the lowercase file name.
    ///
    /// # Errors
    /// [`AppNameError::Blank`] when no file name remains.
    pub fn parse(value: &str) -> Result<Self, AppNameError> {
        let file_name = value
            .trim()
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or_default()
            .trim();
        if file_name.is_empty() {
            return Err(AppNameError::Blank);
        }
        Ok(Self(file_name.to_lowercase()))
    }

    /// The normalized name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests;
