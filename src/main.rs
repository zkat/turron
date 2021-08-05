use std::fmt;

use miette::{Diagnostic, DiagnosticReporter, MietteReporter};
use ruget::RuGet;

struct RuGetReport(Box<dyn Diagnostic + Send + Sync + 'static>);
impl fmt::Debug for RuGetReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        MietteReporter.debug(&*self.0, f)
    }
}

#[async_std::main]
async fn main() {
    match RuGet::load().await.map_err(RuGetReport) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{:?}", err);
            std::process::exit(1);
        }
    }
}
