//! The client against a fake server: sign-in, game load, an unlock in softcore, save states.
#![allow(clippy::panic)] // test code

use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use zax_achievements::{Bucket, Client, Event, HttpRequest, HttpResponse, Memory, Transport};

const LOGIN: &str = r#"{"Success":true,"User":"Player","Token":"ApiToken","Score":12345,"SoftcoreScore":123,"Messages":0,"Permissions":1,"AccountType":"Registered"}"#;

const ACHIEVEMENT_SETS: &str = r#"{"Success":true,"GameId":1234,"Title":"Sample Game","ConsoleId":7,"ImageIconUrl":"http://server/Images/112233.png","RichPresenceGameId":1234,"RichPresencePatch":"Display:\nLevel @Number(0xH0003)\n\nFormat:Number\nFormatType=VALUE\n","Sets":[{"AchievementSetId":1111,"GameId":1234,"Title":null,"Type":"core","ImageIconUrl":"http://server/Images/112233.png","Achievements":[{"ID":5501,"Title":"Ach1","Description":"Desc1","Flags":3,"Points":5,"MemAddr":"0xH0001=3_0xH0002=7","Author":"User1","BadgeName":"00234","Created":1367266583,"Modified":1376929305},{"ID":5502,"Title":"Ach2","Description":"Desc2","Flags":3,"Points":2,"MemAddr":"0xH0004=1.10.","Author":"User1","BadgeName":"00235","Created":1376970283,"Modified":1376970283}],"Leaderboards":[]}]}"#;

/// Answers each request by its `r=` and records it.
#[derive(Default)]
struct FakeServer {
    seen: Mutex<Vec<HttpRequest>>,
}

impl Transport for FakeServer {
    fn send(&self, request: &HttpRequest) -> HttpResponse {
        if let Ok(mut seen) = self.seen.lock() {
            seen.push(request.clone());
        }
        let form = request.post_data.clone().unwrap_or_default();
        let body = if form.starts_with("r=login2") {
            LOGIN
        } else if form.starts_with("r=achievementsets") {
            ACHIEVEMENT_SETS
        } else if form.starts_with("r=awardachievement") {
            r#"{"Success":true,"Score":12345,"SoftcoreScore":128,"AchievementID":5501,"AchievementsRemaining":1}"#
        } else {
            r#"{"Success":true}"#
        };
        HttpResponse {
            status: 200,
            body: body.as_bytes().to_vec(),
        }
    }
}

impl FakeServer {
    fn forms(&self) -> Vec<String> {
        self.seen
            .lock()
            .map(|seen| {
                seen.iter()
                    .map(|request| request.post_data.clone().unwrap_or_default())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// The 2 KiB of a NES's RAM, at $0000.
struct Ram(RefCell<[u8; 0x800]>);

impl Memory for Ram {
    fn read(&self, address: u32) -> Option<u8> {
        self.0.borrow().get(address as usize).copied()
    }
}

impl Ram {
    fn new() -> Self {
        Self(RefCell::new([0; 0x800]))
    }

    fn set(&self, address: usize, value: u8) {
        self.0.borrow_mut()[address] = value;
    }
}

/// Pumps until every request has its answer.
fn settle(client: &mut Client, memory: &Ram) {
    let started = Instant::now();
    while client.has_pending_requests() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the fake server never answered"
        );
        std::thread::sleep(Duration::from_millis(2));
        client.pump(memory);
    }
}

/// An iNES image: the header and 16 KiB of PRG.
fn image() -> Vec<u8> {
    let mut image = vec![0x4E, 0x45, 0x53, 0x1A, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    image.extend(std::iter::repeat_n(0xEA, 0x4000));
    image
}

fn signed_in_with_game(server: &Arc<FakeServer>, memory: &Ram) -> Client {
    let transport: Arc<dyn Transport> = server.clone();
    let Some(mut client) = Client::new(transport, "Zaxnes/test") else {
        panic!("rcheevos could not allocate the client");
    };
    client.login_with_password("Player", "secret");
    settle(&mut client, memory);
    client.load_game(memory, &image());
    settle(&mut client, memory);
    client
}

#[test]
fn a_sign_in_keeps_the_token_and_not_the_password() {
    let server = Arc::new(FakeServer::default());
    let memory = Ram::new();
    let transport: Arc<dyn Transport> = server.clone();
    let Some(mut client) = Client::new(transport, "Zaxnes/test") else {
        panic!("rcheevos could not allocate the client");
    };
    client.login_with_password("Player", "secret");
    settle(&mut client, &memory);

    assert_eq!(client.take_events(), vec![Event::Login(Ok(()))]);
    let user = client.user().unwrap_or_else(|| panic!("nobody signed in"));
    assert_eq!(user.username, "Player");
    assert_eq!(user.token, "ApiToken");
    assert_eq!(user.score_softcore, 123);
    let seen = server
        .seen
        .lock()
        .map(|seen| seen.clone())
        .unwrap_or_default();
    assert!(seen[0].user_agent.starts_with("Zaxnes/test rcheevos/"));
}

#[test]
fn the_game_loads_and_an_achievement_unlocks_in_softcore() {
    let server = Arc::new(FakeServer::default());
    let memory = Ram::new();
    let mut client = signed_in_with_game(&server, &memory);
    let events = client.take_events();
    assert!(events.contains(&Event::GameLoaded(Ok(()))), "{events:?}");
    let game = client.game().unwrap_or_else(|| panic!("no game"));
    assert_eq!(game.id, 1234);
    assert_eq!(game.title, "Sample Game");
    assert_eq!(client.summary().core, 2);
    // The hash of an iNES file leaves the header out.
    assert!(server
        .forms()
        .iter()
        .any(|form| form.starts_with("r=achievementsets") && form.contains("m=")));

    // An achievement already true when it starts waits for a false frame first.
    client.do_frame(&memory);
    memory.set(1, 3);
    memory.set(2, 7);
    memory.set(3, 4);
    client.do_frame(&memory);
    settle(&mut client, &memory);

    let unlocked: Vec<_> = client
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            Event::AchievementUnlocked(item) => Some(item),
            _ => None,
        })
        .collect();
    assert_eq!(unlocked.len(), 1);
    assert_eq!(unlocked[0].id, 5501);
    assert_eq!(unlocked[0].title, "Ach1");
    assert_eq!(unlocked[0].number, 5);
    // Softcore: the unlock is sent with h=0.
    assert!(server
        .forms()
        .iter()
        .any(|form| form.starts_with("r=awardachievement") && form.contains("&h=0&")));
    assert_eq!(client.summary().unlocked, 1);
    assert_eq!(client.rich_presence(), "Level 4");
    let list = client.achievements();
    assert!(list.iter().any(|entry| entry.id == 5501
        && entry.unlocked
        && entry.bucket == Bucket::RecentlyUnlocked));
    assert!(list.iter().any(|entry| entry.id == 5502 && !entry.unlocked));
}

#[test]
fn a_save_state_brings_the_hit_counts_back() {
    let server = Arc::new(FakeServer::default());
    let memory = Ram::new();
    let mut client = signed_in_with_game(&server, &memory);
    client.take_events();
    // Ach2 needs ten frames with $0004 = 1.
    client.do_frame(&memory);
    memory.set(4, 1);
    for _ in 0..5 {
        client.do_frame(&memory);
    }
    let Some(saved) = client.progress() else {
        panic!("no progress with a game loaded");
    };
    for _ in 0..3 {
        client.do_frame(&memory);
    }
    assert!(client.restore_progress(&memory, Some(&saved)));
    for _ in 0..4 {
        client.do_frame(&memory);
    }
    assert!(!client
        .take_events()
        .iter()
        .any(|event| matches!(event, Event::AchievementUnlocked(_))));
    client.do_frame(&memory);
    assert!(client
        .take_events()
        .iter()
        .any(|event| matches!(event, Event::AchievementUnlocked(item) if item.id == 5502)));
}

/// Reaches the real server, without an account (`cargo test -- --ignored`): the TLS stack and
/// the certificates built in.
#[test]
#[ignore = "needs the network"]
fn the_https_transport_reaches_the_server() {
    use zax_achievements::UreqTransport;
    let answer = UreqTransport::new().send(&HttpRequest {
        url: "https://retroachievements.org/dorequest.php".to_owned(),
        post_data: Some("r=gameid&m=00000000000000000000000000000000".to_owned()),
        content_type: Some("application/x-www-form-urlencoded".to_owned()),
        user_agent: "Zaxnes/test".to_owned(),
    });
    assert!(answer.status > 0, "no answer: {answer:?}");
    assert!(
        answer.body.starts_with(b"{"),
        "{}",
        String::from_utf8_lossy(&answer.body)
    );
}
