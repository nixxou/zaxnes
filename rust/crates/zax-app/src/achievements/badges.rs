//! The icons of the achievements and of the game: PNG files of the RetroAchievements media
//! server, fetched when first drawn, kept in memory and handed to the interface renderer as
//! pictures.

use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use zax_achievements::{HttpRequest, Transport};
use zax_platform::ui_image::{PictureUpdate, UiPicture};

/// The first texture id of the icons, far above the interface's own pictures.
pub const FIRST_BADGE_ID: usize = 4096;
/// How many icons are fetched at once.
const PARALLEL_DOWNLOADS: usize = 4;
/// The largest icon accepted (the site's are 64 x 64).
const MAX_SIDE: u32 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Badge {
    Waiting,
    Loading,
    Ready(usize),
    Failed,
}

/// The icons.
pub struct BadgeCache {
    transport: Arc<dyn Transport>,
    user_agent: String,
    badges: HashMap<String, Badge>,
    waiting: VecDeque<String>,
    loading: usize,
    next_id: usize,
    inbox: Receiver<(String, Option<UiPicture>)>,
    outbox: Sender<(String, Option<UiPicture>)>,
    updates: Vec<PictureUpdate>,
}

impl BadgeCache {
    /// An empty cache that fetches through `transport`.
    #[must_use]
    pub fn new(transport: Arc<dyn Transport>, user_agent: &str) -> Self {
        let (outbox, inbox) = channel();
        Self {
            transport,
            user_agent: user_agent.to_owned(),
            badges: HashMap::new(),
            waiting: VecDeque::new(),
            loading: 0,
            next_id: FIRST_BADGE_ID,
            inbox,
            outbox,
            updates: Vec::new(),
        }
    }

    /// The texture id of the icon at `url`, once it is there; the first call asks for it.
    pub fn texture(&mut self, url: &str) -> Option<usize> {
        if url.is_empty() {
            return None;
        }
        match self.badges.get(url) {
            Some(Badge::Ready(id)) => Some(*id),
            Some(_) => None,
            None => {
                self.badges.insert(url.to_owned(), Badge::Waiting);
                self.waiting.push_back(url.to_owned());
                self.start_downloads();
                None
            }
        }
    }

    /// Takes the icons that arrived and starts the next downloads.
    pub fn poll(&mut self) {
        while let Ok((url, picture)) = self.inbox.try_recv() {
            self.loading = self.loading.saturating_sub(1);
            let Some(badge) = self.badges.get_mut(&url) else {
                continue;
            };
            match picture {
                Some(picture) => {
                    let id = self.next_id;
                    self.next_id += 1;
                    *badge = Badge::Ready(id);
                    self.updates.push(PictureUpdate {
                        id,
                        picture: Some(picture),
                    });
                }
                None => *badge = Badge::Failed,
            }
        }
        self.start_downloads();
    }

    /// Forgets every icon (a new game): the renderer drops their textures.
    pub fn clear(&mut self) {
        for badge in self.badges.values() {
            if let Badge::Ready(id) = badge {
                self.updates.push(PictureUpdate {
                    id: *id,
                    picture: None,
                });
            }
        }
        self.badges.clear();
        self.waiting.clear();
    }

    /// The changes of the textures since the last call.
    pub fn take_updates(&mut self) -> Vec<PictureUpdate> {
        std::mem::take(&mut self.updates)
    }

    fn start_downloads(&mut self) {
        while self.loading < PARALLEL_DOWNLOADS {
            let Some(url) = self.waiting.pop_front() else {
                return;
            };
            if self.badges.get(&url) != Some(&Badge::Waiting) {
                continue;
            }
            self.badges.insert(url.clone(), Badge::Loading);
            self.loading += 1;
            let transport = Arc::clone(&self.transport);
            let outbox = self.outbox.clone();
            let request = HttpRequest {
                url: url.clone(),
                post_data: None,
                content_type: None,
                user_agent: self.user_agent.clone(),
            };
            let spawned = std::thread::Builder::new()
                .name("zaxnes-badge".to_owned())
                .spawn(move || {
                    let answer = transport.send(&request);
                    let picture = (answer.status == 200)
                        .then(|| decode_png(&answer.body))
                        .flatten();
                    let _ = outbox.send((request.url, picture));
                });
            if spawned.is_err() {
                let _ = self.outbox.send((url, None));
            }
        }
    }
}

impl std::fmt::Debug for BadgeCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BadgeCache")
            .field("badges", &self.badges.len())
            .finish_non_exhaustive()
    }
}

/// A PNG file as RGBA pixels; `None` for anything unreadable or too big.
#[must_use]
pub fn decode_png(file: &[u8]) -> Option<UiPicture> {
    let mut decoder = png::Decoder::new(file);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let (width, height) = (reader.info().width, reader.info().height);
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return None;
    }
    let mut buffer = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buffer).ok()?;
    let pixels = &buffer[..frame.buffer_size()];
    let rgba: Vec<u8> = match frame.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels
            .chunks_exact(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .chunks_exact(2)
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        png::ColorType::Grayscale => pixels
            .iter()
            .flat_map(|&grey| [grey, grey, grey, 255])
            .collect(),
        png::ColorType::Indexed => return None,
    };
    let picture = UiPicture {
        width,
        height,
        rgba,
    };
    picture.is_whole().then_some(picture)
}
