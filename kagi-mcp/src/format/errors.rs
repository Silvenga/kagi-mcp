use thiserror::Error;

#[derive(Error, Debug)]
pub enum FormatError {
    #[error("failed to render template: {0}")]
    TemplateError(#[from] askama::Error),
}
