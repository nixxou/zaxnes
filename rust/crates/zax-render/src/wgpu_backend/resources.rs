//! The meshes and the textures the wgpu backend keeps on the GPU, the twin of the GL mesh and
//! texture caches: keyed by the address of the shared mesh or texture, kept while used, released
//! after [`RETENTION_FRAMES`] rendered frames without use.

use std::sync::Arc;

use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_mesh::TileMesh;
use zax_engine3d::hashing::FastMap;
use zax_engine3d::palette::nes_color;

/// Rendered frames an unused mesh or texture is kept for, as in the GL caches.
pub const RETENTION_FRAMES: u64 = 600;

/// Bytes per interleaved vertex: position, normal, uv.
const VERTEX_BYTES: usize = 32;
/// The same, as a buffer address.
pub const VERTEX_STRIDE: u64 = VERTEX_BYTES as u64;

/// A resident mesh: its slot in the store and its index count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle {
    pub slot: u32,
    pub index_count: u32,
}

/// A resident texture: its slot in the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u32);

#[derive(Debug)]
struct MeshEntry {
    /// The mesh, kept to re-upload it when the atlas is rebuilt (and so that the keyed address
    /// stays unique while the entry lives).
    mesh: Arc<TileMesh>,
    /// Where its vertices and its indices start in the atlas.
    base_vertex: u32,
    first_index: u32,
    last_used: u64,
}

/// The resident meshes, all in one vertex buffer and one index buffer, so that a pass binds them
/// once and each draw call only names its range. Meshes are appended; a released mesh leaves a
/// hole until the atlas runs out of room, when it is rebuilt with the live meshes only.
#[derive(Debug)]
pub struct MeshStore {
    slots: Vec<Option<MeshEntry>>,
    free: Vec<u32>,
    by_address: FastMap<*const TileMesh, MeshHandle>,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    /// Capacities and filled lengths, in vertices and in indices.
    vertex_capacity: u32,
    index_capacity: u32,
    vertex_top: u32,
    index_top: u32,
    frame: u64,
    uploads: u32,
}

impl MeshStore {
    /// An empty atlas on `device`.
    pub fn new(device: &wgpu::Device) -> Self {
        let (vertex_capacity, index_capacity) = (1 << 16, 1 << 17);
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            by_address: FastMap::default(),
            vertices: atlas_buffer(device, vertex_capacity, VERTEX_STRIDE, true),
            indices: atlas_buffer(device, index_capacity, 4, false),
            vertex_capacity,
            index_capacity,
            vertex_top: 0,
            index_top: 0,
            frame: 0,
            uploads: 0,
        }
    }

    /// Starts a rendered frame.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// Number of resident meshes.
    pub fn count(&self) -> usize {
        self.by_address.len()
    }

    /// Meshes uploaded since the store was created, as the GL cache counts them.
    pub const fn uploads(&self) -> u32 {
        self.uploads
    }

    /// The vertex and the index buffer of the atlas, for the whole of a pass.
    pub const fn buffers(&self) -> (&wgpu::Buffer, &wgpu::Buffer) {
        (&self.vertices, &self.indices)
    }

    /// Where a resident mesh starts: its first index and its base vertex.
    pub fn range(&self, handle: MeshHandle) -> (u32, i32) {
        let entry = self.slots[handle.slot as usize]
            .as_ref()
            .unwrap_or_else(|| unreachable!("a planned mesh is resident for the whole frame"));
        (
            entry.first_index,
            i32::try_from(entry.base_vertex).unwrap_or(i32::MAX),
        )
    }

    /// The resident copy of `mesh`, uploading it on first use; `None` for an empty mesh.
    pub fn get(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mesh: &Arc<TileMesh>,
    ) -> Option<MeshHandle> {
        if mesh.is_empty() {
            return None;
        }

        let key = Arc::as_ptr(mesh);
        if let Some(handle) = self.by_address.get(&key).copied() {
            if let Some(Some(entry)) = self.slots.get_mut(handle.slot as usize) {
                entry.last_used = self.frame;
            }
            return Some(handle);
        }

        let (vertex_count, index_count) = counts(mesh);
        if self.vertex_top + vertex_count > self.vertex_capacity
            || self.index_top + index_count > self.index_capacity
        {
            self.rebuild(device, queue, vertex_count, index_count);
        }
        let (base_vertex, first_index) = (self.vertex_top, self.index_top);
        self.write(queue, mesh, base_vertex, first_index);
        self.vertex_top += vertex_count;
        self.index_top += index_count;

        let entry = MeshEntry {
            mesh: Arc::clone(mesh),
            base_vertex,
            first_index,
            last_used: self.frame,
        };
        let slot = place(&mut self.slots, &mut self.free, entry);
        let handle = MeshHandle { slot, index_count };
        self.by_address.insert(key, handle);
        self.uploads += 1;
        Some(handle)
    }

    /// Makes room for `vertices` and `indices` more: the live meshes are laid out again from
    /// the start of new buffers, at least twice as large as what they and the newcomer need.
    fn rebuild(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: u32, indices: u32) {
        let (mut live_vertices, mut live_indices) = (vertices, indices);
        for entry in self.slots.iter().flatten() {
            let (v, i) = counts(&entry.mesh);
            live_vertices += v;
            live_indices += i;
        }
        self.vertex_capacity = self
            .vertex_capacity
            .max((live_vertices * 2).next_power_of_two());
        self.index_capacity = self
            .index_capacity
            .max((live_indices * 2).next_power_of_two());
        self.vertices = atlas_buffer(device, self.vertex_capacity, VERTEX_STRIDE, true);
        self.indices = atlas_buffer(device, self.index_capacity, 4, false);
        self.vertex_top = 0;
        self.index_top = 0;

        let mut slots = std::mem::take(&mut self.slots);
        for entry in slots.iter_mut().flatten() {
            let (v, i) = counts(&entry.mesh);
            entry.base_vertex = self.vertex_top;
            entry.first_index = self.index_top;
            self.write(queue, &entry.mesh, entry.base_vertex, entry.first_index);
            self.vertex_top += v;
            self.index_top += i;
        }
        self.slots = slots;
    }

    /// Uploads `mesh` at `base_vertex` and `first_index` of the atlas.
    fn write(&self, queue: &wgpu::Queue, mesh: &TileMesh, base_vertex: u32, first_index: u32) {
        let mut vertices = Vec::with_capacity(mesh.vertex_count() * VERTEX_BYTES);
        for vertex in mesh.vertices() {
            for value in [
                vertex.position.x,
                vertex.position.y,
                vertex.position.z,
                vertex.normal.x,
                vertex.normal.y,
                vertex.normal.z,
                vertex.uv.x,
                vertex.uv.y,
            ] {
                vertices.extend_from_slice(&value.to_ne_bytes());
            }
        }
        let mut indices: Vec<u8> = mesh
            .triangles()
            .iter()
            .flat_map(|index| index.to_ne_bytes())
            .collect();
        // Buffer writes go by multiples of 4 bytes: the index data always is, the vertex data
        // too (32 bytes a vertex).
        indices.resize(indices.len().next_multiple_of(4), 0);
        queue.write_buffer(
            &self.vertices,
            u64::from(base_vertex) * VERTEX_STRIDE,
            &vertices,
        );
        queue.write_buffer(&self.indices, u64::from(first_index) * 4, &indices);
    }

    /// Releases every mesh whose last use is older than the retention window (its room is
    /// reclaimed at the next rebuild).
    pub fn collect(&mut self) {
        let frame = self.frame;
        let (slots, free) = (&mut self.slots, &mut self.free);
        self.by_address.retain(|_, handle| {
            let slot = &mut slots[handle.slot as usize];
            let keep = slot
                .as_ref()
                .is_some_and(|entry| frame - entry.last_used <= RETENTION_FRAMES);
            if !keep {
                *slot = None;
                free.push(handle.slot);
            }
            keep
        });
    }

    /// Releases every mesh.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.free.clear();
        self.by_address.clear();
        self.vertex_top = 0;
        self.index_top = 0;
    }
}

/// The vertex and the index count of `mesh`.
#[allow(clippy::cast_possible_truncation)] // A tile mesh holds far fewer than 2^32 of either.
fn counts(mesh: &TileMesh) -> (u32, u32) {
    (mesh.vertex_count() as u32, mesh.triangles().len() as u32)
}

fn atlas_buffer(device: &wgpu::Device, count: u32, stride: u64, vertex: bool) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(if vertex {
            "zax mesh vertices"
        } else {
            "zax mesh indices"
        }),
        size: u64::from(count) * stride,
        usage: if vertex {
            wgpu::BufferUsages::VERTEX
        } else {
            wgpu::BufferUsages::INDEX
        } | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// One resident texture: an indexed (`R8Uint`) or a direct (`Rgba8Unorm`) one.
#[derive(Debug)]
struct TextureEntry {
    _texture: Option<Arc<ObjectTexture>>,
    view: wgpu::TextureView,
    last_used: u64,
}

/// The resident object textures and the NES picture of the 2D mode.
#[derive(Debug, Default)]
pub struct TextureStore {
    slots: Vec<Option<TextureEntry>>,
    free: Vec<u32>,
    by_address: FastMap<*const ObjectTexture, TextureHandle>,
    picture: Option<TextureHandle>,
    picture_pixels: Vec<u8>,
    frame: u64,
    /// Bumped whenever a slot is released or reused, so that the bind groups built on it are
    /// rebuilt.
    generation: u64,
    /// Bumped whenever a slot is filled or released, so that the texture arrays of the batched
    /// draws are rebuilt.
    revision: u64,
}

impl TextureStore {
    /// Starts a rendered frame.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// Number of resident object textures.
    pub fn count(&self) -> usize {
        self.by_address.len()
    }

    /// Changes whenever a slot is released: the bind groups of the textures are stale.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Changes whenever a slot is filled or released: the texture arrays are stale.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Number of slots, the highest resident slot plus one at most.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// The view in each slot, `None` for a free slot.
    pub fn slot_views(&self) -> impl Iterator<Item = Option<&wgpu::TextureView>> {
        self.slots
            .iter()
            .map(|slot| slot.as_ref().map(|entry| &entry.view))
    }

    /// The view of a resident texture.
    pub fn view(&self, handle: TextureHandle) -> &wgpu::TextureView {
        &self.slots[handle.0 as usize]
            .as_ref()
            .unwrap_or_else(|| unreachable!("a planned texture is resident for the whole frame"))
            .view
    }

    /// The resident copy of `texture`, uploading it on first use.
    #[allow(clippy::cast_sign_loss)] // Texture sizes are at least 1.
    pub fn get(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &Arc<ObjectTexture>,
    ) -> TextureHandle {
        let key = Arc::as_ptr(texture);
        if let Some(handle) = self.by_address.get(&key).copied() {
            if let Some(Some(entry)) = self.slots.get_mut(handle.0 as usize) {
                entry.last_used = self.frame;
            }
            return handle;
        }

        let (width, height) = (texture.width(), texture.height());
        let texels = (0..height).flat_map(|y| (0..width).map(move |x| (x, y)));
        let view = if texture.is_direct() {
            let pixels: Vec<u8> = texels
                .flat_map(|(x, y)| {
                    let color = texture.color(x, y);
                    [color.r, color.g, color.b, 255]
                })
                .collect();
            upload(
                device,
                queue,
                wgpu::TextureFormat::Rgba8Unorm,
                (width as u32, height as u32),
                &pixels,
            )
        } else {
            #[allow(clippy::cast_possible_truncation)] // The low byte, as `& 0xFF` in C#.
            let pixels: Vec<u8> = texels
                .map(|(x, y)| (texture.texel(x, y) & 0xFF) as u8)
                .collect();
            upload(
                device,
                queue,
                wgpu::TextureFormat::R8Uint,
                (width as u32, height as u32),
                &pixels,
            )
        };
        let entry = TextureEntry {
            _texture: Some(Arc::clone(texture)),
            view,
            last_used: self.frame,
        };
        let handle = TextureHandle(place(&mut self.slots, &mut self.free, entry));
        self.by_address.insert(key, handle);
        self.revision += 1;
        handle
    }

    /// Uploads the NES picture of the 2D mode (colour indices, row 0 first, 240 rows of 256 or of
    /// a widescreen width), or `None` when it has no such size.
    pub fn picture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        indices: &[u8],
    ) -> Option<TextureHandle> {
        const HEIGHT: usize = 240;
        let width = crate::interop::picture_width(indices.len())?;
        self.picture_pixels.resize(width * HEIGHT * 4, 0);
        for (pixel, &index) in self.picture_pixels.chunks_exact_mut(4).zip(indices) {
            let color = nes_color(usize::from(index));
            pixel.copy_from_slice(&[color.r, color.g, color.b, 255]);
        }

        #[allow(clippy::cast_possible_truncation)] // At most 512 and 240.
        let size = (width as u32, HEIGHT as u32);
        if let Some(handle) = self.picture {
            let entry = self.slots[handle.0 as usize]
                .as_ref()
                .unwrap_or_else(|| unreachable!("the picture slot is never released"));
            let texture = entry.view.texture();
            if (texture.width(), texture.height()) == size {
                write(queue, texture, size, 4, &self.picture_pixels);
                return Some(handle);
            }
            // Another width: the slot gets a new texture of the right size.
            let view = upload(
                device,
                queue,
                wgpu::TextureFormat::Rgba8Unorm,
                size,
                &self.picture_pixels,
            );
            self.slots[handle.0 as usize] = Some(TextureEntry {
                _texture: None,
                view,
                last_used: u64::MAX,
            });
            self.revision += 1;
            return Some(handle);
        }

        let view = upload(
            device,
            queue,
            wgpu::TextureFormat::Rgba8Unorm,
            size,
            &self.picture_pixels,
        );
        let entry = TextureEntry {
            _texture: None,
            view,
            last_used: u64::MAX,
        };
        let handle = TextureHandle(place(&mut self.slots, &mut self.free, entry));
        self.picture = Some(handle);
        self.revision += 1;
        Some(handle)
    }

    /// Releases every object texture whose last use is older than the retention window.
    pub fn collect(&mut self) {
        let frame = self.frame;
        let (slots, free) = (&mut self.slots, &mut self.free);
        let mut released = false;
        self.by_address.retain(|_, handle| {
            let slot = &mut slots[handle.0 as usize];
            let keep = slot
                .as_ref()
                .is_some_and(|entry| frame - entry.last_used <= RETENTION_FRAMES);
            if !keep {
                *slot = None;
                free.push(handle.0);
                released = true;
            }
            keep
        });
        if released {
            self.generation += 1;
            self.revision += 1;
        }
    }

    /// Releases every object texture (the picture of the 2D mode stays).
    pub fn clear(&mut self) {
        for handle in self.by_address.values() {
            self.slots[handle.0 as usize] = None;
            self.free.push(handle.0);
        }
        self.by_address.clear();
        self.generation += 1;
        self.revision += 1;
    }
}

/// Puts `entry` in a free slot, or a new one, and returns the slot.
fn place<T>(slots: &mut Vec<Option<T>>, free: &mut Vec<u32>, entry: T) -> u32 {
    if let Some(slot) = free.pop() {
        slots[slot as usize] = Some(entry);
        return slot;
    }
    slots.push(Some(entry));
    u32::try_from(slots.len() - 1).unwrap_or(u32::MAX)
}

/// A new 2D texture of `format` holding `pixels`, nearest-sampled by `textureLoad`.
pub fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    pixels: &[u8],
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("zax texture"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let bytes_per_texel = format.block_copy_size(None).unwrap_or(4);
    write(queue, &texture, size, bytes_per_texel, pixels);
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Writes the whole of `texture`.
pub fn write(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: (u32, u32),
    bytes_per_texel: u32,
    pixels: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.0 * bytes_per_texel),
            rows_per_image: Some(size.1),
        },
        wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
    );
}
