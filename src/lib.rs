use worker::*;
use resvg::tiny_skia;
// resvg 内部でエクスポートされている usvg を直接使います
use resvg::usvg;
// Error hint suggests this path for the trait
use resvg::usvg_text_layout::TreeTextToPath;

const SVG_TEMPLATE: &str = include_str!("../template.svg");

async fn load_font() -> Result<Vec<u8>> {
    // TODO: Change this URL to your hosted font file (e.g., R2, GitHub Raw, etc.)
    // Example: "https://raw.githubusercontent.com/Nagatani/ogp-image-gen/main/font_sub.ttf"
    let font_url = "https://raw.githubusercontent.com/Nagatani/ogp-image-gen/main/font_sub.ttf";
    let mut response = Fetch::Url(font_url.parse()?).send().await?;
    if response.status_code() != 200 {
        return Err(worker::Error::from(format!("Failed to download font: status {}", response.status_code())));
    }
    response.bytes().await
}

#[event(fetch)]
pub async fn main(req: Request, _env: Env, _ctx: worker::Context) -> Result<Response> {
    let url = req.url()?;
    let query: std::collections::HashMap<String, String> = url.query_pairs().into_owned().collect();

    let title = query.get("title").map(|s| s.as_str()).unwrap_or("こんにちは Rust");
    let author = query.get("author").map(|s| s.as_str()).unwrap_or("@Gemini");

    let svg_data = SVG_TEMPLATE
        .replace("{{TITLE}}", title)
        .replace("{{AUTHOR}}", author);

    let font_data = load_font().await?;
    let mut fontdb = fontdb::Database::new();
    fontdb.load_font_data(font_data);

    let opt = usvg::Options::default();

    // resvg::usvg::Tree を使用
    let mut rtree = usvg::Tree::from_str(&svg_data, &opt)
        .map_err(|e| worker::Error::from(format!("SVG Parse Error: {}", e)))?;

    rtree.convert_text(&fontdb);

    let pixmap_size = rtree.size.to_screen_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
        .ok_or_else(|| worker::Error::from("Failed to create Pixmap"))?;

    pixmap.fill(tiny_skia::Color::WHITE);

    resvg::render(
        &rtree,
        usvg::FitTo::Original,
        tiny_skia::Transform::default(),
        pixmap.as_mut(),
    );

    let png_bytes = pixmap
        .encode_png()
        .map_err(|_| worker::Error::from("PNG Encoding Error"))?;

    let mut headers = Headers::new();
    headers.set("Content-Type", "image/png")?;
    
    Ok(Response::from_bytes(png_bytes)?.with_headers(headers))
}