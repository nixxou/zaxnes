//! The native file dialog behind the `FilePicker` trait: the interface asks the picker first,
//! takes its answer, and falls back to the `ImGui` browser when the picker cannot show a dialog.

mod ui_harness;

use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

use ui_harness::UiHarness;
use zax_app::emulation::commands::Command;
use zax_app::persistence::app_settings::AppSettings;
use zax_app::ui::file_picker::{FilePicker, PickPoll, PickRequest};

/// A picker the test drives: it reports each request and answers with what the test sends.
struct FakePicker {
    available: bool,
    requests: Sender<PickRequest>,
    answers: Receiver<PickPoll>,
}

impl FilePicker for FakePicker {
    fn start(&mut self, request: PickRequest) -> bool {
        if self.available {
            self.requests.send(request).ok();
        }
        self.available
    }

    fn poll(&mut self) -> PickPoll {
        match self.answers.try_recv() {
            Ok(poll) => poll,
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => PickPoll::Pending,
        }
    }
}

struct Scripted {
    harness: UiHarness,
    requests: Receiver<PickRequest>,
    answers: Sender<PickPoll>,
}

fn scripted(available: bool) -> Scripted {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let (requests_in, requests) = channel();
    let (answers, answers_out) = channel();
    harness.ui.set_file_picker(Box::new(FakePicker {
        available,
        requests: requests_in,
        answers: answers_out,
    }));
    Scripted {
        harness,
        requests,
        answers,
    }
}

impl Scripted {
    fn answer(&mut self, poll: PickPoll) {
        self.answers.send(poll).ok();
        self.harness.ui.poll_file_picker(&mut self.harness.host);
    }
}

#[test]
fn the_native_dialog_replaces_the_browser_and_its_answer_is_used() {
    let mut test = scripted(true);
    let rom = test.harness.write_rom("game.nes");
    test.harness.run(Command::OpenRom);

    assert!(test.harness.ui.state.native_dialog);
    assert!(test.harness.ui.state.is_modal());
    assert!(!test.harness.ui.state.file_browser);
    let request = test.requests.try_recv().expect("the picker was asked");
    assert!(!request.folder);
    assert_eq!(["nes", "zip"], request.filters[0].extensions[..]);
    assert!(request.filters.iter().any(|filter| filter.extensions.is_empty()));

    test.answer(PickPoll::Pending);
    assert!(test.harness.ui.state.native_dialog);

    test.answer(PickPoll::Chosen(rom.clone()));
    assert!(!test.harness.ui.state.native_dialog);
    assert!(!test.harness.ui.state.file_browser);
    let folder = std::path::Path::new(&rom).parent().expect("a folder");
    assert_eq!(
        folder.to_string_lossy(),
        test.harness.host.settings().last_browse_directory()
    );
}

#[test]
fn a_cancelled_native_dialog_opens_nothing() {
    let mut test = scripted(true);
    test.harness.run(Command::ImportProfile);
    test.answer(PickPoll::Cancelled);

    assert!(!test.harness.ui.state.native_dialog);
    assert!(!test.harness.ui.state.file_browser);
    assert!(!test.harness.ui.take_effects().is_empty() || test.harness.ui.state.any_window_open() == false);
}

#[test]
fn a_dialog_that_cannot_be_shown_falls_back_to_the_browser() {
    let mut test = scripted(true);
    test.harness.run(Command::ImportCover);
    test.answer(PickPoll::Failed);

    assert!(!test.harness.ui.state.native_dialog);
    assert!(test.harness.ui.state.file_browser);
    assert_eq!(["png"], test.harness.ui.browser.extensions());
}

#[test]
fn an_unavailable_picker_leaves_the_browser_as_before() {
    let mut test = scripted(false);
    test.harness.run(Command::OpenRom);

    assert!(!test.harness.ui.state.native_dialog);
    assert!(test.harness.ui.state.file_browser);
    assert!(test.requests.try_recv().is_err());
}

#[test]
fn the_three_d_sen_import_asks_for_a_folder() {
    let mut test = scripted(true);
    test.harness.run(Command::ImportThreeD Sen);
}
