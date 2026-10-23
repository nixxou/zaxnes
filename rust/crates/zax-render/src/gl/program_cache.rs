//! The on-disk cache of linked shader programs: a driver that can hand out the binary of a
//! program (`GL_ARB_get_program_binary`, core in OpenGL ES 3.0 and OpenGL 4.1) saves it once, and
//! the next start loads it instead of compiling and linking the sources again, which on a
//! handheld GPU takes seconds.
//!
//! This file holds the part that needs no GL: the key a binary is filed under, the file format
//! and the reading and writing. A binary is only valid for the driver that made it, so the key
//! covers the renderer and version strings as well as the sources; and a file that cannot be read
//! back, or that the driver then refuses, is never an error: the program is built from source as
//! if there were no cache (see `Gl::cached_program`).

use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

use glow::Version;

/// First bytes of a cache file.
const MAGIC: &[u8; 6] = b"ZXPB1\n";

/// Bytes of the header: the magic, the binary format and the length of the binary.
const HEADER: usize = MAGIC.len() + 4 + 4;

/// Bytes of the checksum that ends a file.
const CHECKSUM: usize = 8;

/// Whether a context can give the binary of a linked program back: OpenGL ES 3.0 and OpenGL 4.1
/// have the calls in the core, older desktop contexts need the extension.
#[must_use]
pub fn supports_program_binary(version: &Version, extensions: &HashSet<String>) -> bool {
    let core = if version.is_embedded {
        (version.major, version.minor) >= (3, 0)
    } else {
        (version.major, version.minor) >= (4, 1)
    };
    core || extensions.contains("GL_ARB_get_program_binary")
}

/// The folder of cached binaries, for one driver.
#[derive(Debug, Clone)]
pub struct ProgramCache {
    directory: PathBuf,
    /// What tells one driver from another: the renderer and version strings of the context.
    driver: String,
}

impl ProgramCache {
    /// A cache in `directory` for the driver named by `renderer` and `version`.
    #[must_use]
    pub fn new(directory: PathBuf, renderer: &str, version: &str) -> Self {
        Self {
            directory,
            driver: format!("{renderer}\n{version}"),
        }
    }

    /// The name a program is filed under: a hash of the driver, the program's name and both
    /// sources, so that any change to one of them is another file.
    #[must_use]
    pub fn key(&self, name: &str, vertex: &str, fragment: &str) -> String {
        let mut hash = Fnv::new();
        for part in [self.driver.as_str(), name, vertex, fragment] {
            // The length first: "ab" + "c" and "a" + "bc" must not hash alike.
            hash.write(&part.len().to_le_bytes());
            hash.write(part.as_bytes());
        }
        format!("{:016x}", hash.finish())
    }

    fn path(&self, key: &str) -> PathBuf {
        self.directory.join(format!("{key}.bin"))
    }

    /// The binary filed under `key` and its format, or `None` when there is none or the file is
    /// not one this program wrote whole.
    #[must_use]
    pub fn load(&self, key: &str) -> Option<(u32, Vec<u8>)> {
        let bytes = std::fs::read(self.path(key)).ok()?;
        let (format, binary) = decode(&bytes)?;
        Some((format, binary.to_vec()))
    }

    /// Files `binary` under `key`. The file is written aside and renamed, so a reader never sees
    /// half of one.
    ///
    /// # Errors
    ///
    /// The folder or the file could not be written.
    pub fn store(&self, key: &str, format: u32, binary: &[u8]) -> io::Result<()> {
        std::fs::create_dir_all(&self.directory)?;
        let target = self.path(key);
        let partial = target.with_extension("part");
        std::fs::write(&partial, encode(format, binary))?;
        std::fs::rename(partial, target)
    }

    /// Removes the file of `key`, which the driver refused.
    pub fn discard(&self, key: &str) {
        // A file that is already gone is what was asked for.
        let _ = std::fs::remove_file(self.path(key));
    }
}

/// The bytes of a cache file: the header, the binary and a checksum of both.
fn encode(format: u32, binary: &[u8]) -> Vec<u8> {
    let length = u32::try_from(binary.len()).unwrap_or(u32::MAX);
    let mut bytes = Vec::with_capacity(HEADER + binary.len() + CHECKSUM);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&format.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(binary);
    let mut hash = Fnv::new();
    hash.write(&bytes);
    bytes.extend_from_slice(&hash.finish().to_le_bytes());
    bytes
}

/// The format and the binary of a cache file, when it is whole.
fn decode(bytes: &[u8]) -> Option<(u32, &[u8])> {
    let (body, checksum) = bytes.split_at(bytes.len().checked_sub(CHECKSUM)?);
    let mut hash = Fnv::new();
    hash.write(body);
    if hash.finish().to_le_bytes() != checksum {
        return None;
    }
    let rest = body.strip_prefix(MAGIC)?;
    let (format, rest) = rest.split_first_chunk::<4>()?;
    let (length, binary) = rest.split_first_chunk::<4>()?;
    let length = usize::try_from(u32::from_le_bytes(*length)).ok()?;
    (binary.len() == length && length > 0).then(|| (u32::from_le_bytes(*format), binary))
}

/// FNV-1a over 64 bits: a hash that is the same on every machine and every build, which the
/// standard library's does not promise. It names cache files and detects damaged ones; nobody
/// forges them.
struct Fnv(u64);

impl Fnv {
    const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }

    const fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache(renderer: &str, version: &str) -> ProgramCache {
        ProgramCache::new(PathBuf::from("/nowhere"), renderer, version)
    }

    fn folder(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("zax-program-cache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn the_key_follows_the_driver_the_name_and_both_sources() {
        let base = cache("Mali-G31", "OpenGL ES 3.2");
        let key = base.key("scene", "void main(){}", "void main(){}");
        assert_eq!(16, key.len());
        assert_eq!(key, base.key("scene", "void main(){}", "void main(){}"));
        for other in [
            cache("Mali-G52", "OpenGL ES 3.2").key("scene", "void main(){}", "void main(){}"),
            cache("Mali-G31", "OpenGL ES 3.1").key("scene", "void main(){}", "void main(){}"),
            base.key("shadow", "void main(){}", "void main(){}"),
            base.key("scene", "void main(){ }", "void main(){}"),
            base.key("scene", "void main(){}", "void main(){ }"),
        ] {
            assert_ne!(key, other);
        }
        // The boundary between two parts counts.
        assert_ne!(base.key("ab", "c", ""), base.key("a", "bc", ""));
    }

    #[test]
    fn a_stored_binary_comes_back_whole() {
        let cache = ProgramCache::new(folder("round-trip"), "r", "v");
        let key = cache.key("p", "v", "f");
        assert_eq!(None, cache.load(&key), "nothing stored yet");
        cache.store(&key, 0x8E21, &[1, 2, 3, 4, 5]).unwrap();
        assert_eq!(Some((0x8E21, vec![1, 2, 3, 4, 5])), cache.load(&key));
        cache.discard(&key);
        assert_eq!(None, cache.load(&key));
        let _ = std::fs::remove_dir_all(&cache.directory);
    }

    #[test]
    fn a_damaged_file_is_not_a_binary() {
        let good = encode(7, &[9, 8, 7, 6]);
        assert_eq!(Some((7, &[9u8, 8, 7, 6][..])), decode(&good));

        let mut flipped = good.clone();
        flipped[HEADER] ^= 1;
        assert_eq!(None, decode(&flipped), "a changed byte");
        assert_eq!(None, decode(&good[..good.len() - 1]), "a truncated file");
        assert_eq!(None, decode(&[]), "an empty file");
        assert_eq!(None, decode(&encode(7, &[])), "a file with no binary");

        let mut foreign = good;
        foreign[0] = b'X';
        // The checksum covers the magic too, so recompute it to test the magic alone.
        let body = foreign.len() - CHECKSUM;
        let mut hash = Fnv::new();
        hash.write(&foreign[..body]);
        foreign[body..].copy_from_slice(&hash.finish().to_le_bytes());
        assert_eq!(None, decode(&foreign), "another kind of file");
    }

    #[test]
    fn a_damaged_file_on_disk_loads_as_nothing() {
        let cache = ProgramCache::new(folder("damaged"), "r", "v");
        std::fs::create_dir_all(&cache.directory).unwrap();
        let key = cache.key("p", "v", "f");
        std::fs::write(cache.path(&key), b"not a binary").unwrap();
        assert_eq!(None, cache.load(&key));
        let _ = std::fs::remove_dir_all(&cache.directory);
    }

    #[test]
    fn gles_3_and_gl_4_1_have_the_calls_and_older_gl_needs_the_extension() {
        let none = HashSet::new();
        let extension = HashSet::from(["GL_ARB_get_program_binary".to_owned()]);
        let version = |is_embedded, major, minor| Version {
            major,
            minor,
            is_embedded,
            revision: None,
            vendor_info: String::new(),
        };
        assert!(supports_program_binary(&version(true, 3, 0), &none));
        assert!(supports_program_binary(&version(true, 3, 2), &none));
        assert!(!supports_program_binary(&version(true, 2, 0), &none));
        assert!(supports_program_binary(&version(false, 4, 5), &none));
        assert!(!supports_program_binary(&version(false, 3, 3), &none));
        assert!(supports_program_binary(&version(false, 3, 3), &extension));
    }

    /// With a driver: a binary the driver refuses is dropped and the sources are compiled.
    #[cfg(feature = "offscreen")]
    #[test]
    fn a_binary_the_driver_refuses_is_discarded_and_a_good_one_is_reused() {
        use crate::gl::api::Gl;
        use crate::offscreen::{ContextApi, OffscreenContext};

        const VERTEX: &str = "#version 330 core\nvoid main() { gl_Position = vec4(0.0); }\n";
        const FRAGMENT: &str =
            "#version 330 core\nout vec4 colour;\nvoid main() { colour = vec4(1.0); }\n";

        let context = OffscreenContext::new(16, 16, ContextApi::DesktopCore).unwrap();
        let directory = folder("driver");
        let gl = Gl::new(context.gl()).with_program_cache(directory.clone());
        assert!(gl.caches_programs(), "every desktop core driver can");
        assert!(
            gl.cached_program("p", VERTEX, FRAGMENT).is_none(),
            "nothing cached yet"
        );

        let vertex = gl.compile("p", glow::VERTEX_SHADER, VERTEX).unwrap();
        let fragment = gl.compile("p", glow::FRAGMENT_SHADER, FRAGMENT).unwrap();
        let program = gl.link("p", vertex, fragment).unwrap();
        gl.cache_program("p", VERTEX, FRAGMENT, program);
        let cache = ProgramCache::new(
            directory.clone(),
            &gl_string(&context, glow::RENDERER),
            &gl_string(&context, glow::VERSION),
        );
        let key = cache.key("p", VERTEX, FRAGMENT);
        let Some((format, binary)) = cache.load(&key) else {
            // A driver with no program binaries (the extension is advertised, the formats are
            // not) leaves the cache empty and the sources compiled: nothing more to check.
            let _ = std::fs::remove_dir_all(&directory);
            return;
        };
        assert!(
            gl.cached_program("p", VERTEX, FRAGMENT).is_some(),
            "the good binary loads"
        );

        cache
            .store(&key, format, &binary[..binary.len() / 2])
            .unwrap();
        assert!(
            gl.cached_program("p", VERTEX, FRAGMENT).is_none(),
            "half a binary is refused"
        );
        assert_eq!(None, cache.load(&key), "and its file is removed");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[cfg(feature = "offscreen")]
    fn gl_string(context: &crate::offscreen::OffscreenContext, parameter: u32) -> String {
        use glow::HasContext as _;
        let gl = context.gl();
        // SAFETY: the offscreen context is current on this thread; a string query.
        unsafe { gl.get_parameter_string(parameter) }
    }
}
