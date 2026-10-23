#![forbid(unsafe_code)]

//! The `zaxnes` executable. The command line (`Program.cs`, `AppOptions.cs`) is ported with the
//! command-line front end; until then the program starts with the default options and reads
//! only `--ui-shots <folder>`, the capture of the interface screens.

use zax_app::app_options::AppOptions;
use zax_app::application::ZaxApplication;

fn main() {
    let mut options = AppOptions::default();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--ui-shots" {
            options.ui_shot_directory = arguments.next();
        }
    }
    let code = ZaxApplication::new(options).run();
    std::process::exit(code);
}
