use std::fmt;

use miette::{Diagnostic, DiagnosticReporter, MietteReporter, Severity};
use thiserror::Error;

#[derive(Error)]
#[error("{:?}", self)]
pub struct DiagnosticError {
    pub error: Box<dyn std::error::Error + Send + Sync + 'static>,
    pub code: String,
}

impl Diagnostic for DiagnosticError {
    fn code(&self) -> &(dyn std::fmt::Display) {
        todo!()
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }
}

impl fmt::Debug for DiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        MietteReporter.debug(self, f)
    }
}

pub type DiagnosticResult<T> = Result<T, Box<dyn Diagnostic + Send +Sync + 'static>>;

pub trait IntoDiagnostic<T, E> {
    fn into_diagnostic(self, code: &(dyn fmt::Display)) -> Result<T, DiagnosticError>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> IntoDiagnostic<T, E> for Result<T, E> {
    fn into_diagnostic(self, code: &(dyn fmt::Display)) -> Result<T, DiagnosticError> {
        self.map_err(|e| DiagnosticError {
            error: Box::new(e),
            code: format!("{}", code),
        })
    }
}
