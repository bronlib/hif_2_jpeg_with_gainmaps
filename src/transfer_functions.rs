
use ultrahdr_core::{ColorTransfer, RawImage};
use zenjpeg::ultrahdr::{UhdrPixelFormat};

pub fn apply_transfer_vec<F>(input: &[f32], f: F) -> Vec<f32>
where
    F: Fn(f32) -> f32,
{
    input.iter().copied().map(f).collect()
}

pub fn apply_transfer_rgb<F>(r: &mut [f32], g: &mut [f32], b: &mut [f32], f: F)
where
    F: Fn(f32) -> f32 + Copy,
{
    for v in r {
        *v = f(*v);
    }
    for v in g {
        *v = f(*v);
    }
    for v in b {
        *v = f(*v);
    }
}

pub(crate) fn apply_transfer<F>(image: &RawImage, f: F) -> RawImage
where
    F: Fn(f32) -> f32,
{
    assert_eq!(image.format, UhdrPixelFormat::Rgba32F);
    //assert_eq!(image.transfer, ColorTransfer::Linear);

    let mut output = image.clone();

    for pixel in output.data.chunks_exact_mut(16) {
        let r = f32::from_ne_bytes(pixel[0..4].try_into().unwrap());
        let g = f32::from_ne_bytes(pixel[4..8].try_into().unwrap());
        let b = f32::from_ne_bytes(pixel[8..12].try_into().unwrap());

        let r709 = f(r);
        let g709 = f(g);
        let b709 = f(b);

        pixel[0..4].copy_from_slice(&r709.to_ne_bytes());
        pixel[4..8].copy_from_slice(&g709.to_ne_bytes());
        pixel[8..12].copy_from_slice(&b709.to_ne_bytes());
    }

    output.transfer = ColorTransfer::Srgb;

    println!(
        "Let op gamut is : {:?} transfer = {:?}",
        output.gamut, output.transfer
    );

    output
}
