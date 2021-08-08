use std::fmt;

use ruget::RuGet;
use ruget_common::{
    miette::{Diagnostic, DiagnosticReporter, MietteReporter},
    smol,
};

struct RuGetReport(Box<dyn Diagnostic + Send + Sync + 'static>);
impl fmt::Debug for RuGetReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        MietteReporter.debug(&*self.0, f)
    }
}

fn main() {
    smol::block_on(async {
        match RuGet::load().await.map_err(RuGetReport) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{:?}", err);
                std::process::exit(1);
            }
        }
    });
}
