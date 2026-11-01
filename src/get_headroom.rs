use ultrahdr_core::RawImage;
use zenjpeg::ultrahdr::UhdrPixelFormat;
use crate::color_spaces::rgb_to_xyz_wide16;



pub fn extract_rgb(image: &RawImage) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    assert_eq!(image.format, UhdrPixelFormat::Rgba32F);

    let mut r = Vec::with_capacity(image.data.len() / 16);
    let mut g = Vec::with_capacity(image.data.len() / 16);
    let mut b = Vec::with_capacity(image.data.len() / 16);

    for pixel in image.data.chunks_exact(16) {
        let rf = f32::from_ne_bytes(pixel[0..4].try_into().unwrap());
        let gf = f32::from_ne_bytes(pixel[4..8].try_into().unwrap());
        let bf = f32::from_ne_bytes(pixel[8..12].try_into().unwrap());

        r.push(rf);
        g.push(gf);
        b.push(bf);
    }

    (r, g, b)
}



pub (crate) fn headroom_2bt2020raw(hdr:&RawImage, sdr:&RawImage)->f64
{

    let (rhdr, ghdr, bhdr)   = extract_rgb(hdr);

    let (_x, yhdr, _z) = rgb_to_xyz_wide16(&rhdr, &ghdr, &bhdr);

    let ymax_hdr = yhdr.iter().copied().reduce(f32::max).unwrap();

    let (rsdr, gsdr, bsdr)   = extract_rgb(sdr);

    let (_x, ysdr, _z ) = rgb_to_xyz_wide16(&rsdr, &gsdr, &bsdr);

    let ymax_sdr = ysdr.iter().copied().reduce(f32::max).unwrap();

    assert!(ymax_hdr > 0.0 && ymax_sdr > 0.0);
    let headroom :f64 = (ymax_hdr / ymax_sdr).log2().max(2.0) as f64;
    println!("headroom:{:.2}, yhdrmax:{:.2}, ysdrmax:{:.2}", headroom, ymax_hdr, ymax_sdr);
    headroom

}