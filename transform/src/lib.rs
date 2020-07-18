use cairo::{Context, Format, ImageSurface, ImageSurfaceData, Operator};
use image::{DynamicImage, ImageBuffer, ImageOutputFormat};
use poppler::{PopplerDocument, PopplerPage};
use printpdf;
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

pub mod pdf_to_images;
pub mod pdf_to_pdf;

// Pixels are little-endian (b, g, r, a) to match Cairo & Poppler
type LittleEndianRgbPixel<T> = [T; 3];
const WHITE_PIXEL: LittleEndianRgbPixel<i16> = [255, 255, 255];
const RGBA_PIXEL_SIZE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }
}

impl From<Color> for LittleEndianRgbPixel<u8> {
    fn from(color: Color) -> LittleEndianRgbPixel<u8> {
        [color.b, color.g, color.r]
    }
}

const DEFAULT_BACKGROUND_COLOR: Color = Color {
    r: 226,
    g: 97,
    b: 255,
}; // #e261ff, a purple

// TODO: figure out licensing b/c/o gpl and what it applies to

// TODO: overlay text back on so you can copy/paste if you could before
// A decent example is https://docs.fcc.gov/public/attachments/DOC-363399A1.pdf

// TODO: consider adding build stage to cairo-rs that pulls in docs

#[derive(Error, Debug)]
pub enum TransformationError {
    #[error("Error receiving the PDF: {0}")]
    Receiving(String),

    #[error("Error reading the pdf (with Poppler)")]
    Render(#[from] glib::error::Error),

    #[error("Some unknown error while rendering the PDF (with Poppler)")]
    Unknown,

    #[error("Page {0} does not exist")]
    NonexistentPage(usize),

    #[error("Error reading the pixels of a rendered page with Cairo. Cairo status: {0:?}")]
    PixelRead(cairo::Status),

    #[error("Insufficient memory.")]
    InsufficientMemory,

    #[error("Error writing the transformed pages with printpdf")]
    PdfWrite(#[from] printpdf::errors::Error),

    #[error("PDF has zero pages")]
    ZeroPagePdf,

    #[error("Error outputting the transformed page as an image")]
    ImageEncoding(#[from] image::error::ImageError),
}

impl From<cairo::Status> for TransformationError {
    fn from(status: cairo::Status) -> Self {
        TransformationError::PixelRead(status)
    }
}

pub type Result<T> = std::result::Result<T, TransformationError>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct TransformationStateOptions {
    quality: Quality,
    background_color: Color,
    page_range: PageRange,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Quality {
    Extreme,
    High,
    Normal,
    Low,
    ExtremeLow,
}

impl Quality {
    pub fn from_str(src: &str) -> Option<Quality> {
        match src {
            "extreme" => Some(Quality::Extreme),
            "high" => Some(Quality::High),
            "normal" => Some(Quality::Normal),
            "low" => Some(Quality::Low),
            "extremelow" => Some(Quality::ExtremeLow),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PageRange {
    /// Zero indexed, if starting_index is greater than the index of the last page
    /// then no pages are included. If count is greater than the number of
    /// pages available as many pages as possible are included.
    pub starting_index: usize,
    pub count: usize,
}

impl PageRange {
    fn includes(&self, offset: usize, pages_in_doc: usize) -> bool {
        if offset >= self.count {
            return false;
        }

        if offset + self.starting_index >= pages_in_doc {
            return false;
        }

        return true;
    }
}

#[derive(Debug)]
pub struct TransformationState {
    /// The document to transform
    doc: TransformationStateDoc,
    /// How to transform
    options: TransformationStateOptions,
}

#[derive(Debug)]
pub struct TransformationStateDoc {
    original_title: String,
    poppler: PopplerDocument,
    page_count: usize,
    // We get a segfault if we try to read the document without keeping it around
    // TODO: Figure out why the rust bindings for poppler allow us to get a segfault
    // and file an issue / fix it.
    #[allow(dead_code)]
    bytes: Vec<u8>,
}

impl TransformationState {
    pub fn original_title(&self) -> String {
        self.doc.original_title.clone()
    }

    pub fn page_count(&self) -> usize {
        self.doc.page_count
    }

    pub fn includes_offset(&self, offset: usize) -> bool {
        self.options
            .page_range
            .includes(offset, self.doc.page_count)
    }

    pub fn try_new(
        mut in_blob: Vec<u8>,
        selected_page_range: Option<PageRange>,
        quality: Quality,
        background_color: Option<Color>,
    ) -> Result<TransformationState> {
        let poppler = PopplerDocument::new_from_data(&mut in_blob, "")?;
        let page_count = poppler.get_n_pages();
        let original_title = poppler.get_title().unwrap_or("".into());

        if page_count == 0 {
            return Err(TransformationError::ZeroPagePdf);
        }

        let page_range = selected_page_range.unwrap_or_else(|| PageRange {
            starting_index: 0,
            count: page_count,
        });

        let background_color = background_color.unwrap_or(DEFAULT_BACKGROUND_COLOR);

        let options = TransformationStateOptions {
            background_color,
            page_range,
            quality,
        };

        let doc = TransformationStateDoc {
            original_title,
            poppler,
            page_count,
            bytes: in_blob,
        };

        Ok(TransformationState { doc, options })
    }

    pub fn transform_page(&self, offset: usize) -> Result<TransformedPage> {
        let options = &self.options;
        let doc = &self.doc;

        let page_num = options.page_range.starting_index + offset;
        if page_num > doc.page_count - 1 {
            return Err(TransformationError::NonexistentPage(page_num));
        }

        let page = doc
            .poppler
            .get_page(page_num)
            .ok_or(TransformationError::Unknown)?;

        let size = PageSize::from(&page, options.quality);

        let mut page_surface = render_poppler_page(&page, size)?;

        // Panics with runtime borrow error if refs to the page surface exist.
        // We don't make any except for in render_poppler_page, and we drop it at the
        // end of that function
        let mut img_data = page_surface
            .get_data()
            .map_err(|_| TransformationError::Unknown)?;

        transform_page_data(&mut img_data, options.background_color.into());

        let image = page_data_to_pdf_image(&img_data, size)?;

        Ok(TransformedPage { image, size })
    }

    fn to_pdf(self, pages: Vec<TransformedPage>) -> Result<Vec<u8>> {
        use printpdf::{Image, PdfDocument};
        use std::io::BufWriter;

        let doc = PdfDocument::empty(self.doc.original_title);
        for TransformedPage { image, size } in pages {
            // TODO: somewhere the math here is probably slightly wrong, because the
            // pages are slightly too large
            let (page, layer) = doc.add_page(
                size.width_to_mm().into(),
                size.height_to_mm().into(),
                "img_layer",
            );
            let page = doc.get_page(page);
            let layer = page.get_layer(layer);

            let image = Image::from_dynamic_image(&image);
            // NOTE: I'm pretty sure printpdf confuses DPI with PPI. The argument name is dpi but I
            // believe it's interpreted as if it is PPI
            image.add_to_layer(
                layer.clone(),
                None,
                None,
                None,
                None,
                None,
                Some(size.ppi.as_f64()),
            );
        }

        let mut blob: Vec<u8> = Vec::new();
        doc.save(&mut BufWriter::new(&mut blob))?;

        Ok(blob)
    }
}

pub struct TransformedPage {
    image: image::DynamicImage,
    pub size: PageSize,
}

impl TransformedPage {
    pub fn to_png(&self) -> Result<Vec<u8>> {
        let mut vec = Vec::new();
        self.image.write_to(&mut vec, ImageOutputFormat::Png)?;
        Ok(vec)
    }
}

fn transform_page_data(
    img_data: &mut ImageSurfaceData,
    background_color: LittleEndianRgbPixel<u8>,
) {
    // NOTE: By default poppler renders in ARgb32
    // 32 means 4 8-bit parts
    // Little endian, so b, g, r, A
    //
    // Adding PIXEL_SIZE to a pixel takes you to the first part of the next pixel
    // 0 1 2 3 4 5 6 7
    // _ _ _ _ _ _ _ _
    // | --->--|
    //
    // A page of a reasonable pdf processed on Quality::High might have in the single digit
    // millions of pixels

    // NOTE: There's a reason I process the entire block of data as pixels, ignoring that some might
    // be padding. In tests I've found no padding (stride == width * PIXEL_SIZE), and skipping based
    // on stride/width increases perf significantly.

    // So long as img_data.len() % PIXEL_SIZE == 0 every chunk will be of size PIXEL_SIZE and
    // transform_pixel won't panic
    img_data
        .chunks_exact_mut(RGBA_PIXEL_SIZE)
        .for_each(|pixel| transform_pixel(pixel, background_color));
}

fn transform_pixel(pixel: &mut [u8], background_color: LittleEndianRgbPixel<u8>) {
    //! Pixel must hold 4 items (b, g, r, a) or may panic
    if is_background(pixel) {
        // NOTE: This is about 10% faster than using iter_mut and zip
        pixel[0] = background_color[0];
        pixel[1] = background_color[1];
        pixel[2] = background_color[2];
    }
}

fn is_background(pixel: &[u8]) -> bool {
    //! Queen-wise distance algorithm
    //! Mahama 2016 <https://doi.org/10.2352/ISSN.2470-1173.2016.20.COLOR-349>
    let dissimilarity = pixel
        .iter()
        .take(RGBA_PIXEL_SIZE - 1)
        .zip(WHITE_PIXEL.iter())
        .map(|(candidate, white)| ((*candidate as i16) - white).abs())
        .max()
        // Won't panic: max() -> None if iterator is empty and pixel is always &mut[u8; 4]
        .unwrap();

    dissimilarity < 90
}

fn render_poppler_page(page: &PopplerPage, size: PageSize) -> Result<ImageSurface> {
    // Directly ported from pdftoimage.c example code
    // See <https://web.archive.org/web/20200421162328/https://www.cairographics.org/cookbook/renderpdf/>
    // Interleaved with original C code (marked --> ), manual memory management in source omitted

    // NOTE: I tried changing antialiasing settings, but didn't notice any difference on my
    // System76 darter pro laptop. Seems OK as-is.

    // --> poppler_page_get_size (page, &width, &height);
    // --> /* For correct rendering of PDF, the PDF is first rendered to a
    // -->  * transparent image (all alpha = 0). */
    // --> surface = cairo_image_surface_create (CAIRO_FORMAT_ARGB32,
    // -->                                   IMAGE_DPI*width/72.0,
    // -->                                   IMAGE_DPI*height/72.0);

    let surface = ImageSurface::create(
        Format::ARgb32,
        size.width_to_px().as_i32(),
        size.height_to_px().as_i32(),
    )?;

    // --> cr = cairo_create (surface);
    let cr = Context::new(&surface);

    // --> cairo_scale (cr, IMAGE_DPI/72.0, IMAGE_DPI/72.0);
    let scale_factor = Pt::new(1.0, size.ppi).to_px().as_f64();
    cr.scale(scale_factor, scale_factor);

    // --> cairo_save (cr);
    cr.save(); // save a state we can restore to

    // --> poppler_page_render (page, cr);
    // The difference between render and render_for_printing is that the second omits annotations
    // <https://web.archive.org/web/20200413032121/https://stackoverflow.com/questions/37890831/when-should-i-use-poppler-page-render-vs-poppler-page-render-for-printing?rq=1>
    page.render(&cr);

    // --> cairo_restore (cr);
    cr.restore();

    // --> /* Then the image is painted on top of a white "page". Instead of
    // -->  * creating a second image, painting it white, then painting the
    // -->  * PDF image over it we can use the CAIRO_OPERATOR_DEST_OVER
    // -->  * operator to achieve the same effect with the one image. */
    // --> cairo_set_operator (cr, CAIRO_OPERATOR_DEST_OVER);
    cr.set_operator(Operator::DestOver);
    // --> cairo_set_source_rgb (cr, 1, 1, 1);
    cr.set_source_rgb(1.0, 1.0, 1.0);
    // --> cairo_paint (cr);
    cr.paint();

    Ok(surface)
}

#[derive(Debug, Clone, Copy)]
struct PPI(f64);

impl PPI {
    fn as_f64(&self) -> f64 {
        self.0
    }
}

impl Serialize for PPI {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}ppi", self.as_f64()))
    }
}

impl From<Quality> for PPI {
    fn from(quality: Quality) -> Self {
        PPI(match quality {
            Quality::Extreme => 400.0,
            Quality::High => 200.0,
            Quality::Normal => 120.0,
            Quality::Low => 72.0,
            Quality::ExtremeLow => 10.0,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Px(f64);

impl Px {
    fn new(pixels: f64) -> Self {
        Px(pixels)
    }

    fn as_f64(&self) -> f64 {
        self.0
    }

    fn as_usize(&self) -> usize {
        self.as_f64().ceil() as usize
    }

    fn as_u32(&self) -> u32 {
        //! Note: fails if usize < u32
        //! See <https://web.archive.org/web/20200421150714/https://stackoverflow.com/questions/47786322/why-is-type-conversion-from-u64-to-usize-allowed-using-as-but-not-from/47786517>
        self.as_usize() as u32
    }

    fn as_i32(&self) -> i32 {
        self.as_usize() as i32
    }
}

impl From<Px> for printpdf::Px {
    fn from(px: Px) -> Self {
        Self(px.as_usize())
    }
}

#[derive(Clone, Copy, Debug)]
struct Pt(f64, PPI);

impl Pt {
    fn new(points: f64, ppi: PPI) -> Self {
        Pt(points, ppi)
    }

    fn as_f64(&self) -> f64 {
        self.0
    }

    fn to_px(&self) -> Px {
        let inches = self.as_f64() * (0.996264 / 72.0);
        Px::new(inches * self.1.as_f64())
    }

    fn to_mm(&self) -> Mm {
        Mm::new(self.as_f64() * 0.352778)
    }
}

impl Serialize for Pt {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:.2}px", self.to_px().as_f64()))
    }
}

impl From<Pt> for printpdf::Pt {
    fn from(pt: Pt) -> Self {
        Self(pt.as_f64())
    }
}

#[derive(Clone, Copy, Debug)]
struct Mm(f64);

impl Mm {
    fn new(millimeters: f64) -> Self {
        Mm(millimeters)
    }

    fn as_f64(&self) -> f64 {
        self.0
    }
}

impl From<Mm> for printpdf::Mm {
    fn from(mm: Mm) -> Self {
        Self(mm.as_f64())
    }
}

#[derive(Serialize, Clone, Copy, Debug)]
pub struct PageSize {
    width: Pt,
    height: Pt,
    ppi: PPI,
}

impl PageSize {
    fn new(width: Pt, height: Pt, ppi: PPI) -> Self {
        Self { width, height, ppi }
    }

    fn from(page: &PopplerPage, quality: Quality) -> Self {
        let (width, height) = page.get_size();
        let ppi = PPI::from(quality);
        Self::new(Pt::new(width, ppi), Pt::new(height, ppi), ppi)
    }

    fn width_to_px(&self) -> Px {
        self.width.to_px()
    }

    fn height_to_px(&self) -> Px {
        self.height.to_px()
    }

    fn width_to_mm(&self) -> Mm {
        self.width.to_mm()
    }

    fn height_to_mm(&self) -> Mm {
        self.height.to_mm()
    }
}

fn page_data_to_pdf_image(
    bgra_data: &ImageSurfaceData,
    size: PageSize,
) -> Result<image::DynamicImage> {
    let bgra_data: Vec<u8> = (**bgra_data).into();
    let image = DynamicImage::ImageBgra8(
        ImageBuffer::from_raw(
            size.width_to_px().as_u32(),
            size.height_to_px().as_u32(),
            bgra_data,
        )
        .ok_or(TransformationError::InsufficientMemory)?,
    );
    let image = DynamicImage::ImageRgb8(image.to_rgb());
    Ok(image)
}

pub fn list_error_sources(error: &dyn std::error::Error) -> Vec<String> {
    match error.source() {
        Some(error) => {
            let mut sources = vec![format!("{:?}", error)];
            sources.extend(list_error_sources(error));
            sources
        }
        None => vec![],
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lists_error_sources() {
        use std::error::Error;
        use std::fmt;

        #[derive(Debug)]
        enum NestableError {
            Root(Box<NestableError>),
            Middle(Box<NestableError>),
            Leaf,
        }

        impl fmt::Display for NestableError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let level = match self {
                    NestableError::Root(_) => "Root",
                    NestableError::Middle(_) => "Middle",
                    NestableError::Leaf => "Leaf",
                };
                write!(f, "{}NestableError is here!", level)
            }
        }

        impl Error for NestableError {
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                match self {
                    NestableError::Root(child) | NestableError::Middle(child) => Some(child),
                    NestableError::Leaf => None,
                }
            }
        }

        let nested_error = NestableError::Root(Box::new(NestableError::Middle(Box::new(
            NestableError::Middle(Box::new(NestableError::Leaf)),
        ))));

        assert_eq!(
            list_error_sources(&nested_error),
            vec!["Middle(Middle(Leaf))", "Middle(Leaf)", "Leaf",]
        );
    }

    fn get_in_blob() -> Vec<u8> {
        include_bytes!("../test_assets/multipage_test.pdf").to_vec()
    }
}
