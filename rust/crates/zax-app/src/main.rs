#![forbid(unsafe_code)]

//! The `zaxnes` executable. The command line (`Program.cs`, `AppOptions.cs`) is ported with the
//! command-line front end; until then the program starts with the default options.

use zax_app::app_options::AppOptions;
use zax_app::application::{NoUi, ZaxApplication};

fn main() {
    let code = ZaxApplication::new(AppOptions::default()).run(Box::new(NoUi));
    std::process::exit(code);
}
