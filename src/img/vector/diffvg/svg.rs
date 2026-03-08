pub(super) fn to_svg(params: &[f64], w: usize, h: usize, ns: usize, sw: f64) -> String {
    let mut paths = String::new();
    for s in 0..ns {
        let b = s * super::P;
        let (x0, y0) = (params[b], params[b + 1]);
        let (cx1, cy1) = (params[b + 2], params[b + 3]);
        let (cx2, cy2) = (params[b + 4], params[b + 5]);
        let (x3, y3) = (params[b + 6], params[b + 7]);
        let (r, g, bc, alpha) = (params[b + 8], params[b + 9], params[b + 10], params[b + 11]);
        let ir = (r * 255.0) as u8;
        let ig = (g * 255.0) as u8;
        let ib = (bc * 255.0) as u8;
        paths.push_str(&format!(
            "<path d=\"M{:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" \
             fill=\"none\" stroke=\"#{ir:02X}{ig:02X}{ib:02X}\" \
             stroke-width=\"{sw:.1}\" stroke-opacity=\"{alpha:.3}\" \
             stroke-linecap=\"round\"/>\n",
            x0, y0, cx1, cy1, cx2, cy2, x3, y3
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!-- DiffVG-lite: SPSA gradient estimation + Adam optimizer -->\n\
         <!-- Paper: Li et al., ACM SIGGRAPH Asia 2020 -->\n\
         <svg version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\" \
         width=\"{w}\" height=\"{h}\">\n\
         <rect width=\"{w}\" height=\"{h}\" fill=\"white\"/>\n\
         {paths}</svg>"
    )
}
