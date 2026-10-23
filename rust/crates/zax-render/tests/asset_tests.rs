//! The readers the extracted assets of SPEC-06 go through: the PNG decoder used by the skybox
//! cubemaps (REQ-SCN-050, REQ-SCN-051) and the OBJ reader of the mesh folder (port of
//! `AssetTests.cs`).

// A malformed test file fails the test loudly.
#![allow(clippy::expect_used)]

use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use zax_engine3d::vectors::{Vec2, Vec3};
use zax_render::assets::obj;
use zax_render::assets::png_image::{self, PngError};

#[test]
fn req_scn_050_the_png_decoder_reads_an_rgba_image_whatever_the_row_filter() {
    // One 4 x 5 image, a different filter on every row, so that all five of PNG section 9.2
    // are exercised at once.
    let mut pixels = vec![0_u8; 4 * 5 * 4];
    for i in 0..4 * 5 {
        pixels[i * 4] = u8::try_from(i * 11).expect("small value");
        pixels[i * 4 + 1] = u8::try_from(255 - i * 7).expect("small value");
        pixels[i * 4 + 2] = u8::try_from(i * 3).expect("small value");
        pixels[i * 4 + 3] = if i % 2 == 0 { 255 } else { 128 };
    }

    let decoded = png_image::decode(&png::rgba(4, 5, &pixels)).expect("decode");

    assert_eq!(4, decoded.width);
    assert_eq!(5, decoded.height);
    assert_eq!(pixels, decoded.pixels);
}

#[test]
fn req_scn_050_a_grey_png_becomes_an_opaque_grey_rgba_image() {
    let grey = [0, 64, 128, 255, 10, 20, 30, 40];
    let decoded = png_image::decode(&png::grey(4, 2, &grey)).expect("decode");

    assert_eq!((0, 0, 0, 255), decoded.pixel(0, 0));
    assert_eq!((128, 128, 128, 255), decoded.pixel(2, 0));
    assert_eq!((40, 40, 40, 255), decoded.pixel(3, 1));
}

#[test]
fn the_png_decoder_refuses_a_file_that_is_not_a_png() {
    assert!(matches!(
        png_image::decode(b"not a png"),
        Err(PngError::Decode(_))
    ));
}

#[test]
fn req_scn_054_the_obj_reader_builds_triangles_with_their_normals_and_uvs() {
    let mesh = obj::parse(
        "g cabinet
        v 0 0 0
        v 1 0 0
        v 1 1 0
        v 0 1 0
        vt 0 0
        vt 1 0
        vt 1 1
        vt 0 1
        vn 0 0 -1
        f 1/1/1 2/2/1 3/3/1 4/4/1",
    )
    .expect("parse");

    // The quad is triangulated as a fan: two triangles, four shared vertices.
    assert_eq!(4, mesh.vertex_count());
    assert_eq!(2, mesh.triangle_count());
    assert_eq!(Vec3::new(0.0, 0.0, -1.0), mesh.vertices()[0].normal);
    assert_eq!(Vec2::new(1.0, 1.0), mesh.vertices()[2].uv);
    assert_eq!(Vec3::new(1.0, 1.0, 0.0), mesh.bounds_max());
}

#[test]
fn q_023_only_the_first_group_is_kept_because_one_material_is_listed() {
    // Q-023: an exported model may keep a second submesh in its own group; one material per
    // model means one group, so the reader keeps the first and drops the rest.
    let source = "g Object004
        v 0 0 0
        v 1 0 0
        v 1 1 0
        v 0 0 1
        vn 0 0 -1
        g Object004_0
        f 1//1 2//1 3//1
        g Object004_1
        f 1//1 2//1 4//1";

    let groups = obj::parse_groups(source).expect("parse");
    assert_eq!(2, groups.len());
    assert_eq!("Object004_0", groups[0].0);
    assert_eq!("Object004_1", groups[1].0);
    assert_eq!(1, obj::parse(source).expect("parse").triangle_count());
}

#[test]
fn a_negative_obj_index_counts_back_from_the_end() {
    let mesh = obj::parse(
        "g g
        v 0 0 0
        v 1 0 0
        v 1 1 0
        f -3 -2 -1",
    )
    .expect("parse");

    assert_eq!(1, mesh.triangle_count());
    assert_eq!(3, mesh.vertex_count());
}

/// Builds the smallest valid PNG files the decoder has to read.
mod png {
    use super::{Compression, Write, ZlibEncoder};

    pub fn rgba(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
        build(
            width,
            height,
            6,
            &filtered(width as usize, height as usize, 4, pixels),
        )
    }

    pub fn grey(width: u32, height: u32, samples: &[u8]) -> Vec<u8> {
        build(
            width,
            height,
            0,
            &filtered(width as usize, height as usize, 1, samples),
        )
    }

    /// Applies filter `row % 5` to each row, filter byte included.
    fn filtered(width: usize, height: usize, bytes_per_pixel: usize, pixels: &[u8]) -> Vec<u8> {
        let stride = width * bytes_per_pixel;
        let mut raw = vec![0_u8; (stride + 1) * height];
        for y in 0..height {
            let filter = y % 5;
            raw[y * (stride + 1)] = u8::try_from(filter).expect("filter 0 to 4");
            for x in 0..stride {
                let value = i32::from(pixels[y * stride + x]);
                let a = if x >= bytes_per_pixel {
                    i32::from(pixels[y * stride + x - bytes_per_pixel])
                } else {
                    0
                };
                let b = if y > 0 {
                    i32::from(pixels[(y - 1) * stride + x])
                } else {
                    0
                };
                let c = if y > 0 && x >= bytes_per_pixel {
                    i32::from(pixels[(y - 1) * stride + x - bytes_per_pixel])
                } else {
                    0
                };

                let encoded = match filter {
                    0 => value,
                    1 => value - a,
                    2 => value - b,
                    3 => value - ((a + b) >> 1),
                    _ => value - paeth(a, b, c),
                };
                // The filters work modulo 256 (PNG §9.2).
                raw[y * (stride + 1) + 1 + x] =
                    u8::try_from(encoded.rem_euclid(256)).expect("a byte");
            }
        }

        raw
    }

    fn paeth(a: i32, b: i32, c: i32) -> i32 {
        let p = a + b - c;
        let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
        if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        }
    }

    fn build(width: u32, height: u32, color_type: u8, raw: &[u8]) -> Vec<u8> {
        let mut file = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

        let mut header = Vec::with_capacity(13);
        header.extend_from_slice(&width.to_be_bytes());
        header.extend_from_slice(&height.to_be_bytes());
        header.extend_from_slice(&[8, color_type, 0, 0, 0]);
        chunk(&mut file, *b"IHDR", &header);

        let mut deflate = ZlibEncoder::new(Vec::new(), Compression::fast());
        deflate.write_all(raw).expect("deflate");
        chunk(&mut file, *b"IDAT", &deflate.finish().expect("deflate"));
        chunk(&mut file, *b"IEND", &[]);
        file
    }

    fn chunk(file: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
        file.extend_from_slice(
            &u32::try_from(data.len())
                .expect("small chunk")
                .to_be_bytes(),
        );
        file.extend_from_slice(&kind);
        file.extend_from_slice(data);

        // The decoder does not check the CRC, so four zero bytes stand in for it.
        file.extend_from_slice(&[0, 0, 0, 0]);
    }
}
